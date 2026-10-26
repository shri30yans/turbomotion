// Layers as builders. A layer holds its static properties and the tweens a timeline attached to it; `toJSON()` turns
// both into one composition layer.

import { checkGroupOpts, checkLayerKeys, checkRectSize, checkSolidColor, checkTextContent, checkTextureShader, LAYER_STATE, refuseLayerAssignment } from "./arg-check";
import { checkImage } from "./image-check";
import { noteSite } from "./site";
import { compileTweens } from "./compile";
import type { Tween } from "./timeline";
import { checkVectors, fillVector, vectorComponents } from "./vector-tweens";
import type {
  CameraOpts, Color, CounterSpec, ImageOpts, LayerOpts, LightOpts, RectOpts, ShapeItem, ShapeOpts, SolidOpts, TextOpts,
} from "./types";
import type { RawSpec, TextureOpts } from "./raw-types";

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
    checkLayerKeys(kind, this.id, Object.keys(this.props));
    this.settleRotation();
    noteSite(this.id);
    // A layer's options live in `props`; assigning one as a field (`layer.effects = ...`) would be written nowhere.
    return new Proxy(this, {
      set(layer, key, value, receiver) {
        if (!LAYER_STATE.includes(String(key))) refuseLayerAssignment(layer.kind, layer.id, String(key));
        return Reflect.set(layer, key, value, receiver);
      },
    });
  }

  /** Make this layer a child of another (it follows the parent's transform). */
  parentTo(parent: Layer | string): this {
    this.props.parent = typeof parent === "string" ? parent : parent.id;
    return this;
  }

  /** Change static properties after creation. */
  set(vars: Record<string, unknown>): this {
    checkLayerKeys(this.kind, this.id, Object.keys(vars));
    const base = Array.isArray(this.props.rotation) ? vectorComponents(this.props, "rotation") : undefined;
    Object.assign(this.props, vars);
    this.settleRotation(base);
    return this;
  }

  /** A partial rotation leaves the axes it does not name where they are. */
  private settleRotation(base?: Array<number | string>): void {
    const value = this.props.rotation;
    if (!Array.isArray(value)) return;
    this.props.rotation = fillVector(base ?? [0, 0, 0], value);
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
    checkVectors(out, this.id);
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
  throw new Error(`${where}: expected a layer (text(), shape(), group()...) but got ${describe(items)}${plainSpecHint(items)}`);
}

/** Every layer in `items`, groups opened so children follow their parent. */
export const flatten = (items: unknown, where = "layers"): Layer[] => list(items, where).flatMap((l) => l.flat());

/** A plain `{ type: "adjustment", ... }` object is a layer spec, which only `raw()` turns into a layer. */
function plainSpecHint(v: unknown): string {
  const type = v && typeof v === "object" ? (v as { type?: unknown }).type : undefined;
  return typeof type === "string" ? `; a plain layer spec needs raw(): raw(${JSON.stringify(v).slice(0, 80)})` : "";
}

export function describe(v: unknown): string {
  if (v === null) return "null";
  if (typeof v === "object") return `an object with keys ${Object.keys(v as object).slice(0, 5).join(", ")}`;
  return `${typeof v} ${String(v)}`;
}

/**
 * A text layer: `text("Hello", { size, font, weight, color, pos, preset })`. Also takes a counter `{ from, to, dur }`.
 * @example
 * import { comp, text } from "turbomotion";
 * const film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: "#1a1d29" });
 * film.add(
 *   text("Hello", { id: "title", size: 160, weight: 800, color: "#ffffff", align: "center", pos: [960, 440], preset: { name: "rise", at: 0.2 } }),
 *   text({ from: 0, to: 100, dur: 1.5, suffix: "%" }, { id: "count", size: 120, color: "#ffd166", align: "center", pos: [960, 680] }),
 * );
 * export default film;
 */
export const text = (content: string | CounterSpec, o: TextOpts = {}): Layer => {
  checkTextContent(content);
  return new Layer("text", { text: content, ...o });
};

/**
 * A flat rectangle: backgrounds, flashes, mattes.
 * @example
 * import { comp, solid, text } from "turbomotion";
 * const film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: "#1a1d29" });
 * film.add(
 *   solid("#2a4d9b", { id: "card", size: [960, 540], pos: [960, 540] }),
 *   text("Card", { id: "label", size: 140, weight: 800, color: "#ffffff", align: "center", pos: [960, 540] }),
 * );
 * export default film;
 */
export const solid = (color: Color, o: SolidOpts = {}): Layer => {
  checkSolidColor(color);
  return new Layer("solid", { color, ...o });
};

/**
 * A generator shader painted over the whole layer: a noise field, a gradient, grain, or your own `effects/<name>.sksl` with
 * `@kind generator`. `turbomotion comp fx list` shows each generator's parameters (`params`).
 * @example
 * import { comp, texture, text } from "turbomotion";
 * const film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: "#1a1d29" });
 * film.add(
 *   texture("noise", { id: "plate", size: [1920, 1080], pos: [960, 540], opacity: 0.5 }),
 *   text("Plate", { size: 160, color: "#ffffff", align: "center", pos: [960, 540] }),
 * );
 * export default film;
 */
export const texture = (shader: string, o: TextureOpts = {}): Layer => {
  checkTextureShader(shader);
  return new Layer("texture", { shader, ...o });
};

/**
 * A picture: `image("photo.jpg", { fit, focus, camera, parallax })`.
 * @example
 * import { comp, image } from "turbomotion";
 * const film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: "#1a1d29" });
 * film.add(image("photo.jpg", { id: "photo", fit: "cover", camera: { from: { zoom: 1 }, to: { zoom: 1.15 }, dur: 3 } }));
 * export default film;
 */
export const image = (src: string, o: ImageOpts = {}): Layer => new Layer("image", { src: checkImage(src, o), ...o });

/**
 * A vector shape from raw items: `{rect: [x, y, w, h], radius, fill, stroke}`, `{ellipse: [...]}`, `{d: "M..."}`.
 * @example
 * import { comp, shape } from "turbomotion";
 * const film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: "#1a1d29" });
 * film.add(shape(
 *   [{ ellipse: [0, 0, 200, 200], fill: "#ff5a1f" }, { rect: [-300, 240, 600, 16], fill: "#ffffff" }],
 *   { id: "badge", pos: [960, 440] },
 * ));
 * export default film;
 */
export const shape = (items: ShapeItem[], o: ShapeOpts = {}): Layer => {
  if (!Array.isArray(items)) throw new Error("shape(items, opts): items must be a list like [{ rect: [-50, -50, 100, 100], fill: \"#fff\" }]");
  return new Layer("shape", { items, ...o });
};

/**
 * A rectangle centred on its position, optionally rounded: `rect(260, 340, { fill: "#ff6b3d", radius: 18 })`.
 * @example
 * import { comp, rect } from "turbomotion";
 * const film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: "#1a1d29" });
 * film.add(rect(260, 340, { id: "card", fill: "#ff6b3d", radius: 18, pos: [960, 540] }));
 * export default film;
 */
export function rect(w: number, h: number, o: RectOpts = {}): Layer {
  checkRectSize(w, h);
  const { fill, radius, stroke, ...layer } = o;
  const item: Record<string, unknown> = { rect: [-w / 2, -h / 2, w, h] };
  if (radius !== undefined) item.radius = radius;
  if (fill !== undefined) item.fill = fill;
  if (stroke !== undefined) item.stroke = stroke;
  return shape([item], layer);
}

/**
 * An invisible handle: parent layers to it to move, scale or rotate them as one.
 * @example
 * import { comp, group, rect } from "turbomotion";
 * const film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: "#1a1d29" });
 * film.add(group({ id: "rig", pos: [960, 540] }, [
 *   rect(300, 300, { id: "left", fill: "#ff6b3d", pos: [-200, 0] }),
 *   rect(300, 300, { id: "right", fill: "#3d6bff", pos: [200, 0] }),
 * ]));
 * export default film;
 */
export function group(o: LayerOpts = {}, children: unknown = []): Layer {
  checkGroupOpts(o);
  const g = new Layer("null", o);
  g.children = list(children, "group children");
  return g;
}

/**
 * A 3D camera: `camera({ pos, poi, zoom, dof })`. Shots with 3D layers and no camera get a default one.
 * @example
 * import { comp, box, camera } from "turbomotion";
 * const film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: "#1a1d29" });
 * film.add(
 *   box({ size: 300 }, { id: "cube", color: "#e8402f", pos: [960, 540, 0], rotation: [20, 30, 0] }),
 *   camera({ id: "cam", pos: [960, 540, -1800], poi: [960, 540, 0] }),
 * );
 * export default film;
 */
export const camera = (o: CameraOpts = {}): Layer => new Layer("camera", o);

/**
 * A light that shades every `three_d` layer: `light({ kind: "point", pos, color, intensity, radius })`; kinds are ambient, point, directional and spot.
 * @example
 * import { comp, box, camera, light } from "turbomotion";
 * const film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: "#1a1d29" });
 * film.add(
 *   light({ id: "key", kind: "point", pos: [400, 200, -500], color: "#ffd9a0", intensity: 1.2, radius: 1400 }),
 *   box({ size: 300 }, { id: "cube", color: "#e8402f", pos: [960, 540, 0], rotation: [20, 30, 0] }),
 *   camera({ id: "cam", pos: [960, 540, -1800], poi: [960, 540, 0] }),
 * );
 * export default film;
 */
export const light = (o: LightOpts): Layer => {
  if (!o || !o.kind) throw new Error('light({ kind }) needs a kind: "ambient", "point", "directional" or "spot"');
  return new Layer("light", o);
};

/**
 * Escape hatch: any layer the engine knows, as written in `turbomotion comp catalogue`.
 * @example
 * import { comp, raw, solid } from "turbomotion";
 * const film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: "#1a1d29" });
 * film.add(
 *   solid("#2a4d9b", { id: "plate" }),
 *   raw({ type: "adjustment", id: "grade", fx: [{ effect: "vignette" }] }),
 * );
 * export default film;
 */
export const raw = (spec: RawSpec): Layer => {
  const { type, ...rest } = spec;
  if (typeof type !== "string") throw new Error('raw(spec): spec needs a "type", like "particles"');
  return new Layer(type, rest);
};
