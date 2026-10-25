// A camera lens seen from the front: barrel, glass with a reflection, and an iris of blades that can open.

import { group, shape } from "../layers";
import type { Layer } from "../layers";
import type { Timeline } from "../timeline";
import type { Color, Vec2 } from "../types";
import { Part, THEME, Theme } from "./util";

export interface LensOpts {
  id: string;
  pos: Vec2;
  /** Outer diameter (300). */
  size?: number;
  /** Iris blades (6). */
  blades?: number;
  /** Barrel rim, glass tint and flare colour. */
  rim?: Color;
  glass?: Color;
  accent?: Color;
  /** Open the iris from nearly shut: needs `tl`. */
  open?: { tl: Timeline; at: number; dur?: number; ease?: string; sfx?: string };
  theme?: Partial<Theme>;
}

export interface LensPart extends Part {
  root: Layer;
  iris: Layer;
}

/** `lens({ id: "lens", pos: [960, 540], size: 360, open: { tl, at: 0.5, sfx: "click" } })`. */
export function lens(o: LensOpts): LensPart {
  const th = { ...THEME, ...o.theme };
  const { id, pos, size = 300, blades = 6, rim = th.steel, glass = th.cobalt, accent = th.turbo } = o;
  const R = size / 2;
  const root = group({ id, pos });
  const part = (name: string, items: Array<Record<string, unknown>>, more: Record<string, unknown> = {}): Layer => shape(items, { id: `${id}-${name}`, parent: root, ...more });
  const barrel = part("barrel", [
    { ellipse: [0, 0, R, R], fill: { gradient: { type: "radial", center: [0, 0], radius: R, stops: [[0.6, th.line], [0.92, th.ink], [1, "#2a2d38"]] } }, stroke: { color: rim, width: Math.max(2, R * 0.04) } },
    { ellipse: [0, 0, R * 0.8, R * 0.8], stroke: { color: rim, width: 1.5, dash: [R * 0.05, R * 0.04] }, opacity: 0.6 },
  ]);
  const glassLayer = part("glass", [
    { ellipse: [0, 0, R * 0.62, R * 0.62], fill: { gradient: { type: "radial", center: [-R * 0.15, -R * 0.2], radius: R * 0.8, stops: [[0, glass], [0.7, th.ink], [1, "#05060a"]] } }, stroke: { color: glass, width: 2 } },
  ]);
  const blade = (i: number): Record<string, unknown> => {
    const [a, b] = [-Math.PI / 2 + (i / blades) * Math.PI * 2, -Math.PI / 2 + ((i + 0.55) / blades) * Math.PI * 2];
    return { d: `M ${R * 0.42 * Math.cos(a)} ${R * 0.42 * Math.sin(a)} L ${R * 0.16 * Math.cos(b)} ${R * 0.16 * Math.sin(b)}`, stroke: { color: rim, width: 1.5, opacity: 0.8 } };
  };
  const iris = part("iris", [
    { polygon: [0, 0, R * 0.42, blades], rotate: -90, fill: { gradient: { type: "radial", center: [0, 0], radius: R * 0.42, stops: [[0.3, "#3a3f50"], [1, "#1d2029"]] } }, stroke: { color: rim, width: 2 } },
    ...Array.from({ length: blades }, (_, i) => blade(i)),
    { ellipse: [0, 0, R * 0.14, R * 0.14], fill: "#000000", stroke: { color: rim, width: 1.5 } },
  ]);
  const shine = part("shine", [
    { ellipse: [-R * 0.22, -R * 0.26, R * 0.17, R * 0.07], rotation: -35, fill: { color: "#ffffff", opacity: 0.55 } },
    { ellipse: [R * 0.24, R * 0.28, R * 0.06, R * 0.06], fill: { color: accent, opacity: 0.8 } },
  ], { fx: [{ effect: "blur", radius: 1.5 }] });
  let end = 0;
  if (o.open) {
    const { tl, at, dur = 0.6, ease = "back.out(1.6)", sfx } = o.open;
    tl.from(iris, { scale: 0.2, duration: dur, ease }, at);
    tl.from(root, { scale: 0.92, duration: dur, ease: "expo.out" }, at);
    if (sfx) tl.hit(at, sfx);
    end = at + dur;
  }
  return { layers: [root, barrel, glassLayer, iris, shine], root, iris, end };
}
