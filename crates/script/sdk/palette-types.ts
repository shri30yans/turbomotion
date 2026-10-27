// A palette that colours the copies of a repeater, as the engine reads it (crates/comp/src/schema/palette.rs). The layer
// `repeat` and the shape item's `repeat` take the same three keys.

import type { Color, Prop } from "./types";

/** How copies pick from the palette. Copy `i` falls `i + offset` colours along it. */
export interface PaletteCycle {
  /** `"step"` (default): colour `floor(i + offset) mod n`. `"gradient"`: a blend of the two colours either side, in the film's working colour space. `"ping_pong"`: forwards then backwards, the end colours not doubled. */
  mode?: "step" | "gradient" | "ping_pong";
  /** Colours the palette is shifted on. Animatable, negative and fractional allowed. */
  offset?: Prop;
  /** Read the palette from its last colour to its first. */
  reverse?: boolean;
}

/** Which paint of a copy takes the colour: the fill (a text or solid layer's own colour), the stroke, or both. */
export type PalettePaint = "fill" | "stroke" | "both";

/** What a repeater takes to colour its copies. */
export interface PaletteOpts {
  /** The colours, at least one. */
  palette?: Color[];
  cycle?: PaletteCycle;
  /** Default `"fill"`. */
  paint?: PalettePaint;
}

/** One colour of a palette cycle by the layer's place in a row, which is what a layer repeater gives each copy; also usable by hand. */
export interface Swatch extends PaletteOpts {
  palette: Color[];
  /** The layer's place in the row, counting from 0. */
  index: number;
}
