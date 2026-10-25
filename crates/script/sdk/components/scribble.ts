// A hand-drawn strike-through: a jittered zigzag path drawn on with `trim`, with a `glitch` on the first stroke.

// imported from the package root (not "../layers") so the file also works as a plain relative import before it is registered in loader.rs
import { Layer, shape, rand } from "turbomotion";
import type { Timeline } from "turbomotion";
import { Part, THEME, Theme, r3 } from "./util";

export interface ScribbleOpts {
  tl: Timeline;
  /** Seconds at which the stroke starts to draw. */
  at: number;
  dur?: number;
  /** Centre of the scribble and the box it covers. */
  pos?: [number, number];
  w?: number;
  h?: number;
  /** Number of back-and-forth strokes. */
  passes?: number;
  width?: number;
  color?: string;
  seed?: number;
  /** Sound at the start; `false` for none. */
  sfx?: string | false;
  ease?: string;
  theme?: Partial<Theme>;
  id?: string;
  parent?: Layer | string;
}

/** `scribble({ tl, at: 1.2, pos: [960, 560], w: 1300, h: 760 })`: `shape` is the layer; the path overshoots the box slightly. */
export function scribble(o: ScribbleOpts): Part {
  const th = { ...THEME, ...o.theme };
  const { tl, at, dur = 0.45, pos = [960, 540], w = 1200, h = 700, passes = 3, width = 26, seed = 7, sfx = "glitch", ease = "power2.inOut" } = o;
  const jit = rand(seed);
  const n = passes * 2 + 1;
  const pts: string[] = [];
  for (let i = 0; i < n; i++) {
    const side = i % 2 === 0 ? -1 : 1;
    const x = side * (w / 2 + jit.range(-70, 50));
    const y = -h / 2 + (h * i) / (n - 1) + jit.range(-h * 0.07, h * 0.07);
    pts.push(`${i === 0 ? "M" : "L"} ${r3(x)} ${r3(y)}`);
  }
  const layer = shape([{ d: pts.join(" "), stroke: { color: o.color ?? th.turbo, width, cap: "round", join: "round" }, trim: { end: { from: 0, to: 1, at, dur, ease } } }],
    { id: o.id ?? "scribble", pos, parent: o.parent as any });
  if (sfx) tl.hit(at, sfx);
  return { layers: [layer], end: at + dur, shape: layer };
}
