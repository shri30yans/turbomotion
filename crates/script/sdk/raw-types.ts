// The two builders that take the engine's own vocabulary: `texture` (a generator and its parameters) and `raw` (any layer type).

import type { Color, LayerOpts, Vec2 } from "./types";

/** One generator parameter, as `turbomotion comp fx list` shows it: a number, a colour or a name (a choice), a point, or an animated property. */
export type TextureParam = number | string | Color | Vec2 | { [key: string]: unknown };

/** What a `texture` layer takes besides the layer options: the generator's `params` and the size it paints. */
export interface TextureOpts extends LayerOpts {
  /** The generator's parameters by name (`{ scale: 0.01, seed: 7 }`); unknown names are errors that list the valid ones. */
  params?: Record<string, TextureParam>;
  /** Pixels painted: the composition size when left out. */
  size?: Vec2;
}

/** Any layer the engine knows, as `turbomotion comp catalogue layer <type>` prints it: `type` plus that type's keys. */
export interface RawSpec {
  type: string;
  [key: string]: unknown;
}
