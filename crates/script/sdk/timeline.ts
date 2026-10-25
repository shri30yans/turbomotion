// A GSAP-style timeline. `tl.to(layer, { rotation: [null, 90, null], duration: 0.8 }, 0.2)` records tweens on the layer;
// when the layer is written out, all tweens on one property merge into one key list in time order.

import { overwritten } from "./overwrite";
import { flattenItems, isItemKey, itemBase, setItem } from "./item-tweens";
import { relative } from "./relative";
import type { Val } from "./relative";
import { callSite } from "./site";
import { flattenVector, isVectorKey, parseVectorKey, setVector, VECTORS, vectorBase } from "./vector-tweens";
import { Layer, describe, list } from "./layers";
import type { Animatable, Ease, Hit, Seconds, Stagger, TweenVars } from "./types";

export interface Tween {
  prop: string;
  at: Seconds;
  dur: Seconds;
  from?: Val;
  to?: Val;
  ease: Ease;
  /** Where in the author's script this tween was written (for errors found later, when the layer is written out). */
  site: string;
  overwrite?: "auto" | false;
}

const PROPS = ["x", "y", "z", "opacity", "scale", "size", "intensity", "zoom", "color"];
const VECTOR_PROPS = ["rotation", "orientation", "aim"];
const CONTROLS = ["duration", "ease", "delay", "stagger", "sfx", "overwrite"];
const DEFAULTS: Record<string, number> = { opacity: 1, scale: 1, z: 0 };
const EPS = 1e-4;
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

  /** Jump to these values at a moment. */
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
        const tw: Tween = { prop, at, dur: Math.max(dur, EPS), ease: dur === 0 ? "none" : ease, site, overwrite: vars.overwrite };
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

const same = (a: Val | undefined, b: Val | undefined): boolean => JSON.stringify(a) === JSON.stringify(b);

/** Write the tweens of one layer into `out` (the layer's JSON): one merged property per animated name. */
export function compileTweens(out: Record<string, any>, tweens: Tween[], id: string): void {
  if (!tweens.length) return;
  const byProp = new Map<string, Tween[]>();
  for (const t of tweens) byProp.set(t.prop, [...(byProp.get(t.prop) ?? []), t]);
  const axes: Record<string, any> = {};
  const vectors = new Map<string, Map<number, unknown>>();
  for (const [prop, list] of byProp) {
    const sorted = list.map((t, i) => ({ t, i })).sort((a, b) => a.t.at - b.t.at || a.i - b.i).map((x) => x.t);
    const value = mergeProp(sorted, baseOf(out, prop, id), id, prop);
    if (prop === "x" || prop === "y" || prop === "z") axes[prop] = value;
    else if (isItemKey(prop)) setItem(out, prop, value);
    else if (isVectorKey(prop)) {
      const { name, index } = parseVectorKey(prop);
      const parts = vectors.get(name) ?? new Map<number, unknown>();
      parts.set(index, value);
      vectors.set(name, parts);
    } else out[prop] = value;
  }
  for (const [name, parts] of vectors) setVector(out, name, parts);
  if (Object.keys(axes).length) {
    const pos = out.pos;
    if (pos !== undefined && !Array.isArray(pos)) throw new Error(`layer "${id}": cannot animate x/y/z when pos is not a list of numbers`);
    const comps: any[] = pos ? [...pos] : [];
    if (comps.length < 3 && axes.z !== undefined) while (comps.length < 3) comps.push(0);
    ["x", "y", "z"].forEach((a, i) => {
      if (axes[a] !== undefined) comps[i] = axes[a];
    });
    out.pos = comps;
  }
}

function baseOf(out: Record<string, any>, prop: string, id: string): Val | undefined {
  if (isItemKey(prop)) return itemBase(out, prop, id);
  if (prop === "x" || prop === "y" || prop === "z") {
    const v = Array.isArray(out.pos) ? out.pos[{ x: 0, y: 1, z: 2 }[prop]!] : undefined;
    if (typeof v === "number") return v;
    if (prop === "z") return 0;
    return undefined;
  }
  if (isVectorKey(prop)) {
    const { name, index } = parseVectorKey(prop);
    return vectorBase(out, name, index, id);
  }
  const v = out[prop];
  if (v === undefined) return DEFAULTS[prop];
  if (typeof v === "number" || typeof v === "string" || (Array.isArray(v) && v.every((n) => typeof n === "number"))) return v;
  throw new Error(`layer "${id}": "${prop}" is already animated or an expression; tween it from the timeline only`);
}

function mergeProp(tweens: Tween[], base: Val | undefined, id: string, prop: string): unknown {
  if (tweens.some((tween) => tween.overwrite === "auto")) return overwritten(tweens, base, id, prop);
  const keys: Array<[number, Val, string?]> = [];
  let cur = base;
  let simple = true;
  for (const tw of tweens) {
    const where = `layer "${id}": "${prop}"`;
    const from = tw.from === undefined ? cur : relative(tw.from, cur, where);
    const end = tw.to === undefined ? base : tw.to;
    const to = end === undefined ? end : relative(end, from, where);
    if (from === undefined || to === undefined) {
      throw new Error(`layer "${id}": cannot tween "${prop}" without a start and an end value; give the layer a starting "${prop}" (or use fromTo)`);
    }
    if (keys.length) {
      const [lastT, lastV] = keys[keys.length - 1];
      if (tw.at < lastT - 1e-9) {
        throw new Error(`layer "${id}": two tweens of "${prop}" overlap (one starts at ${tw.at}s, before the previous one ends at ${lastT}s); the later tween was written at ${tw.site}`);
      }
      if (tw.at > lastT + EPS) keys.push([tw.at, lastV]);
      if (!same(lastV, from)) {
        keys.push([r6(Math.max(tw.at, lastT) + EPS), from]);
        simple = false;
      }
      simple = false;
    } else {
      keys.push([tw.at, from]);
    }
    keys.push([r6(tw.at + tw.dur), to, tw.ease]);
    cur = to;
  }
  if (simple && tweens.length === 1) {
    const tw = tweens[0];
    return { from: keys[0][1], to: keys[1][1], at: tw.at, dur: r6(tw.dur), ease: tw.ease };
  }
  return { keys };
}
