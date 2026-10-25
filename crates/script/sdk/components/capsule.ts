// A pill: two round ends joined by straight sides. Shape items turn about the layer origin, not their own centre, so
// capsules at an angle are built as paths.

import { Layer, shape } from "../layers";
import type { Color, LayerOpts, Vec2 } from "../types";

const f1 = (n: number): string => n.toFixed(1);

/** SVG path of a capsule `len` long and `w` wide, starting at (x, y) and pointing `deg` degrees clockwise from +x. */
export function capsulePath(x: number, y: number, len: number, w: number, deg: number): string {
  const a = (deg * Math.PI) / 180;
  const [ux, uy] = [Math.cos(a), Math.sin(a)];
  const [nx, ny] = [-uy * (w / 2), ux * (w / 2)];
  const [ex, ey] = [x + ux * len, y + uy * len];
  const r = w / 2;
  return `M ${f1(x + nx)} ${f1(y + ny)} L ${f1(ex + nx)} ${f1(ey + ny)} A ${r} ${r} 0 0 0 ${f1(ex - nx)} ${f1(ey - ny)} L ${f1(x - nx)} ${f1(y - ny)} A ${r} ${r} 0 0 0 ${f1(x + nx)} ${f1(y + ny)} Z`;
}

export interface CapsuleOpts extends LayerOpts {
  /** Width (thickness). */
  w: number;
  /** Length and direction from the layer's position, or give `from` and `to` instead. */
  len?: number;
  deg?: number;
  from?: Vec2;
  to?: Vec2;
  fill?: Color | Record<string, unknown>;
  stroke?: { color: Color; width: number; [key: string]: unknown };
}

/** `capsule({ w: 40, len: 200, deg: 30, fill: "#3D6BFF", pos: [400, 300] })`, or `capsule({ from: [100, 100], to: [500, 300], w: 24, fill })`. */
export function capsule(o: CapsuleOpts): Layer {
  const { w, len, deg = 0, from, to, fill, stroke, ...layer } = o;
  if (!(w > 0)) throw new Error(`capsule(): w (thickness) must be above 0, got ${w}`);
  let [length, angle, origin] = [len ?? 0, deg, layer.pos];
  if (from || to) {
    if (!from || !to) throw new Error("capsule(): give both from and to");
    length = Math.hypot(to[0] - from[0], to[1] - from[1]);
    angle = (Math.atan2(to[1] - from[1], to[0] - from[0]) * 180) / Math.PI;
    origin = from;
  } else if (!(length > 0)) throw new Error("capsule(): give len (and deg), or from and to");
  const item: Record<string, unknown> = { d: capsulePath(0, 0, length, w, angle) };
  if (fill !== undefined) item.fill = fill;
  if (stroke !== undefined) item.stroke = stroke;
  return shape([item], { ...layer, pos: origin });
}
