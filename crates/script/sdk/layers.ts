// Layers as builders. A layer holds its static properties and the tweens a timeline attached to it; `toJSON()` turns
// both into one composition layer.

import { compileTweens } from "./timeline";
import type { Tween } from "./timeline";
import type {
  CameraOpts, Color, CounterSpec, ImageOpts, LayerOpts, LightOpts, RectOpts, ShapeOpts, SolidOpts, TextOpts,
} from "./types";

let counter = 0;

/** A layer of any kind as a builder: its properties plus the tweens a timeline attached to it. */
export class Layer {
  readonly kind: string;
  id: string;
  props: Record<string, any>;
  tweens: Tween[] = [];
  children: Layer[] = [];

  constructor(kind: string, props: Record<string, any> = {}) {
    this.kind = kind;
    const { x, y, z, extra, ...rest } = props;
    this.props = { ...rest, ...(extra ?? {}) };
    if (x !== undefined || y !== undefined || z !== undefined) {
      if (this.props.pos !== undefined) throw new Error(`${kind} layer: give "pos" or x/y/z, not both`);
      this.props.pos = z === undefined ? [x ?? 0, y ?? 0] : [x ?? 0, y ?? 0, z];
    }
    const parent = this.props.parent;
    if (parent && typeof parent === "object") this.props.parent = parent.id;
    for (const k of Object.keys(this.props)) if (this.props[k] === undefined) delete this.props[k];
    this.id = this.props.id ?? `${kind}${++counter}`;
    this.props.id = this.id;
  }

  /** Make this layer a child of another (it follows the parent's transform). */
  parentTo(parent: Layer | string): this {
    this.props.parent = typeof parent === "string" ? parent : parent.id;
    return this;
  }

  /** Change static properties after creation. */
  set(vars: Record<string, unknown>): this {
    Object.assign(this.props, vars);
    return this;
  }

  /** This layer and everything parented under it, parents first. */
  flat(): Layer[] {
    const kids = this.children.flatMap((c) => {
      if (!c.props.parent) c.props.parent = this.id;
      return c.flat();
    });
    return [this, ...kids];
  }

  toJSON(): Record<string, unknown> {
    const out: Record<string, any> = { type: this.kind, ...this.props };
    compileTweens(out, this.tweens, this.id);
    return out;
  }
}

/** The top-level layers in `items` (arrays are opened; groups stay whole). */
export function list(items: unknown, where = "layers"): Layer[] {
  if (items === undefined || items === null) return [];
  if (Array.isArray(items)) return items.flatMap((i) => list(i, where));
  if (items instanceof Layer) return [items];
  // a component's result ({ layers, ... }) stands for its layers
  if (typeof items === "object" && Array.isArray((items as { layers?: unknown }).layers)) return list((items as { layers: unknown[] }).layers, where);
  throw new Error(`${where}: expected a layer (text(), shape(), group()...) but got ${describe(items)}`);
}

/** Every layer in `items`, groups opened so children follow their parent. */
export const flatten = (items: unknown, where = "layers"): Layer[] => list(items, where).flatMap((l) => l.flat());

export function describe(v: unknown): string {
  if (v === null) return "null";
  if (typeof v === "object") return `an object with keys ${Object.keys(v as object).slice(0, 5).join(", ")}`;
  return `${typeof v} ${String(v)}`;
}

/** A text layer: `text("Hello", { size, font, weight, color, pos, preset })`. Also takes a counter `{ from, to, dur }`. */
export const text = (content: string | CounterSpec, o: TextOpts = {}): Layer => new Layer("text", { text: content, ...o });

/** A flat rectangle: backgrounds, flashes, mattes. */
export const solid = (color: Color, o: SolidOpts = {}): Layer => new Layer("solid", { color, ...o });

/** A picture: `image("photo.jpg", { fit, focus, camera, parallax })`. */
export const image = (src: string, o: ImageOpts = {}): Layer => new Layer("image", { src, ...o });

/** A vector shape from raw items: `{rect: [x, y, w, h], radius, fill, stroke}`, `{ellipse: [...]}`, `{d: "M..."}`. */
export const shape = (items: Array<Record<string, unknown>>, o: ShapeOpts = {}): Layer => {
  if (!Array.isArray(items)) throw new Error("shape(items, opts): items must be a list like [{ rect: [-50, -50, 100, 100], fill: \"#fff\" }]");
  return new Layer("shape", { items, ...o });
};

/** A rectangle centred on its position, optionally rounded: `rect(260, 340, { fill: "#ff6b3d", radius: 18 })`. */
export function rect(w: number, h: number, o: RectOpts = {}): Layer {
  const { fill, radius, stroke, ...layer } = o;
  const item: Record<string, unknown> = { rect: [-w / 2, -h / 2, w, h] };
  if (radius !== undefined) item.radius = radius;
  if (fill !== undefined) item.fill = fill;
  if (stroke !== undefined) item.stroke = stroke;
  return shape([item], layer);
}

/** An invisible handle: parent layers to it to move, scale or rotate them as one. */
export function group(o: LayerOpts = {}, children: unknown = []): Layer {
  const g = new Layer("null", o);
  g.children = list(children, "group children");
  return g;
}

/** A 3D camera: `camera({ pos, poi, zoom, dof })`. Shots with 3D layers and no camera get a default one. */
export const camera = (o: CameraOpts = {}): Layer => new Layer("camera", o);

export const light = (o: LightOpts): Layer => {
  if (!o || !o.kind) throw new Error('light({ kind }) needs a kind: "ambient", "point", "directional" or "spot"');
  return new Layer("light", o);
};

/** Escape hatch: any layer the engine knows, as written in `turbomotion comp catalogue`. */
export const raw = (spec: Record<string, unknown>): Layer => {
  const { type, ...rest } = spec;
  if (typeof type !== "string") throw new Error('raw(spec): spec needs a "type", like "particles"');
  return new Layer(type, rest);
};
