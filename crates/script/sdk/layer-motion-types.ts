// Layer-level engine objects that are not keyframes: position wrapping, stagger, flip, motion along a path, floor reflection
// (crates/comp/src/schema/mod.rs). Everything here is on `LayerOpts`.

import type { PathGeometry } from "./shape-types";
import type { Ease, Prop, Seconds } from "./types";

/** After the position is evaluated, wrap `x` and/or `y` into `[min, max)`: a ticker is `pos: ["time * -80", 900]` with `wrap: { x: [-1920, 1920] }`. */
export interface Wrap {
  x?: [number, number];
  y?: [number, number];
}

/** Direct children start `each` seconds apart. */
export interface LayerStagger {
  each?: Seconds;
  from?: "start" | "end" | "center" | "edges" | "random";
  seed?: number;
}

/** This layer flies from `from`'s transform onto its own, starting at `at`. */
export interface Flip {
  from: string;
  at?: Seconds;
  dur?: Seconds;
  ease?: Ease;
}

/** The layer's position follows an outline as `t` goes 0 to 1. */
export interface Along {
  path: PathGeometry;
  t: Prop;
  /** Turn the layer to face the direction of travel. */
  orient?: boolean;
}

/** A faded, softened mirror of the layer on the floor beneath it. */
export interface Reflect {
  /** Opacity at the floor line (0.4). */
  strength?: Prop;
  /** How much of the layer's height is mirrored, 0..1 (0.55). */
  fade?: number;
  /** Pixels between the layer and its mirror (8). */
  gap?: number;
  /** Softness of the mirror in pixels (3). */
  blur?: number;
}
