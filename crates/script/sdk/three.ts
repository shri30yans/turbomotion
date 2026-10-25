// Solids. Every 3D object is one `mesh` layer whose `geometry` says what it is made of: glyph outlines, a box, a sphere,
// a torus, a cylinder or an SVG path pushed into depth. `text3d` and `letters3d` are the text geometry; `spin` turns
// any of them about its own axis.

import { Layer } from "./layers";
import { measure } from "./measure";
import type { BoxGeometry, CylinderGeometry, ExtrudeGeometry, Geometry, Letters3DOpts, MeshOpts, SphereGeometry, Text3DOpts, TorusGeometry, Vec2, Vec3 } from "./types";

/** A mesh from any geometry: `mesh({ kind: "sphere", size: 300 }, { color: "#e8402f", pos: [960, 540, 0] })`. */
export function mesh(geometry: Geometry, o: MeshOpts = {}): Layer {
  return new Layer("mesh", { three_d: true, geometry, ...o });
}

/** A box: `box({ size, height?, depth? }, { color, side, pos, rotation })`. */
export const box = (g: Omit<BoxGeometry, "kind">, o?: MeshOpts): Layer => mesh({ kind: "box", ...g }, o);
/** A sphere of diameter `size`. */
export const sphere = (g: Omit<SphereGeometry, "kind">, o?: MeshOpts): Layer => mesh({ kind: "sphere", ...g }, o);
/** A ring facing the viewer: outer diameter `size`, band thickness `tube`. */
export const torus = (g: Omit<TorusGeometry, "kind">, o?: MeshOpts): Layer => mesh({ kind: "torus", ...g }, o);
/** A cylinder of diameter `size` and `height`; `top: 0` makes a cone. */
export const cylinder = (g: Omit<CylinderGeometry, "kind">, o?: MeshOpts): Layer => mesh({ kind: "cylinder", ...g }, o);
/** An SVG path `d` filled and pushed `depth` deep. */
export const extrude = (g: Omit<ExtrudeGeometry, "kind">, o?: MeshOpts): Layer => mesh({ kind: "extrude", ...g }, o);

/** Extruded text: `text3d("Hi", { font, weight, size, depth, bevel, color, side, material, ...layer })`. */
export function text3d(content: string, o: Text3DOpts = {}): Layer {
  const { font, weight, italic, size = 120, depth, bevel, ...layer } = o;
  return mesh({ kind: "text", text: content, font, weight, italic, size, depth, bevel }, layer);
}

/**
 * Split `word` into one 3D text layer per letter, centred on `pos` (default the origin: pass your own). Letter boxes
 * come from the renderer's own shaping (`measure`), so kerning and tracking match flat text. Spaces take room but make
 * no layer. Returns the layers in reading order, ready for a staggered tween.
 */
export function letters3d(word: string, o: Letters3DOpts = {}): Layer[] {
  const { tracking = 0, pos, x, y, z, id, ...rest } = o;
  const size = rest.size ?? 120;
  const m = measure(word, { font: rest.font, size, weight: rest.weight, italic: rest.italic, tracking });
  const centre: Vec3 | Vec2 = (pos as Vec3 | Vec2 | undefined) ?? [x ?? 0, y ?? 0, z ?? 0];
  const left = centre[0] - m.width / 2;
  const prefix = id ?? "letter";
  const out: Layer[] = [];
  m.chars.forEach((c, i) => {
    if (c.ch.trim() === "") return;
    out.push(text3d(c.ch, { ...rest, id: `${prefix}${i}`, pos: [left + c.x + c.w / 2, centre[1], centre[2] ?? 0] as Vec3 }));
  });
  return out;
}

/**
 * A turntable: the angle of a steady turn as an expression, for the Z Rotation of a 3D object
 * (`rotation`), or any of the three axes (`rotation: [0, spin(90), 0]`). `box({ size: 200 }, { rotation: spin(90) })`
 * makes a full turn in four seconds; `from` is the starting angle.
 */
export function spin(degreesPerSecond: number, from = 0): string {
  return `${from}+time*${degreesPerSecond}`;
}
