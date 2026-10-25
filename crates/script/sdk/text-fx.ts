// Text effects that need more than a few options: type on a path, wrapped on a surface, scrolling rows, flipping
// letters, cipher decode. Each returns plain engine JSON, typed in ./types.

import { Layer, text } from "./layers";
import type { CipherCharset, Color, MarqueeOpts, Seconds, TextAnimator, TextOpts, TextPathOpts, TextSurfaceOpts } from "./types";

/**
 * Text along any path (After Effects' Path Options): `textOnPath("HELLO", { ellipse: [0, 0, 300, 300] }, { pos: [960, 540] })`.
 * The path is in pixels from the layer's `pos`. Characters advance by arc length and turn to the tangent; `align` picks which
 * end of the text sits at `first_margin` (left), the middle, or `last_margin` (right). Characters pushed past an open path's ends hide.
 */
export const textOnPath = (content: string, path: TextPathOpts, o: TextOpts = {}): Layer => text(content, { ...o, path });

/**
 * Text circling a point: one exact lap every `lap` seconds, forever (the lap is `travel` 0 to 1 on a closed path, so the loop has no seam).
 * `orbitText("SPIN ", 300, 6, { pos: [960, 540], size: 60 })`. A negative `lap` goes the other way round.
 */
export function orbitText(content: string, radius: number, lap: Seconds, o: TextOpts = {}): Layer {
  return textOnPath(content, { ellipse: [0, 0, radius, radius], travel: `time / ${lap}` }, o);
}

/**
 * Text wrapped on a cylinder or sphere (the CC Cylinder / CC Sphere look, with live type): `wrapText("WORLD TOUR", "cylinder", { pos: [960, 540] })`.
 * Each character stands on the surface and turns with it; `turn` (degrees, animatable) scrolls the text round, characters facing away hide
 * unless `back` is set. The surface's axis goes through `pos` and the vanishing point is `pos`.
 */
export const wrapText = (content: string, kind: "cylinder" | "sphere", o: TextOpts & { radius?: TextSurfaceOpts["radius"]; turn?: TextSurfaceOpts["turn"]; back?: boolean } = {}): Layer => {
  const { radius, turn, back, ...layer } = o;
  return text(content, { ...layer, surface: { kind, radius, turn, back } });
};

/**
 * A seamlessly repeating, scrolling row (ticker, marquee). The text and `gap` form one period; copies are laid end to end across the frame
 * and shifted by `speed` px/s. Left aligned from `pos`, so put the layer at x 0 for a full-width row.
 */
export const marquee = (content: string, m: MarqueeOpts, o: TextOpts = {}): Layer => text(content, { ...o, marquee: m });

/** Letters flip in one by one (After Effects' per-character 3D): turn about `axis` from `from` degrees to flat, `each` seconds apart. */
export function charFlip(o: { axis?: "x" | "y"; from?: number; each?: Seconds; at?: Seconds; dur?: Seconds; z?: number; by?: "chars" | "words" } = {}): TextAnimator {
  const { axis = "y", from = 90, each = 0.04, at = 0, dur = 0.5, z, by = "chars" } = o;
  return { by, at, dur, ease: "back.out(1.2)", stagger: { each }, props: { [`rotation_${axis}`]: from, opacity: -1, ...(z === undefined ? {} : { z }) } };
}

/**
 * Cipher decode: characters show random noise, then lock on to the real ones left to right, `each` seconds apart.
 * Deterministic by `seed`. `charset` is a name (`auto`, `upper`, `lower`, `digits`, `alnum`, `hex`, `binary`, `symbols`, `matrix`) or the characters themselves.
 */
export function decode(o: { charset?: CipherCharset; each?: Seconds; at?: Seconds; dur?: Seconds; rate?: number; seed?: number } = {}): TextAnimator {
  const { charset, each = 0.06, at = 0, dur = 0.01, rate, seed } = o;
  return { by: "chars", at, dur, ease: "none", stagger: { each }, cipher: { charset, rate, seed } };
}

/** Outline-only type that writes itself on: stroked glyph outlines from 0 to 1 of their length over `dur` seconds. */
export const outlineText = (content: string, o: TextOpts & { stroke_color?: Color; width?: number; at?: Seconds; dur?: Seconds } = {}): Layer => {
  const { stroke_color = "#ffffff", width = 3, at = 0, dur = 1.2, ...layer } = o;
  return text(content, { ...layer, outline_only: true, stroke: { color: stroke_color, width }, stroke_trim: [0, { from: 0, to: 1, at, dur, ease: "power2.inOut" }] });
};
