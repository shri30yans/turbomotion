// A two-sided card in 3D: it turns over to show its back. Each side is a fill, shape items and an optional label.

import { group, shape, text } from "../layers";
import type { Layer } from "../layers";
import type { Timeline } from "../timeline";
import type { Color, Vec2, Vec3 } from "../types";
import type { Items } from "./cube";
import { Part, THEME } from "./util";

export interface TileFace {
  fill?: Color;
  /** Shape items drawn on the face, centred on it. */
  items?: Items;
  /** Text centred on the face. */
  label?: string;
  labelColor?: Color;
  font?: string;
  weight?: number;
  /** Label size in px (a quarter of the tile's height). */
  labelSize?: number;
}

export interface FlipTileOpts {
  tl: Timeline;
  id: string;
  /** Width and height, or one number for a square. */
  size: number | Vec2;
  pos: Vec2 | Vec3;
  front: TileFace;
  back: TileFace;
  /** Seconds at which the turn starts, and how long it takes. */
  at: number;
  dur?: number;
  ease?: string;
  /** Turn about the vertical axis ("y") or the horizontal one ("x"). */
  axis?: "x" | "y";
  radius?: number;
  edge?: Color;
  sfx?: string;
}

export interface FlipTilePart extends Part {
  root: Layer;
  front: Layer;
  back: Layer;
}

/** `flipTile({ tl, id: "t", size: 240, pos: [960, 540], front: { fill: "#3D6BFF", label: "?" }, back: { fill: "#C8F031", label: "A" }, at: 1 })`. */
export function flipTile(o: FlipTileOpts): FlipTilePart {
  const { tl, id, at, dur = 0.7, ease = "expo.inOut", axis = "y", radius = 14, edge = "#ffffff" } = o;
  const [w, h] = typeof o.size === "number" ? [o.size, o.size] : o.size;
  const [x, y, z = 0] = o.pos;
  const root = group({ id, three_d: true, pos: [x, y, z] });
  const side = (f: TileFace, name: string, flipped: boolean): Layer[] => {
    const turn = flipped ? (axis === "y" ? { rotation: [0, 180, 0] } : { rotation: [180, 0, 0] }) : {};
    const dz = flipped ? 0.5 : -0.5;
    const card = shape([{ rect: [-w / 2, -h / 2, w, h], radius, fill: f.fill ?? THEME.ink, stroke: { color: edge, width: 2 } }, ...(f.items ?? [])], { id: `${id}-${name}`, three_d: true, parent: root, pos: [0, 0, dz], ...turn });
    if (!f.label) return [card];
    const label = text(f.label, { id: `${id}-${name}-label`, font: f.font ?? THEME.text, weight: f.weight ?? 700, size: f.labelSize ?? h / 4, color: f.labelColor ?? "#ffffff", align: "center", three_d: true, parent: root, pos: [0, 0, dz * 3], ...turn });
    return [card, label];
  };
  const [front, back] = [side(o.front, "front", false), side(o.back, "back", true)];
  tl.to(root, { rotation: axis === "y" ? [null, 180, null] : [180, null, null], duration: dur, ease }, at);
  if (o.sfx) tl.hit(at, o.sfx);
  return { layers: [root, ...front, ...back], root, front: front[0], back: back[0], end: at + dur };
}
