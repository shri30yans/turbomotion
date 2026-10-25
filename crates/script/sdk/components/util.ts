// Shared by the components: the default look (the launch film's tokens), the mono glyph width and a few helpers.

import type { Layer } from "../layers";

/** What every component returns: its layers (drop it straight into `shot({ layers })`), when it finishes, plus named parts. */
export interface Part {
  layers: Layer[];
  /** Seconds (the timeline's clock) at which the component has finished its motion. */
  end: number;
  [name: string]: unknown;
}

export interface Theme {
  ink: string;
  paper: string;
  turbo: string;
  volt: string;
  cobalt: string;
  steel: string;
  line: string;
  mono: string;
  monoBold: string;
  text: string;
  display: string;
  accent: string;
}

/** The defaults match `launch/design.ts`; pass `theme` to a component to restyle it. */
export const THEME: Theme = {
  ink: "#0B0C10", paper: "#F3F0E8", turbo: "#FF5A1F", volt: "#C8F031", cobalt: "#3D6BFF", steel: "#8A93A6", line: "#1D2029",
  mono: "JetBrains Mono", monoBold: "JetBrains Mono Bold", text: "Avenir Next", display: "Avenir Next Condensed", accent: "Instrument Serif",
};

/** JetBrains Mono (and most monospace faces) advance 0.6 em per character. */
export const MONO_ADVANCE = 0.6;

/** Width in pixels of `n` mono characters at `size`. */
export const monoWidth = (n: number, size: number): number => n * size * MONO_ADVANCE;

export const r3 = (n: number): number => Math.round(n * 1000) / 1000;

/** `#rrggbb` plus an alpha 0..1 as `#rrggbbaa`. */
export function alpha(hex: string, a: number): string {
  const m = /^#([0-9a-f]{6})/i.exec(hex);
  if (!m) return hex;
  return `#${m[1]}${Math.round(Math.min(1, Math.max(0, a)) * 255).toString(16).padStart(2, "0")}`;
}

/** A drop shadow effect. */
export const shadow = (blur = 40, dy = 24, opacity = 0.2, color: [number, number, number] = [11, 12, 16]): Record<string, unknown> =>
  ({ effect: "shadow", blur, dx: 0, dy, opacity, color });
