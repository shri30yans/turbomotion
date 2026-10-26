// Turns the tweens a timeline recorded on a layer into the engine's property forms. Timeline positions are in the clock the
// layer lives in (shot time, or comp time); the engine reads a layer's keys from the layer's own `start`, so every time
// written out is shifted by that start here, and nowhere else.

import { matchAxes } from "./dimension";
import { overwritten } from "./overwrite";
import { isItemKey, itemBase, setItem } from "./item-tweens";
import { relative } from "./relative";
import type { Val } from "./relative";
import { applyHandles, takePath } from "./motion-path";
import { isVectorKey, parseVectorKey, setVector, vectorBase } from "./vector-tweens";
import type { Tween } from "./timeline";

const DEFAULTS: Record<string, number> = { opacity: 1, scale: 1, z: 0, skew: 0 };
const TIE = 1e-9;
const r6 = (n: number): number => Math.round(n * 1e6) / 1e6;
const same = (a: Val | undefined, b: Val | undefined): boolean => JSON.stringify(a) === JSON.stringify(b);

/** Converts a time on the timeline's clock to the layer's own time. */
export type LayerTime = (t: number) => number;

/** The layer's `start` as a number; tweens cannot be placed against a start the SDK cannot read. */
export function layerTime(out: Record<string, any>, tweens: Tween[], id: string): LayerTime {
  const start = out.start ?? 0;
  if (typeof start !== "number") {
    if (!tweens.length) return (t) => t;
    throw new Error(`layer "${id}": its start is ${JSON.stringify(start)}, which a timeline cannot convert to layer time; give start as a number of seconds, or animate this layer with expressions`);
  }
  return (t) => r6(t - start);
}

/** Write the tweens of one layer into `out` (the layer's JSON): one merged property per animated name. */
export function compileTweens(out: Record<string, any>, all: Tween[], id: string): void {
  if (!all.length) return;
  const local = layerTime(out, all, id);
  const tweens = takePath(out, all, local, id);
  if (!tweens.length) return;
  const byProp = new Map<string, Tween[]>();
  for (const t of tweens) byProp.set(t.prop, [...(byProp.get(t.prop) ?? []), t]);
  const axes: Record<string, any> = {};
  const vectors = new Map<string, Map<number, unknown>>();
  for (const [prop, list] of byProp) {
    const ordered = list.map((t, i) => ({ t, i })).sort((a, b) => a.t.at - b.t.at || a.i - b.i).map((x) => x.t);
    const { tweens: sorted, base } = matchAxes(prop, ordered, baseOf(out, prop, id), id);
    const value = mergeProp(sorted, base, id, prop, local);
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

type Key = [number, Val] | [number, Val, string] | Record<string, any>;
const valueOf = (k: Key): Val => (Array.isArray(k) ? k[1] : k.v);

/** The engine's ease for a jump: the key before it holds until this key's time, then the value changes. */
const HOLD = "hold";

/**
 * One property as keys. A `set`, or a tween that starts from a value other than where the last one ended, is a key with
 * the `hold` ease: the old value stays exactly up to its time and the new one is read from that moment on.
 */
function mergeProp(tweens: Tween[], base: Val | undefined, id: string, prop: string, local: LayerTime): unknown {
  if (tweens.some((tween) => tween.overwrite === "auto")) return overwritten(tweens, base, id, prop, local);
  const keys: Key[] = [];
  let cur = base;
  let lastEnd = -Infinity;
  for (const tw of tweens) {
    const where = `layer "${id}": "${prop}"`;
    const from = tw.from === undefined ? cur : relative(tw.from, cur, where);
    const end = tw.to === undefined ? base : tw.to;
    const to = end === undefined ? end : relative(end, from, where);
    if (from === undefined || to === undefined) {
      throw new Error(`layer "${id}": cannot tween "${prop}" without a start and an end value; give the layer a starting "${prop}" (or use fromTo)`);
    }
    if (tw.at < lastEnd - TIE) {
      throw new Error(`layer "${id}": two tweens of "${prop}" overlap (one starts at ${tw.at}s, before the previous one ends at ${lastEnd}s); the later tween was written at ${tw.site}`);
    }
    const t = local(tw.at);
    const step = tw.dur === 0;
    const last = keys[keys.length - 1];
    if (!last) {
      // a step needs a key before it: at the layer's first moment, unless the step is already over by then
      if (!step) keys.push([t, from]);
      else if (t > 0) keys.push([0, from]);
    } else if (!same(valueOf(last), from)) keys.push([t, from, HOLD]);
    else if (!step && tw.at > lastEnd + TIE) keys.push([t, valueOf(last)]);
    keys.push(step ? [t, to, HOLD] : [local(tw.at + tw.dur), to, tw.ease]);
    if (tw.handles && !step) applyHandles(keys, tw.handles);
    cur = to;
    lastEnd = tw.at + tw.dur;
  }
  if (tweens.length === 1 && tweens[0].dur > 0 && !tweens[0].handles) {
    const tw = tweens[0];
    return { from: valueOf(keys[0]), to: valueOf(keys[keys.length - 1]), at: local(tw.at), dur: r6(tw.dur), ease: tw.ease };
  }
  return { keys };
}
