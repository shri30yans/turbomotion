// Chapter 7: Motion Tile. One small card (a precomp) is tiled three ways by the `motion_tile` effect: plain, with mirrored
// edges, and with the phase, the tile size and the tile centre animated.

import { solid, text, timeline } from "turbomotion";
import type { Layer } from "turbomotion";
import { C, FONT, T0, border, frame, newFilm, place, subComp, tag } from "./theme";

const DUR = 5.5;
const CARD = 140;
const OUT = CARD * 3;
const tl = timeline();
const GO = T0 + 0.2;

/** The source: four colour quadrants, a white "F" (so mirroring is visible) and a corner mark. */
const card = subComp(CARD, CARD, DUR, [
  solid(C.orange, { id: "q1", size: [70, 70], pos: [35, 35] }),
  solid(C.violet, { id: "q2", size: [70, 70], pos: [105, 35] }),
  solid(C.cyan, { id: "q3", size: [70, 70], pos: [35, 105] }),
  solid(C.pink, { id: "q4", size: [70, 70], pos: [105, 105] }),
  text("F", { id: "f", font: FONT.display, size: 92, color: "#ffffff", pos: [38, 72], align: "left" }),
  solid("#ffffff", { id: "mark", size: [22, 10], pos: [18, 9] }),
]);

// the tiled output is larger than the card, so the effect runs in layer space, where the layer's picture may grow
const film = newFilm(DUR, { effect_space: "layer", extra: { comps: { card } } });

const GAP = 80;
const X0 = 96 + CARD + GAP + OUT / 2; // centre of the first panel
const CY = 500;
const tiled = (id: string, cx: number, fx: Record<string, unknown>): Layer =>
  place(id, "card", cx - CARD / 2, CY - CARD / 2, { fx: [{ effect: "motion_tile", output_width: 300, output_height: 300, ...fx }] });

const cx = [X0, X0 + OUT + GAP, X0 + 2 * (OUT + GAP)];
film.add(
  place("source", "card", 96, CY - CARD / 2),
  border("source-bd", 96, CY - CARD / 2, CARD, CARD),
  tag("source-lab", "source", 96, CY + CARD / 2 + 36, C.dim),
  tiled("tile-plain", cx[0], {}),
  tiled("tile-mirror", cx[1], { mirror_edges: "on" }),
  tiled("tile-anim", cx[2], {
    mirror_edges: "on",
    horizontal_phase_shift: "on",
    phase: { from: 0, to: 270, at: GO, dur: 3.4, ease: "power2.inOut" },
    tile_width: { from: 100, to: 60, at: GO, dur: 3.4, ease: "power2.inOut" },
    tile_height: { from: 100, to: 60, at: GO, dur: 3.4, ease: "power2.inOut" },
    tile_center: { from: [0.5, 0.5], to: [0.75, 0.3], at: GO, dur: 3.4, ease: "power2.inOut" },
  }),
);
cx.forEach((x, i) => {
  film.add(border(`bd${i}`, x - OUT / 2, CY - OUT / 2, OUT, OUT));
});
film.add(
  tag("lab-plain", "tiles", cx[0] - OUT / 2, CY + OUT / 2 + 36, C.text),
  tag("lab-mirror", "mirror_edges: on", cx[1] - OUT / 2, CY + OUT / 2 + 36, C.text),
  tag("lab-anim", "phase + size + centre", cx[2] - OUT / 2, CY + OUT / 2 + 36, C.text),
);

frame(film, tl, {
  num: "07",
  title: "Motion Tile",
  sub: "Tile a layer over a larger output, with mirrored edges and animated phase.",
  caption: "Mirrored edges meet edge to edge, with no seams. On the right the phase, the tile size and the tile centre all animate.",
  hot: ["no seams", "phase", "tile size", "tile centre"],
  dur: DUR,
  accent: C.cyan,
});
export default film;
