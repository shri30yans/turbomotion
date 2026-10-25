// A 3D word that tumbles in letter by letter: `letters3d` plus the launch film's entrance (stagger, back.out) as one call.

import { letters3d } from "../three";
import type { Timeline } from "../timeline";
import type { Material, Text3DOpts } from "../types";
import { Part, THEME, Theme } from "./util";

export interface Title3DOpts extends Omit<Text3DOpts, "pos"> {
  tl: Timeline;
  /** Seconds at which the first letter starts to land. */
  at?: number;
  pos?: [number, number, number];
  /** Pixels between letters. */
  tracking?: number;
  /** Seconds between letters. */
  stagger?: number;
  duration?: number;
  ease?: string;
  /** A sound per letter at its tween start. */
  sfx?: string;
  theme?: Partial<Theme>;
}

/** `title3d("MOTION", { tl, at: 0.3, size: 240 })`: `letters` are the layers, `end` is when the last one has landed. */
export function title3d(word: string, o: Title3DOpts): Part {
  const th = { ...THEME, ...o.theme };
  const { tl, at = 0.3, pos = [960, 540, 0], stagger = 0.05, duration = 1, ease = "back.out(1.4)", sfx, theme: _t, ...rest } = o;
  const material: Material = { specular: 0.7, shininess: 40, ...(rest.material ?? {}) };
  const letters = letters3d(word, {
    font: th.display, weight: 800, size: 240, depth: 70, bevel: 6, color: th.paper, ...rest, material, pos,
  });
  tl.from(letters, { rot_x: -100, rot_y: 60, opacity: 0, z: pos[2] - 400, duration, ease, stagger, sfx }, at);
  const lastAt = at + (letters.length - 1) * stagger;
  return { layers: letters, end: lastAt + duration, letters, lastAt, landed: lastAt + duration * 0.7 };
}
