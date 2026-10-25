// A tilted wall of coloured 3D cards that pops in from the centre, with a hero card the camera can dolly toward. The wall's
// origin is the hero card, so a camera aimed at the wall is aimed at the hero.

import { Layer, group, rect, text } from "../layers";
import { grid, rand } from "../motion";
import type { Timeline } from "../timeline";
import type { Material } from "../types";
import { Part, THEME, Theme } from "./util";
import { entrance } from "./appear";
import type { AppearOpt } from "./appear";

export interface CardWallOpts {
  /** An entrance for the whole component (an engine preset such as "fade" or "slide_up", or a spec); see `appear`. */
  appear?: AppearOpt;
  tl: Timeline;
  cols?: number;
  rows?: number;
  /** Cell size [w, h]; cards are drawn 88 % of it. */
  cell?: [number, number];
  /** Wall tilt in degrees: `{ x, y }`. */
  tilt?: { x: number; y: number };
  /** Card colours (picked with `seed`). */
  palette?: string[];
  seed?: number;
  /** Index of the hero card (row * cols + col); default the middle one. */
  hero?: number;
  heroColor?: string;
  /** Words on some cards; `heroLabel` goes on the hero. */
  words?: string[];
  heroLabel?: string;
  /** Chance a card has a word. */
  wordChance?: number;
  /** Where the hero sits in the frame. */
  pos?: [number, number];
  /** When the cards start popping in; `sfx` is one `card-in` hit there (false for none). */
  at?: number;
  sfx?: string | false;
  material?: Material;
  theme?: Partial<Theme>;
  id?: string;
}

/** `cardWall({ tl, cols: 8, rows: 5, seed: 11, hero: 20, at: 0.1 })`: `wall` is the parent, `hero` the hero card layer. */
export function cardWall(o: CardWallOpts): Part {
  const th = { ...THEME, ...o.theme };
  const { tl, cols = 8, rows = 5, cell = [236, 164], tilt = { x: 25, y: -20 }, seed = 1, at = 0, sfx = "card-in", pos = [960, 500] } = o;
  const palette = o.palette ?? [th.turbo, th.cobalt, th.volt, th.paper, th.steel];
  const words = o.words ?? ["GO", "NEW", "4K", "3D", "FAST", "LIVE", "RUST", "FX"];
  const heroIndex = o.hero ?? Math.floor(rows / 2) * cols + Math.floor(cols / 2);
  const id = o.id ?? "wall";
  const paint = rand(seed);
  const material = o.material ?? { specular: 0.8, shininess: 40 };
  const wall = group({ id, three_d: true, pos, rotation: [tilt.x, tilt.y, 0] });
  const tiles = grid(cols, rows, cell);
  const centre = tiles[heroIndex];
  const cells = tiles.map((c) => ({ ...c, x: c.x - centre.x, y: c.y - centre.y }));
  const cards = cells.map((c) =>
    rect(c.i === heroIndex ? cell[0] * 1.27 : cell[0] * 0.88, c.i === heroIndex ? cell[1] * 1.16 : cell[1] * 0.88, {
      id: `${id}-card${c.i}`, fill: c.i === heroIndex ? (o.heroColor ?? "#ffffff") : paint.pick(palette), radius: 18, three_d: true,
      pos: [c.x, c.y, c.i === heroIndex ? -140 : 0], parent: wall, material, extra: c.i === heroIndex ? { lit: false } : undefined,
    }),
  );
  const labels = cells
    .filter((c) => c.i === heroIndex ? !!o.heroLabel : paint() < (o.wordChance ?? 0.35))
    .map((c) =>
      text(c.i === heroIndex ? o.heroLabel! : paint.pick(words), {
        id: `${id}-label${c.i}`, font: th.display, weight: 800, size: c.i === heroIndex ? 54 : 46, color: th.ink, align: "center", three_d: true,
        pos: [c.x, c.y, c.i === heroIndex ? -146 : -6], parent: wall,
      }),
    );
  tl.from(cards, { scale: 0, opacity: 0, rotation: [null, 70, null], duration: 0.55, ease: "back.out(1.7)", stagger: { each: 0.03, from: "center" } }, at);
  tl.from(labels, { opacity: 0, duration: 0.3, stagger: { each: 0.03, from: "center" } }, at + 0.5);
  if (sfx) tl.hit(at, sfx);
  const layers: Layer[] = [wall, ...cards, ...labels];
  return entrance({ layers, end: at + 0.85 + cards.length * 0.03 / 2, wall, cards, labels, hero: cards[heroIndex] }, o.appear, at);
}
