// An entrance for a whole component: every visible layer enters with an engine preset ("fade", "slide_up", "pop", ...),
// optionally one after another. It uses the layers' own `enter`, so it never fights the animation a component already has.

import type { Layer } from "../layers";
import type { Part } from "./util";

/** The presets of `comp catalogue` that make sense as an entrance. */
export type AppearPreset = "fade" | "slide_up" | "slide_down" | "slide_left" | "slide_right" | "pop" | "zoom_in" | "zoom_out" | "blur" | "whip_left" | "whip_right" | "spin";

export interface AppearSpec {
  preset?: AppearPreset;
  dur?: number;
  /** Seconds between layers (0: all together). */
  stagger?: number;
  /** Extra seconds before the first layer starts. */
  delay?: number;
  ease?: string;
  /** Travel for slides, px. */
  dist?: number;
  /** Blur for the blur preset, px. */
  blur?: number;
}

/** A preset name, or the full spec. */
export type AppearOpt = AppearPreset | AppearSpec;

/** Make `layers` enter from `at`. Groups (null layers) are skipped: their children enter themselves. Returns when the last one is in. */
export function appear(layers: Layer[] | Part, opt: AppearOpt, at: number): number {
  const spec: AppearSpec = typeof opt === "string" ? { preset: opt } : opt;
  const { preset = "fade", dur = 0.45, stagger = 0, delay = 0, ...more } = spec;
  const list = (Array.isArray(layers) ? layers : layers.layers).filter((l) => l.kind !== "null");
  let end = at;
  list.forEach((layer, i) => {
    const wait = delay + i * stagger;
    if (!layer.props.enter) layer.set({ enter: { preset, dur, delay: at + wait, ...more } });
    end = Math.max(end, at + wait + dur);
  });
  return end;
}

/** What a component does last: add its entrance (when asked for) and report the later `end`. */
export function entrance<T extends Part>(part: T, opt: AppearOpt | undefined, at: number): T {
  return opt ? { ...part, end: Math.max(part.end, appear(part, opt, at)) } : part;
}
