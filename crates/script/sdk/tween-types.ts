import type { Color, Ease, Seconds, Vec2 } from "./types";

/** Properties a tween can animate. `x`/`y`/`z` move `pos`. */
export interface Animatable {
  x?: number;
  y?: number;
  z?: number;
  opacity?: number;
  rotation?: number;
  rot_x?: number;
  rot_y?: number;
  scale?: number | Vec2;
  size?: number;
  intensity?: number;
  zoom?: number;
  color?: Color;
}

export type Stagger = number | { each: number; from?: "start" | "end" | "center" };

export interface TweenControls {
  /** Interrupt only overlapping properties at the new tween start, preserving their original curves. */
  overwrite?: "auto" | false;
  /** One sound per animated layer, at that layer's tween start (stagger included): `"pop"`, `{ cue: "tick", volume: 0.3 }`. */
  sfx?: string | { cue: string; volume?: number };
  duration?: Seconds;
  ease?: Ease;
  delay?: Seconds;
  stagger?: Stagger;
}

export type TweenVars = Animatable & TweenControls;
