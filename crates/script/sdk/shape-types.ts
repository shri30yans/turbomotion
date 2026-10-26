// Shape layers: the items a `shape([...])` takes, as the engine reads them (crates/comp/src/schema/shape.rs and path.rs).
// An item is one geometry plus how it is painted; `trim`, `offset`, `rotation`, `scale` and `opacity` can be tweened.

import type { Color, Prop, Vec2 } from "./types";

/** Exactly one of `d`, `points`, `rect`, `ellipse`, `polygon`, `star` names the outline; the rest refine it. */
export interface PathGeometry {
  /** An SVG path: `"M 0 0 L 100 0 L 100 100 Z"`. */
  d?: string;
  /** Straight lines through these points, or a smooth curve with `smooth`. */
  points?: Vec2[];
  closed?: boolean;
  smooth?: boolean;
  /** `[x, y, w, h]`; `radius` rounds the corners. */
  rect?: [number, number, number, number];
  radius?: number;
  /** `[cx, cy, rx, ry]`. */
  ellipse?: [number, number, number, number];
  /** Regular polygon: `[cx, cy, radius, sides]`. */
  polygon?: [number, number, number, number];
  /** `[cx, cy, outer, inner, points]`. */
  star?: [number, number, number, number, number];
  /** Degrees; polygons and stars. */
  rotate?: number;
  /** Morph toward another SVG path of the same structure by `morph` (0..1). */
  morph_to?: string;
  morph?: Prop;
}

export interface GradientFill {
  type?: "linear" | "radial";
  /** `[[0, "#ff8800"], [1, "#2200ff"]]`. */
  stops: Array<[number, Color]>;
  from?: Vec2;
  to?: Vec2;
  center?: Vec2;
  radius?: number;
}

/** A bare colour, `"metal"`, or one of `color`, `gradient`, `metal` with an `opacity` and a winding `rule`. */
export type ItemFill = Color | "metal" | { color?: Color; gradient?: GradientFill; metal?: boolean | { highlight?: Color; mid?: Color; shadow?: Color }; opacity?: Prop; rule?: "nonzero" | "evenodd" };

export interface ItemStroke {
  color: Color;
  width: Prop;
  cap?: "butt" | "round" | "square";
  join?: "miter" | "round" | "bevel";
  /** `[on, off, ...]` in pixels. */
  dash?: number[];
  dash_offset?: Prop;
  opacity?: Prop;
}

/** Draw only the part of the outline between `start` and `end` (0..1), shifted by `offset`: animate `end` to draw on. */
export interface ItemTrim {
  start?: Prop;
  end?: Prop;
  offset?: Prop;
}

export interface ItemRepeat {
  copies: number;
  /** Added to every copy's position, cumulatively. */
  offset?: Vec2;
  /** Degrees added per copy. */
  rotation?: number;
  /** Multiplied per copy. */
  scale?: number;
  /** Opacity multiplied per copy. */
  opacity?: number;
}

export interface ShapeItem extends PathGeometry {
  fill?: ItemFill;
  stroke?: ItemStroke;
  trim?: ItemTrim;
  offset?: Vec2 | Array<Prop>;
  /** The point `rotation` and `scale` turn about, in layer space (the layer's origin when left out). */
  anchor?: Vec2;
  rotation?: Prop;
  scale?: Prop | [Prop, Prop];
  opacity?: Prop;
  repeat?: ItemRepeat;
}
