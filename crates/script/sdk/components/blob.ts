// An organic blob: a smooth closed outline with seeded wobble, optionally morphing into a second blob (and back).

import { shape } from "../layers";
import type { Layer } from "../layers";
import { rand } from "../motion";
import type { Timeline } from "../timeline";
import type { Color, LayerOpts } from "../types";
import type { Part } from "./util";

const r2 = (n: number): number => Math.round(n * 100) / 100;

/** A smooth closed path of `points` lobes around radius `r`; the same `points` always give a path the engine can morph between. */
export function blobPath(points: number, r: number, wobble: number, seed: number): string {
  const rnd = rand(seed);
  const radii = Array.from({ length: points }, () => r * (1 + wobble * (rnd() * 2 - 1)));
  const at = (i: number): [number, number] => {
    const a = (i / points) * Math.PI * 2;
    const k = radii[((i % points) + points) % points];
    return [Math.cos(a) * k, Math.sin(a) * k];
  };
  // Catmull-Rom through the points, as cubic Beziers
  const segments = Array.from({ length: points }, (_, i) => {
    const [p0, p1, p2, p3] = [at(i - 1), at(i), at(i + 1), at(i + 2)];
    const c1 = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
    const c2 = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
    return `C ${r2(c1[0])} ${r2(c1[1])} ${r2(c2[0])} ${r2(c2[1])} ${r2(p2[0])} ${r2(p2[1])}`;
  });
  const [sx, sy] = at(0);
  return `M ${r2(sx)} ${r2(sy)} ${segments.join(" ")} Z`;
}

export interface BlobOpts extends LayerOpts {
  id: string;
  /** Diameter in px (300). */
  size?: number;
  /** Lobes around the outline (7). */
  points?: number;
  /** How far lobes stray from a circle, 0 to 1 (0.22). */
  wobble?: number;
  seed?: number;
  fill?: Color | Record<string, unknown>;
  stroke?: { color: Color; width: number; [key: string]: unknown };
  /** Morph into a second blob: needs `tl`. `back` returns to the first afterwards. */
  morph?: { tl: Timeline; seed: number; at: number; dur: number; ease?: string; back?: boolean };
}

export interface BlobPart extends Part {
  shape: Layer;
}

/** `blob({ id: "b", size: 320, seed: 4, fill: "#3D6BFF", pos: [960, 540], morph: { tl, seed: 9, at: 1, dur: 1.2, back: true } })`. */
export function blob(o: BlobOpts): BlobPart {
  const { size = 300, points = 7, wobble = 0.22, seed = 1, fill, stroke, morph, ...layer } = o;
  if (points < 3) throw new Error(`blob("${o.id}"): points must be at least 3, got ${points}`);
  const item: Record<string, unknown> = { d: blobPath(points, size / 2, wobble, seed) };
  if (fill !== undefined) item.fill = fill;
  if (stroke !== undefined) item.stroke = stroke;
  if (morph) item.morph_to = blobPath(points, size / 2, wobble, morph.seed);
  const made = shape([item], layer);
  let end = 0;
  if (morph) {
    const ease = morph.ease ?? "sine.inOut";
    morph.tl.to(made, { items: { 0: { morph: 1 } }, duration: morph.dur, ease }, morph.at);
    end = morph.at + morph.dur;
    if (morph.back) {
      morph.tl.to(made, { items: { 0: { morph: 0 } }, duration: morph.dur, ease }, end);
      end += morph.dur;
    }
  }
  return { layers: [made], shape: made, end };
}
