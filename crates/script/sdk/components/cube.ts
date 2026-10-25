// Cubes made of 3D shape faces: a solid cube with its own art on each face, a 3x3x3 puzzle cube whose slabs twist, and
// a tile that flips from art to a letter. Every function returns `{ root, layers, ... }`: put `layers` straight into a shot
// and move, spin or scale `root`.

import { group, shape, text } from "../layers";
import type { Layer } from "../layers";
import type { Color, Material, Vec3 } from "../types";
import { THEME } from "./util";
import type { Part } from "./util";

export type Items = Array<Record<string, unknown>>;
/** One face: a fill, and shape items drawn on it (centred on the face, in pixels). */
export interface CubeFace { fill?: Color; items?: Items }

export const CUBE_COLORS: Color[] = ["#3EE6D6", "#FFE94A", "#FF57E6", "#8F6CFF", "#C4F4FF", "#FF4A55", "#52E08A", "#FFB24A"];
const MATERIAL: Material = { specular: 0.5, shininess: 30 };
/** The six faces: [x rotation, y rotation, x, y, z] in units of half the side. Order: front (towards the lens), back, right, left, top, bottom. */
const FACES: Array<[number, number, number, number, number]> = [
  [0, 0, 0, 0, -1], [0, 180, 0, 0, 1], [0, 90, 1, 0, 0], [0, -90, -1, 0, 0], [90, 0, 0, -1, 0], [-90, 0, 0, 1, 0],
];

export interface CubeOpts {
  id: string;
  size: number;
  /** One colour or face for all six, or up to six in the order front, back, right, left, top, bottom (missing ones reuse the first fill). */
  faces: Color | CubeFace | Array<Color | CubeFace>;
  pos?: Vec3;
  /** Starting rotation in degrees: [x, y, z]. */
  rot?: Vec3;
  /** Outline colour and width. */
  edge?: Color;
  edgeWidth?: number;
  material?: Material;
}

export interface CubePart extends Part {
  root: Layer;
  faces: Layer[];
}

const face = (f: Color | CubeFace): CubeFace => (typeof f === "string" || Array.isArray(f) ? { fill: f } : f);

/** `cube({ id: "c", size: 200, faces: ["#3D6BFF", { fill: "#FF5A1F", items: icon }] , pos: [960, 540, 0] })`. */
export function cube(o: CubeOpts): CubePart {
  const { id, size: s, edge = "#ffffff", edgeWidth = Math.max(1.5, s * 0.025), material = MATERIAL } = o;
  if (!(s > 0)) throw new Error(`cube("${id}"): size must be above 0, got ${s}`);
  const list = (Array.isArray(o.faces) && typeof o.faces[0] !== "number" ? o.faces : [o.faces]) as Array<Color | CubeFace>;
  if (!list.length || list.length > 6) throw new Error(`cube("${id}"): faces takes one face or up to six, got ${list.length}`);
  const [rx = 0, ry = 0, rz = 0] = o.rot ?? [];
  const root = group({ id, three_d: true, pos: o.pos ?? [0, 0, 0], rotation: [rx, ry, rz] });
  const faces = FACES.map(([frx, fry, fx, fy, fz], i) => {
    const f = i < list.length ? face(list[i]) : { fill: face(list[0]).fill };
    const items: Items = [{ rect: [-s / 2, -s / 2, s, s], fill: f.fill ?? THEME.cobalt, stroke: { color: edge, width: edgeWidth } }, ...(f.items ?? [])];
    return shape(items, { id: `${id}-f${i}`, three_d: true, parent: root, pos: [(fx * s) / 2, (fy * s) / 2, (fz * s) / 2], rotation: [frx, fry, 0], material });
  });
  return { root, faces, layers: [root, ...faces], end: 0 };
}

export interface PuzzleCubeOpts {
  id: string;
  size: number;
  /** Tile colours, cycled with `seed` (the cube palette). */
  palette?: Color[];
  /** Art for a side tile: `(running index, tile art size)` returns shape items centred on 0,0. */
  tile?: (index: number, size: number) => Items;
  seed?: number;
  pos?: Vec3;
  edge?: Color;
  material?: Material;
}

export interface PuzzleCubePart extends Part {
  root: Layer;
  /** Top, middle, bottom: groups to rotate about y to twist a layer. */
  slabs: Layer[];
}

/** A 3x3x3 puzzle cube: three slabs you can twist, each tile its own colour and art. */
export function puzzleCube(o: PuzzleCubeOpts): PuzzleCubePart {
  const { id, size: s, palette = CUBE_COLORS, tile, edge = "#ffffff", material = MATERIAL } = o;
  const t = s / 3;
  const root = group({ id, three_d: true, pos: o.pos ?? [0, 0, 0] });
  const [slabs, layers]: [Layer[], Layer[]] = [[], [root]];
  let k = o.seed ?? 0;
  const colour = (step: number): Color => palette[(k++ * step) % palette.length];
  const side = (c: number): Items => {
    const cx = (c - 1) * t;
    const art = tile ? tile(k + c, t * 0.62).map((it) => ({ ...it, offset: [cx, 0] })) : [];
    return [{ rect: [cx - t / 2 + 1.5, -t / 2 + 1.5, t - 3, t - 3], radius: 3, fill: colour(3), stroke: { color: edge, width: 2 } }, ...art];
  };
  for (let row = 0; row < 3; row++) {
    const slab = group({ id: `${id}-slab${row}`, three_d: true, parent: root, pos: [0, (row - 1) * t, 0] });
    slabs.push(slab);
    layers.push(slab);
    [[0, 0, -1], [180, 0, 1], [90, 1, 0], [-90, -1, 0]].forEach(([ry, fx, fz], i) => {
      layers.push(shape([0, 1, 2].flatMap(side), { id: `${id}-s${row}${i}`, three_d: true, parent: slab, pos: [(fx * s) / 2, 0, (fz * s) / 2], rotation: [0, ry, 0], material }));
    });
    if (row !== 1) {
      const top = row === 0;
      const items = [0, 1, 2].flatMap((r) => [0, 1, 2].map((c) => ({ rect: [(c - 1) * t - t / 2, (r - 1) * t - t / 2, t - 3, t - 3], radius: 3, fill: colour(5), stroke: { color: edge, width: 2 } })));
      layers.push(shape(items, { id: `${id}-c${row}`, three_d: true, parent: slab, pos: [0, top ? -t / 2 : t / 2, 0], rotation: [top ? 90 : -90, 0, 0], material }));
    }
  }
  return { root, slabs, layers, end: 0 };
}

export interface LetterCubeOpts {
  id: string;
  size: number;
  fill: Color;
  /** Art on the front face. */
  items?: Items;
  /** The thin white letter on the back: turn `root` by 180 about y to reveal it. */
  letter: string;
  font?: string;
  weight?: number;
  letterColor?: Color;
  pos?: Vec3;
  edge?: Color;
}

/** A tile that flips from art (front) to a letter (back): rotate `root` 180 about y. */
export function letterCube(o: LetterCubeOpts): CubePart {
  const { id, size: s, fill, items, letter, pos, edge } = o;
  const c = cube({ id, size: s, faces: [{ fill, items }, { fill }, fill], pos, edge });
  const mark = text(letter, { id: `${id}-letter`, font: o.font ?? THEME.text, weight: o.weight ?? 200, size: s * 0.52, color: o.letterColor ?? "#ffffff", align: "center", three_d: true, parent: c.root, pos: [0, 0, s / 2 + 1], rotation: [0, 180, 0] });
  return { ...c, faces: [...c.faces, mark], layers: [...c.layers, mark] };
}
