import type { Handles, PathPoint } from "./motion-types";
import type { Color, Ease, Seconds } from "./types";

/** A number, or one relative to the property when the tween starts: `"+=40"`, `"-=0.5"`. */
export type Num = number | `+=${number}` | `-=${number}`;

/** What an item of a shape layer lets a tween change (`tl.to(layer, { items: { 0: { trim_end: 1 } } })`). A rect's size and radius are fixed; animate `scale` instead. */
export interface ItemVars {
  trim_start?: Num;
  trim_end?: Num;
  trim_offset?: Num;
  offset?: [Num, Num];
  rotation?: Num;
  scale?: Num | [Num, Num];
  opacity?: Num;
  morph?: Num;
  stroke_width?: Num;
  stroke_opacity?: Num;
  dash_offset?: Num;
  stroke_color?: Color;
  fill_opacity?: Num;
  fill_color?: Color;
}

/** Properties a tween can animate. `x`/`y`/`z` move `pos`; `pos: [x, y, z]` moves several at once. */
export interface Animatable {
  x?: Num;
  y?: Num;
  z?: Num;
  /** `[x, y]` or `[x, y, z]`; a `null` entry leaves that axis alone. */
  pos?: [Num | null, Num | null] | [Num | null, Num | null, Num | null];
  opacity?: Num;
  /** After Effects' X, Y and Z Rotation: one number is the Z Rotation, three turn it about each axis. */
  rotation?: Num | [Num | null, Num | null, Num | null];
  /** After Effects' X, Y and Z Orientation: the layer's own pose. */
  orientation?: [Num | null, Num | null, Num | null];
  /** The point the layer's face turns toward, in the parent's space. */
  aim?: [Num | null, Num | null, Num | null];
  scale?: Num | [Num, Num];
  /** Shear in degrees; positive leans the top to the right. */
  skew?: Num;
  size?: Num;
  intensity?: Num;
  zoom?: Num;
  color?: Color;
  /** Shape items by number: `{ 0: { trim_end: 1, stroke_width: 6 } }`. */
  items?: Record<number, ItemVars>;
}

export type Stagger = number | { each: number; from?: "start" | "end" | "center" };

export interface TweenControls {
  /** Interrupt only overlapping properties at the new tween start, preserving their original curves. */
  overwrite?: "auto" | false;
  /** One sound per animated layer, at that layer's tween start (stagger included): `"pop"`, `{ cue: "tick", volume: 0.3 }`. */
  sfx?: string | { cue: string; volume?: number };
  duration?: Seconds;
  ease?: Ease;
  /** Speed and influence at the start and end of the move (After Effects' keyframe handles), in place of `ease`: `{ out: "easy", in: { speed: 0, influence: 60 } }`. */
  handles?: Handles;
  /**
   * The layer follows a curve through these points over the tween: the first at its start, the last at its end, the
   * points between passed at an even speed (roving keys), the whole move shaped by `ease` or `handles`. A point can be
   * `[x, y]`, or `{ v: [x, y], out: [dx, dy] }` with the handles of the curve there. `autoOrient: true` turns the layer along it.
   */
  path?: PathPoint[] | { points: PathPoint[]; autoOrient?: boolean };
  delay?: Seconds;
  stagger?: Stagger;
}

export type TweenVars = Animatable & TweenControls;
