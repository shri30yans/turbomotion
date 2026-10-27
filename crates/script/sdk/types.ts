// Shared types. Everything an author can write is described here, so an editor catches mistakes before the render does.

import type { BlendMode } from "./blend-types";
import type { ItemFill } from "./shape-types";
import type { Along, Flip, LayerStagger, Reflect, Wrap } from "./layer-motion-types";
import type { MotionPath } from "./motion-types";
import type { PaletteOpts, Swatch } from "./palette-types";

export type Ease = string; // any engine ease: "none", "power2.out", "back.out(1.4)", "elastic.out(1, 0.4)", "steps(8)"...
export type Vec2 = [number, number];
export type Vec3 = [number, number, number];
export type Color = string | [number, number, number] | [number, number, number, number];
export type Seconds = number;

/** A property: a constant, an expression string, or any raw form the engine accepts (tween, keys, loop). */
export type Prop<T = number> = T | string | { [key: string]: unknown };

/** How a lit 3D layer answers to light. */
export interface Material {
  ambient?: number;
  diffuse?: number;
  specular?: number;
  shininess?: number;
}

export interface Repeat extends PaletteOpts {
  /** Layers in all, the original included (2 to 256). */
  count: number;
  /** Move per copy: a number or an expression (`"time * 20"`) per axis. */
  offset?: Prop | [Prop, Prop] | [Prop, Prop, Prop];
  /** Degrees per copy (one number, or three for a `three_d` layer). */
  rotation?: Prop | [Prop, Prop, Prop];
  /** Each copy is this fraction of the one before (1). */
  scale?: number;
  /** Each copy is this fraction as opaque as the one before (1). */
  opacity?: number;
  /** The point the steps turn and scale about, in the parent's space ([0, 0]). */
  origin?: [number, number] | [number, number, number];
  /** "below" (default): later copies sit behind the original. */
  order?: "below" | "above";
}

export interface Extrude {
  depth: number;
  color?: Color;
  far?: Color;
}

/** A named entrance or exit of the layer as a whole. Its times are in the layer's own clock (they start at the layer's `start`). */
export interface Transit {
  preset: string;
  dur?: Seconds;
  delay?: Seconds;
  ease?: Ease;
  /** Travel in pixels for slides and whips. */
  dist?: number;
  /** Blur in pixels at the hidden end. */
  blur?: number;
  /** `window` only: where the small portal sits, `[centre x, centre y, width]` as fractions of the frame. */
  portal?: [number, number, number];
  /** `window` only: corner radius in px of the small portal. */
  radius?: number;
}

/** A 3D layer is lit when the composition has lights; `false` keeps it flat. */
export type Lit = boolean;

/** What every layer takes. `x`, `y`, `z` are shorthand for `pos`. */
export interface LayerOpts {
  id?: string;
  /** `[x, y]`, `[x, y, z]`, one property per axis, or a curve: `motionPath([...])`. */
  pos?: Vec2 | Vec3 | Array<Prop> | MotionPath;
  x?: number;
  y?: number;
  z?: number;
  anchor?: Vec2 | Vec3;
  scale?: Prop | [Prop, Prop];
  /** After Effects' X, Y and Z Rotation: the route the layer takes. One number is the Z Rotation, three turn it about each axis. */
  rotation?: Prop | [Prop, Prop, Prop];
  /** After Effects' X, Y and Z Orientation: the layer's own pose, `[x, y, z]`. */
  orientation?: [Prop, Prop, Prop];
  /** The point the layer's face turns toward, in the parent's space. */
  aim?: [Prop, Prop, Prop];
  /** Which side of a 3D layer is hidden. After Effects draws both sides. */
  cull?: "none" | "front" | "back";
  opacity?: Prop;
  /** Shear in degrees; positive leans the top to the right. Tweenable. */
  skew?: Prop;
  three_d?: boolean;
  material?: Material;
  extrude?: Extrude;
  /** Draw `count` copies of this layer, each one step further on (After Effects' Repeater). Needs an `id`. */
  repeat?: Repeat;
  /** Seconds into the composition (or shot) when the layer begins. Timeline positions stay in the composition's clock; the SDK converts them to the layer's own. */
  start?: Seconds;
  dur?: Seconds;
  parent?: string | { id: string };
  /** How the layer combines with what is below (default `"normal"`). In a film with `color.linear_light` it blends light. */
  blend?: BlendMode;
  fx?: Array<Record<string, unknown>>;
  masks?: Array<Record<string, unknown>>;
  matte?: Record<string, unknown>;
  enter?: Transit;
  exit?: Transit;
  hidden?: boolean;
  mblur?: boolean;
  lit?: Lit;
  /** Shear in degrees (GSAP `skewX`); positive leans the top to the right. */
  skew?: Prop;
  wrap?: Wrap;
  stagger?: LayerStagger;
  flip?: Flip;
  /** Keep this layer inside another layer's alpha, and still draw that layer. */
  clip_to?: string;
  /** How `check --layout` treats this layer. */
  layout?: "foreground" | "backdrop" | "ignore";
  along?: Along;
  reflect?: Reflect;
  /** One colour of a palette cycle in place of the layer's own (a text, solid or shape layer). A repeater's `palette` sets it on every copy. */
  swatch?: Swatch;
  /** Anything else the engine takes (see `turbomotion comp catalogue`); merged as written. */
  extra?: Record<string, unknown>;
}

/** A rectangle fitted to the characters showing: a text bar, pill, chip or lower third. Drawn behind the glyphs, in the layer's own pixels. */
export interface TextBox {
  /** What the box is fitted to; `"text"` is the only choice. */
  fit: "text";
  /** `[x, y]` px beyond the characters: sides, then top and bottom. Each is a number or a keyed value; 0 or more. Default `[0, 0]`. */
  padding?: [Prop, Prop];
  /** Corner radius px; held to half the box's shorter side, so a large one makes a pill. Default 0. */
  radius?: number;
  fill: Color | Record<string, unknown>;
  /** The least width of a box, padding included. Default 0. */
  min_width?: number;
  /** Which side of a box widened to `min_width` keeps its place. Default: the text's `align`. */
  align?: "left" | "center" | "right";
  /** One box per visible line instead of one round the block. */
  per_line?: boolean;
}

export interface TextOpts extends LayerOpts {
  font?: string;
  weight?: number;
  size?: Prop;
  color?: Color | Record<string, unknown>;
  /** A gradient or `"metal"` that paints the glyphs instead of `color`. */
  fill?: ItemFill;
  /** Text that belongs to a mocked interface: a deliberate size, so the phone-legibility rule does not apply. */
  ui?: boolean;
  align?: "left" | "center" | "right";
  width?: number;
  role?: "display" | "title" | "body";
  preset?: { name: string; at?: Seconds; [key: string]: unknown };
  tracking?: number;
  /** Italic type (`italic: true`); there is no `style` or `fontStyle` key. */
  italic?: boolean;
  /** Line height multiplier. */
  leading?: number;
  /** Restyle stretches of the text: `{ match: "render", color: "#ff5a1f", font, weight, italic, nth }`. */
  runs?: TextRun[];
  /** A highlighter swept behind a word: `{ word, at, color }` (see `comp catalogue`). */
  mark?: Record<string, unknown>;
  /** A box that grows with the characters showing, behind the glyphs: `{ fit: "text", padding: [24, 12], radius: 16, fill: "#e63946" }`. */
  box?: TextBox;
  /** Per-character, word or line animations: build them with `charFlip`, `decode` or write `{ by, props, stagger | selector }`. */
  animators?: TextAnimator[];
  stroke?: { color: Color; width: number };
  /** Outlined type: paint only the stroke (2 px in the text colour when `stroke` is not given). The key is `outline_only`; `outline` is not a key. */
  outline_only?: boolean;
  /** Draw only `[start, end]` (0..1) of the outline length: animate `end` from 0 to 1 and the letters write themselves. */
  stroke_trim?: [Prop, Prop];
  /** Trim each glyph's own outline instead of the whole line's. */
  stroke_trim_each?: boolean;
  /** Lay the text along a path: see `textOnPath`. */
  path?: TextPathOpts;
  /** Wrap the text on a cylinder or sphere: see `wrapText`. */
  surface?: TextSurfaceOpts;
  /** A seamlessly scrolling row: see `marquee`. */
  marquee?: MarqueeOpts;
  /** Viewer distance in px for per-character 3D on a flat layer, or `"orthographic"`. Default: the default camera's zoom, width / 0.72. */
  perspective?: number | "orthographic";
}

/** What a text animator does to a unit at full weight. Rotations share the layer's `rotation: [x, y, z]` axes and signs. */
export interface TextUnitProps {
  x?: number;
  y?: number;
  /** Added to the scale: `-1` shrinks to nothing, `1` doubles. */
  scale?: number;
  /** Degrees, clockwise (the same as `rotation_z`; the two add). */
  rotation?: number;
  opacity?: number;
  blur?: number;
  tracking?: number;
  color?: Color;
  skew?: number;
  /** Pixels away from the viewer (the layer's z convention). */
  z?: number;
  rotation_x?: number;
  rotation_y?: number;
  rotation_z?: number;
  scale_z?: number;
  /** Degrees added to the fill's hue. */
  fill_hue?: number;
  fill_saturation?: number;
  fill_brightness?: number;
  /** Pixels added to the stroke width. */
  stroke_width?: number;
}

export interface TextSelector {
  start?: Prop;
  end?: Prop;
  offset?: Prop;
  shape?: "square" | "ramp_up" | "ramp_down" | "triangle" | "round" | "smooth";
  smooth?: number;
  random?: boolean;
  seed?: number;
  /** Wiggly selector: each unit's weight is scaled by its own random factor between `min` and `max`. */
  wiggle?: TextWiggle;
}

/** A random factor per unit (`min` 0, `max` 1; either may be negative). `rate` 0 keeps each unit's factor, above 0 it drifts, that many new targets a second. */
export interface TextWiggle {
  min?: number;
  max?: number;
  rate?: number;
  seed?: number;
}

/** Where a cipher's random characters come from: a name, or any string of characters. */
export type CipherCharset = "auto" | "upper" | "lower" | "digits" | "alnum" | "hex" | "binary" | "symbols" | "matrix" | (string & {});

export interface CipherOpts {
  charset?: CipherCharset;
  /** Changes per second (24). */
  rate?: number;
  seed?: number;
  /** A unit locks on to its real character when its weight falls to this or below (0..1; 0). */
  settle?: number;
}

export interface TextAnimator {
  by?: "chars" | "words" | "lines";
  props?: TextUnitProps;
  at?: Seconds;
  dur?: Seconds;
  ease?: Ease;
  direction?: "in" | "out";
  stagger?: { each?: Seconds; from?: "start" | "end" | "center" | "edges" | "random"; seed?: number; ease?: Ease };
  selector?: TextSelector;
  mask?: boolean;
  count_spaces?: boolean;
  cipher?: CipherOpts;
}

/** Text along a path. Exactly one geometry (`d`, `points`, `ellipse`, `rect`, `polygon`, `star`) or `layer`. */
export interface TextPathOpts {
  d?: string;
  points?: Vec2[];
  closed?: boolean;
  smooth?: boolean;
  ellipse?: [number, number, number, number];
  rect?: [number, number, number, number];
  polygon?: [number, number, number, number];
  star?: [number, number, number, number, number];
  /** Borrow the first shape item of the layer with this id. */
  layer?: string;
  reverse?: boolean;
  /** Turn each character to the tangent (true). */
  perpendicular?: boolean;
  force_align?: boolean;
  first_margin?: Prop;
  last_margin?: Prop;
  /** Fractions of the path length added to `first_margin`; 0 to 1 is one exact lap of a closed path. */
  travel?: Prop;
}

export interface TextSurfaceOpts {
  kind?: "cylinder" | "sphere";
  /** Pixels; the text's width over pi when left out. */
  radius?: Prop;
  /** Degrees the surface has turned; animate it to scroll the text round. */
  turn?: Prop;
  /** Draw the characters that face away (mirrored). */
  back?: boolean;
}

export interface MarqueeOpts {
  /** Pixels per second; negative scrolls left (-120). */
  speed?: number;
  /** Pixels between copies (80). */
  gap?: number;
  /** Copies; enough to cover the composition width when left out. */
  repeat?: number;
  /** Pixels already travelled at time 0. */
  offset?: number;
}

export interface TextRun {
  match: string;
  nth?: number;
  color?: Color;
  font?: string;
  weight?: number;
  italic?: boolean;
}

/** `text` may also be a counter that counts from `from` to `to`. */
export interface CounterSpec {
  from: number;
  to: number;
  at?: Seconds;
  dur?: Seconds;
  ease?: Ease;
  decimals?: number;
  sep?: string;
  prefix?: string;
  suffix?: string;
}

export interface AudioOpts extends LayerOpts {
  volume?: Prop;
  fade_in?: Seconds;
  fade_out?: Seconds;
  src_in?: Seconds;
  speed?: number;
}

export interface ShapeOpts extends LayerOpts {}

export type { PathGeometry, GradientFill, ItemFill, ItemStroke, ItemTrim, ItemRepeat, ItemTaper, TaperTip, TaperExtent, ShapeItem } from "./shape-types";
export type { PaletteCycle, PalettePaint, PaletteOpts, Swatch } from "./palette-types";
export type { Wrap, LayerStagger, Flip, Along, Reflect } from "./layer-motion-types";
export interface SolidOpts extends LayerOpts {
  size?: Vec2;
}
export interface ImageOpts extends LayerOpts {
  /** Nearest preserves pixel art; linear interpolates. Omit for automatic cubic/mipmapped filtering. */
  sampling?: "nearest" | "linear";
  fit?: "cover" | "contain" | "fill" | "none";
  size?: Vec2;
  /** The point kept when cropping, as fractions of the image (0..1). */
  focus?: Vec2;
  /** A Ken Burns move between two framings. */
  camera?: { from: Framing; to: Framing; at?: Seconds; dur?: Seconds; ease?: Ease };
  /** 2.5D: displace by a depth map (white = near). */
  parallax?: { depth: string; strength: Prop; look?: Vec2 | Array<Prop> };
}

/** One end of an image's Ken Burns move: `zoom` 1 is the fitted image; `center` is a fraction of the image. */
export interface Framing {
  zoom?: number;
  center?: Vec2;
}

export interface RectOpts extends ShapeOpts {
  fill?: Color | Record<string, unknown>;
  radius?: number;
  stroke?: { color: Color; width: number; [key: string]: unknown };
}

export interface CameraOpts extends LayerOpts {
  poi?: Vec3 | Array<Prop>;
  zoom?: Prop;
  dof?: { focus: number; blur: number };
}

export interface LightOpts extends LayerOpts {
  kind: "ambient" | "point" | "directional" | "spot";
  color?: Color;
  intensity?: Prop;
  radius?: number;
  cone?: number;
  shine?: number;
  target?: Vec3;
}

export interface TextGeometry {
  kind: "text";
  text: string;
  font?: string;
  weight?: number;
  italic?: boolean;
  /** Font size. */
  size: number;
  /** Extrusion; 0.2 x size when left out. */
  depth?: number;
  /** Chamfer on the front and back edges; 0 is a hard edge. */
  bevel?: number;
}
export interface BoxGeometry { kind: "box"; size: number; height?: number; depth?: number }
export interface SphereGeometry { kind: "sphere"; size: number }
/** A ring facing the viewer: `size` is the outer diameter, `tube` the band's thickness (a quarter of `size`). */
export interface TorusGeometry { kind: "torus"; size: number; tube?: number }
/** `top` is the top diameter (0 makes a cone). */
export interface CylinderGeometry { kind: "cylinder"; size: number; height: number; top?: number }
/** An SVG path `d` (filled) scaled to `size` wide and pushed `depth` deep (0.3 x size when left out). */
export interface ExtrudeGeometry { kind: "extrude"; d: string; size: number; depth?: number; bevel?: number }
export type Geometry = TextGeometry | BoxGeometry | SphereGeometry | TorusGeometry | CylinderGeometry | ExtrudeGeometry;

export interface MeshOpts extends LayerOpts {
  /** The front face and a curved surface's whole skin. */
  color?: Color;
  /** Walls and bevel; a darkened `color` when left out. */
  side?: Color;
  /** The back face; `side` when left out. */
  back?: Color;
  material?: Material;
  /** Segments around a full turn for curved geometry (48). */
  segments?: number;
}

export interface Text3DOpts extends MeshOpts, Omit<TextGeometry, "kind" | "text" | "size"> {
  size?: number;
}

export interface Letters3DOpts extends Text3DOpts {
  /** Extra space between letters, in pixels. */
  tracking?: number;
}

export type { TransitionName, TransitionSpec, MoveName, CameraMoveName, CameraMove, Layerish, ShotOpts, OverlayOpts, Hit, TimelineOpts, CompOpts } from "./film-types";
export type { Animatable, Stagger, TweenControls, TweenVars } from "./tween-types";
