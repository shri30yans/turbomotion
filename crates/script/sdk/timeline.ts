// A GSAP-style timeline. `tl.to(layer, { rotation: [null, 90, null], duration: 0.8 }, 0.2)` records tweens on the layer;
// when the layer is written out (compile.ts), all tweens on one property merge into one key list in time order.
// Every position on a timeline is the clock of the shot (or comp) the layers live in, whatever a layer's own `start` is.

import { flattenItems, isItemKey } from "./item-tweens";
import type { Val } from "./relative";
import { callSite } from "./site";
import { flattenVector, isVectorKey, VECTORS } from "./vector-tweens";
import { Layer, list } from "./layers";
import type { Animatable, Ease, Hit, Seconds, Stagger, TweenVars } from "./types";

export interface Tween {
  prop: string;
  /** Seconds on the clock of the shot or film the layer lives in (not the layer's own time). */
  at: Seconds;
  dur: Seconds;
  from?: Val;
  to?: Val;
  ease: Ease;
  /** Where in the author's script this tween was written (for errors found later, when the layer is written out). */
  site: string;
  overwrite?: "auto" | false;
}

const PROPS = ["x", "y", "z", "opacity", "scale", "size", "intensity", "zoom", "color", "skew"];
const VECTOR_PROPS = ["rotation", "orientation", "aim"];
const CONTROLS = ["duration", "ease", "delay", "stagger", "sfx", "overwrite"];
const r6 = (n: number): number => Math.round(n * 1e6) / 1e6;

/** `pos: [x, y, z]` becomes x/y/z tweens, `items: { 0: {...} }` one per item property, and a vector one per component. */
function flat(vars: Record<string, any>, where: string): Record<string, Val> {
  const out: Record<string, Val> = {};
  for (const [key, value] of Object.entries(vars)) {
    if (key === "pos") {
      if (!Array.isArray(value) || value.length < 2 || value.length > 3) throw new Error(`${where}: pos is [x, y] or [x, y, z]`);
      if (["x", "y", "z"].some((a) => a in vars)) throw new Error(`${where}: give "pos" or x/y/z, not both`);
      value.forEach((c, i) => { if (c !== null && c !== undefined) out["xyz"[i]] = c; });
    } else if (key === "items") Object.assign(out, flattenItems(value, where));
    else if (key in VECTORS) Object.assign(out, flattenVector(key, value, where));
    else out[key] = value;
  }
  return out;
}

export type Position = number | string;
type Target = Layer | Layer[] | unknown;

/** Tweens placed on a time axis, compiled into each layer's keyframes. */
export class Timeline {
  private labels = new Map<string, number>();
  private prevStart = 0;
  private prevEnd = 0;
  private total = 0;
  private sounds: Hit[] = [];

  constructor(private defaults: { ease?: Ease; duration?: Seconds } = {}) {}

  /** Animate to these values. */
  to(target: Target, vars: TweenVars, position?: Position): this {
    return this.tween(target, "to", undefined, vars, position);
  }

  /** Animate from these values to wherever the layer is. */
  from(target: Target, vars: TweenVars, position?: Position): this {
    return this.tween(target, "from", vars, vars, position);
  }

  fromTo(target: Target, from: Animatable, to: TweenVars, position?: Position): this {
    return this.tween(target, "fromTo", from, to, position);
  }

  /** Jump to these values at a moment: the old value up to it, the new one from it, exactly. */
  set(target: Target, vars: Animatable, position?: Position): this {
    return this.tween(target, "to", undefined, { ...vars, duration: 0 }, position);
  }

  addLabel(name: string, position?: Position): this {
    this.labels.set(name, this.resolve(position));
    return this;
  }

  /** A sound at a moment of this timeline (a position, like `to`): `tl.hit(1.2, "boom")`. */
  hit(position: Position, cue: string, volume?: number): this {
    const at = r6(this.resolve(position));
    this.sounds.push(volume === undefined ? { at, cue } : { at, cue, volume });
    return this;
  }

  /** Every sound this timeline made (`sfx` on tweens, `hit()`), in seconds of this timeline: pass to `shot({ hits })`. */
  hits(): Hit[] {
    return [...this.sounds].sort((a, b) => (a.at as number) - (b.at as number));
  }

  /** Seconds from 0 to the end of the last tween. */
  duration(): Seconds {
    return r6(this.total);
  }

  private resolve(p: Position | undefined): number {
    if (p === undefined) return this.total;
    if (typeof p === "number") return p;
    if (this.labels.has(p)) return this.labels.get(p)!;
    const m = /^(<|>|\+=|-=)(-?\d*\.?\d*)$/.exec(p);
    if (!m) throw new Error(`timeline position ${JSON.stringify(p)}: use a number, "+=0.5", "-=0.2", "<", ">" or a label name`);
    const n = m[2] === "" ? 0 : Number(m[2]);
    switch (m[1]) {
      case "<": return this.prevStart + n;
      case ">": return this.prevEnd + n;
      case "+=": return this.total + n;
      default: return this.total - n;
    }
  }

  private tween(target: Target, mode: "to" | "from" | "fromTo", fromVars: Animatable | undefined, vars: TweenVars, position?: Position): this {
    const layers = list(target, `tl.${mode === "fromTo" ? "fromTo" : mode}() target`);
    if (!layers.length) throw new Error(`tl.${mode}(): no layers to animate (is the array empty, or the variable undefined?)`);
    const where = `tl.${mode}()`;
    const to = flat(vars, where);
    const from = fromVars && fromVars !== vars ? flat(fromVars, where) : to;
    const props = Object.keys(to).filter((k) => !CONTROLS.includes(k));
    for (const k of [...props, ...Object.keys(from).filter((k) => !CONTROLS.includes(k))]) {
      if (!PROPS.includes(k) && !isItemKey(k) && !isVectorKey(k)) throw new Error(`${where}: "${k}" cannot be animated here; animatable: ${[...PROPS, ...VECTOR_PROPS].join(", ")}, pos, items`);
    }
    if (vars.overwrite !== undefined && vars.overwrite !== false && vars.overwrite !== "auto") throw new Error('timeline overwrite must be "auto" or false');
    const dur = vars.duration ?? this.defaults.duration ?? 0.5;
    if (!(dur >= 0)) throw new Error(`tl.${mode}(): duration must be 0 or more, got ${dur}`);
    const ease = vars.ease ?? this.defaults.ease ?? "power1.out";
    const start = this.resolve(position) + (vars.delay ?? 0);
    const offsets = staggerOffsets(layers.length, vars.stagger);
    const site = callSite();
    const sfx = typeof vars.sfx === "string" ? { cue: vars.sfx, volume: undefined } : vars.sfx;
    let end = start;
    layers.forEach((layer, i) => {
      const at = r6(start + offsets[i]);
      if (sfx) this.hit(at, sfx.cue, sfx.volume);
      for (const prop of props) {
        const v = to[prop];
        const tw: Tween = { prop, at, dur, ease: dur === 0 ? "none" : ease, site, overwrite: vars.overwrite };
        if (mode === "to") tw.to = v;
        else if (mode === "from") tw.from = v;
        else {
          tw.to = v;
          tw.from = from[prop];
        }
        layer.tweens.push(tw);
        end = Math.max(end, at + dur);
      }
    });
    this.prevStart = start;
    this.prevEnd = end;
    this.total = Math.max(this.total, end);
    return this;
  }
}

function staggerOffsets(n: number, s: Stagger | undefined): number[] {
  if (s === undefined) return Array(n).fill(0);
  const { each, from = "start" } = typeof s === "number" ? { each: s, from: "start" as const } : s;
  return Array.from({ length: n }, (_, i) => each * (from === "end" ? n - 1 - i : from === "center" ? Math.abs(i - (n - 1) / 2) : i));
}

/** A GSAP-style timeline: `tl.to(layer, { opacity: 1, duration: 0.5 }, at)`, `from`, `set`, with `stagger` and `overwrite`. */
export const timeline = (defaults: { ease?: Ease; duration?: Seconds } = {}): Timeline => new Timeline(defaults);
