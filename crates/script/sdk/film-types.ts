import type { Color, Seconds, Ease } from "./types";

export type TransitionName =
  | "cut" | "dissolve" | "blur_dissolve" | "dip_black" | "dip_white" | "dip"
  | "push_left" | "push_right" | "push_up" | "push_down"
  | "slide_over_left" | "slide_over_right" | "slide_over_up" | "slide_over_down"
  | "whip_pan_left" | "whip_pan_right"
  | "pan_out_left" | "pan_out_right" | "pan_out_up" | "pan_out_down"
  | "zoom_through" | "zoom_back" | "cross_zoom" | "window" | "iris" | "flood"
  | "wipe_left" | "wipe_right" | "wipe_up" | "wipe_down" | "glitch";

export interface TransitionSpec {
  type: TransitionName;
  dur?: Seconds;
  ease?: Ease;
  color?: Color;
  /** `window`: where the portal sits when small, `[centre x, centre y, width]`, all fractions of the frame. */
  portal?: [number, number, number];
  /** `window`: the portal's corner radius in pixels. */
  radius?: number;
}

export type MoveName =
  | "push_in" | "pull_out" | "drift_left" | "drift_right" | "drift_up" | "drift_down"
  | "ken_burns" | "handheld" | "punch";

export type CameraMoveName =
  | "orbit" | "dolly_in" | "dolly_out" | "dolly_zoom" | "crane_up" | "crane_down" | "truck_left" | "truck_right";

export interface CameraMove {
  move: CameraMoveName;
  deg?: number;
  amount?: number;
  ease?: Ease;
  dof?: { focus: number; blur: number };
}

export type Layerish = unknown; // a Layer, or an array of them, or a Group (flattened by the SDK)

export interface ShotOpts {
  id: string;
  dur: Seconds;
  layers?: Layerish[];
  in?: TransitionName | TransitionSpec;
  move?: MoveName | { type: MoveName; amount?: number; at?: number[] };
  camera?: CameraMove;
  bg?: Color;
  speed?: number | Array<[number, number] | [number, number, Ease]>;
  start?: Seconds;
  fx?: Array<Record<string, unknown>>;
  /** Sounds on shot-local seconds (`tl.hits()`); `film.timeline` writes them as `"<id>.start+<t>"`. */
  hits?: Hit[];
  extra?: Record<string, unknown>;
}

export interface OverlayOpts {
  shot: string;
  at: Seconds;
  dur: Seconds;
  layer: Layerish;
}

/** A sound on a moment. `at` is film time (a number, or a literal like `"title.start+0.8"`); inside `shot({ hits })` it is shot-local seconds. */
export interface Hit {
  at: string | number;
  /** A sound-kit cue (`pop`, `tick`, `boom`...) or a path to a sound file. */
  cue: string;
  volume?: number;
}

export interface TimelineOpts {
  shots: Array<{ toJSON(): unknown }>;
  overlays?: OverlayOpts[];
  hits?: Hit[];
  transition?: TransitionSpec;
  stage?: Color;
  sfx?: boolean | { volume: number };
  /** True motion blur on every shot, so transitions and moves smear like a camera. */
  mblur?: boolean | { samples?: number; shutter?: number };
}

/** The `color` option of a composition: see `CompOpts.color`. */
export interface ColorOpts {
  /** Numeric depth of every picture; half-float is the only one. */
  depth?: "half";
  /** Film only. The gamut the finished frame is encoded in (with the sRGB curve), which the file is tagged with. Default `"srgb"`. */
  space?: "srgb" | "display_p3";
  /** Blend, blur, resample and add in linear light instead of on display-encoded numbers. Default `false`. */
  linear_light?: boolean;
  /** The gamut pictures are kept in. Default `"srgb"`. */
  working_profile?: "srgb" | "display_p3";
}

export interface CompOpts {
  /** Asset directory relative to the script; defaults to the script's folder. */
  root?: string;
  /** Shader plugin directories relative to the asset root. */
  fx_path?: string[];
  /** Block directories relative to the asset root. */
  block_path?: string[];
  /** Cinematic bars, applied after transitions. */
  letterbox?: [number, number];
  /**
   * Where layers run their effects and masks. `"screen"` (the default) runs them on the picture already placed in the frame, so
   * a 10 px blur is 10 screen pixels at any scale. `"layer"` is After Effects' order: effects and masks run on the layer's own
   * pixels (grown to fit what the effects spread) and the layer's transform comes after, so a blur scales and turns with the
   * layer and content outside the frame still feeds it. Nested compositions take the film's unless they say their own.
   */
  effect_space?: "screen" | "layer";
  /**
   * What the numbers of the pictures mean. Left out, nothing changes: colours, blends and blurs work on display-encoded sRGB
   * numbers. `{ linear_light: true }` keeps pictures as light, so blends, blurs, glows and gradients are computed on light
   * (50% white over black shows as level 188, not 128). Nested compositions take the film's unless they say their own.
   */
  color?: ColorOpts;
  width: number;
  height: number;
  fps: number;
  bg?: Color;
  dur?: Seconds;
  /** Beats per minute, so `beat:8` time literals and beat-synced effects work. */
  bpm?: number;
  palette?: Record<string, string>;
  fonts?: Record<string, string>;
  look?: Array<Record<string, unknown>>;
  extra?: Record<string, unknown>;
}

