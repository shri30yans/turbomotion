// Keyframe handles and motion paths: the types of After Effects' temporal and spatial interpolation, as the engine reads them
// (crates/comp/src/anim/prop). Everything here is also spelled in JSON; `motionPath` and `tl.to(layer, { path })` write it for you.

import type { Seconds, Vec2, Vec3 } from "./types";

/** How a value leaves or arrives at a key, as After Effects' Keyframe Velocity dialog names it. Influence is a percent, 0 to 100. */
export type Handle = { speed: number | number[]; influence: number };

/**
 * One side of a key. `"linear"` keeps the segment's own speed, `"auto"` takes the slope between the neighbouring keys,
 * `"easy"` is Easy Ease (speed 0, influence 33.33 %), `"hold"` (leaving only) keeps the value until the next key.
 */
export type Side = "linear" | "hold" | "auto" | "easy" | Handle;

/** Where a key's motion path bends. `in` points back along the path, `out` forward, both as offsets from the key. */
export interface PathHandles {
  spatial?: "auto" | "linear" | "bezier" | "continuous";
  in?: Vec2 | Vec3;
  out?: Vec2 | Vec3;
}

/** A key of a motion path, at a time. `ease` is the ease of the move INTO it; `ease_in` / `ease_out` replace it with speed and influence. */
export interface PathKey extends PathHandles {
  t: Seconds;
  v: Vec2 | Vec3;
  ease?: string;
  ease_in?: Side;
  ease_out?: Side;
  /** Both sides share one speed (give it on one side; the other side gives only its `influence`). */
  continuous?: boolean;
}

/** A key without a time: its place on the path is kept, and its time is found so the speed stays even between the fixed keys around it. */
export interface RovingKey extends PathHandles {
  roving: true;
  v: Vec2 | Vec3;
}

/** A position that follows a curve: `pos: motionPath([...])`. */
export interface MotionPath {
  keys: Array<PathKey | RovingKey>;
  /** The default handles of keys that give none: `"auto"` is a smooth curve through every key, `"linear"` a polyline. */
  spatial?: "auto" | "linear" | "bezier";
  /** Turn the layer to face along the path. */
  auto_orient?: "path";
  loop?: "cycle" | "pingpong" | "offset";
}

/** A key of a scalar or vector property that carries handles. */
export interface HandleKey<V = number> {
  t: Seconds;
  v: V;
  ease?: string;
  ease_in?: Side;
  ease_out?: Side;
  continuous?: boolean;
}

/** A point of `tl.to(layer, { path })`: a place, or a place with the handles of the curve there. */
export type PathPoint = Vec2 | Vec3 | ({ v: Vec2 | Vec3 } & PathHandles);

/** The two ends of a tween as speed and influence; they take the place of `ease`. */
export interface Handles {
  /** How the value leaves the start of the tween. */
  out?: Side;
  /** How it arrives (not `"hold"`). */
  in?: Exclude<Side, "hold">;
}
