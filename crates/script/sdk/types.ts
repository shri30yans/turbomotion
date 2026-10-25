// Shared types. Everything an author can write is described here, so an editor catches mistakes before the render does.

export type Ease = string; // any engine ease: "none", "power2.out", "back.out(1.4)", "elastic.out(1, 0.4)", "steps(8)"...
export type Vec2 = [number, number];
export type Vec3 = [number, number, number];
export type Color = string | [number, number, number] | [number, number, number, number];
export type Seconds = number;

/** A property: a constant, an expression string, or any raw form the engine accepts (tween, keys, loop). */
export type Prop<T = number> = T | string | { [key: string]: unknown };

export interface Material {
  specular?: number;
  shininess?: number;
  [key: string]: unknown;
}

export interface Extrude {
  depth: number;
  color?: Color;
  far?: Color;
}

export interface Transit {
  preset: string;
  dur?: Seconds;
  [key: string]: unknown;
}

/** What every layer takes. `x`, `y`, `z` are shorthand for `pos`. */
export interface LayerOpts {
  id?: string;
  pos?: Vec2 | Vec3 | Array<Prop>;
  x?: number;
  y?: number;
  z?: number;
  anchor?: Vec2 | Vec3;
  scale?: Prop | [Prop, Prop];
  rotation?: Prop;
  rot_x?: Prop;
  rot_y?: Prop;
  opacity?: Prop;
  three_d?: boolean;
  material?: Material;
  extrude?: Extrude;
  start?: Seconds;
  dur?: Seconds;
  parent?: string | { id: string };
  blend?: string;
  fx?: Array<Record<string, unknown>>;
  masks?: Array<Record<string, unknown>>;
  matte?: Record<string, unknown>;
  enter?: Transit;
  exit?: Transit;
  hidden?: boolean;
  mblur?: boolean;
  /** Anything else the engine takes (see `turbomotion comp catalogue`); merged as written. */
  extra?: Record<string, unknown>;
}

export interface TextOpts extends LayerOpts {
  font?: string;
  weight?: number;
  size?: number;
  color?: Color | Record<string, unknown>;
  align?: "left" | "center" | "right";
  width?: number;
  role?: "display" | "title" | "body";
  preset?: { name: string; at?: Seconds; [key: string]: unknown };
  tracking?: number;
  italic?: boolean;
  /** Line height multiplier. */
  leading?: number;
  /** Restyle stretches of the text: `{ match: "render", color: "#ff5a1f", font, weight, italic, nth }`. */
  runs?: TextRun[];
  /** A highlighter swept behind a word: `{ word, at, color }` (see `comp catalogue`). */
  mark?: Record<string, unknown>;
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
export interface SolidOpts extends LayerOpts {
  size?: Vec2;
}
export interface ImageOpts extends LayerOpts {
  /** Nearest preserves pixel art; linear interpolates. Omit for automatic cubic/mipmapped filtering. */
  sampling?: "nearest" | "linear";
  fit?: "cover" | "contain" | "fill" | "none";
  size?: Vec2;
  focus?: Vec2;
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
