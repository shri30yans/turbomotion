// Chapter 6: repeater palette cycle and stroke taper. Left: three layer repeaters take their colours from one palette,
// each cycling its own way with an animated offset. Right: tapered strokes drawing on, thin at the ends and full in the middle.

import { rect, shape, timeline } from "turbomotion";
import { C, T0, border, frame, newFilm, tag } from "./theme";

const DUR = 6;
const tl = timeline();
const film = newFilm(DUR);
const GO = T0 + 0.2;

// left panel: palette cycle ---------------------------------------------------------------------------------------
const PALETTE = [C.orange, C.pink, C.violet, C.cyan, C.lime];
film.add(rect(840, 700, { id: "left-bg", fill: C.panel, pos: [516, 490] }), border("left-bd", 96, 140, 840, 700));
film.add(tag("pal-lab", "palette", 124, 190, C.dim));
PALETTE.forEach((c, i) => film.add(rect(56, 56, { id: `sw${i}`, radius: 10, fill: c, pos: [330 + i * 72, 190] })));

const rows: Array<{ mode: "step" | "gradient" | "ping_pong"; y: number }> = [
  { mode: "step", y: 360 },
  { mode: "gradient", y: 560 },
  { mode: "ping_pong", y: 760 },
];
for (const r of rows) {
  film.add(
    tag(`lab-${r.mode}`, `cycle: ${r.mode}`, 124, r.y - 62, C.text),
    shape([{ ellipse: [0, 0, 31, 31], fill: "#ffffff" }], {
      id: `row-${r.mode}`,
      pos: [160, r.y],
      repeat: {
        count: 11, offset: [68, 0], palette: PALETTE,
        cycle: { mode: r.mode, offset: { from: 0, to: 7, at: GO, dur: 5.8, ease: "none" } },
      },
    }),
  );
}

// right panel: stroke taper ----------------------------------------------------------------------------------------
film.add(rect(840, 700, { id: "right-bg", fill: C.panel, pos: [1404, 490] }), border("right-bd", 984, 140, 840, 700));
const draw = { from: 0, to: 1, at: GO, dur: 2.4, ease: "power2.inOut" };
film.add(
  shape(
    [{
      d: "M 1030 560 C 1180 180 1330 180 1420 380 S 1620 640 1780 250",
      stroke: { color: C.orange, width: 60, cap: "round", join: "round" },
      taper: { start: { length: 380, width: 0 }, end: { length: { fraction: 0.14 }, width: { fraction: 0.3 } }, ease: "power2.out" },
      trim: { end: draw },
    }],
    { id: "ribbon" },
  ),
  shape(
    [{
      ellipse: [1150, 745, 80, 80],
      stroke: { color: C.lime, width: 34, cap: "round" },
      taper: { start: { length: 260 }, end: { length: 150 } },
      trim: { end: { ...draw, at: GO + 0.6 } },
    }],
    { id: "ring" },
  ),
  shape(
    [{
      d: "M 1330 745 L 1790 745",
      stroke: { color: C.cyan, width: 40, cap: "butt" },
      taper: { start: { length: 200, width: 0 }, end: { length: 160, width: 6 }, ease: "sine.inOut" },
      trim: { end: { ...draw, at: GO + 1.2 } },
    }],
    { id: "line" },
  ),
  tag("t-ribbon", "start: point, end: 30% width", 1010, 190, C.orange),
  tag("t-ring", "closed: pinched", 1010, 640, C.lime),
  tag("t-line", "line: eased ends", 1330, 640, C.cyan),
);

frame(film, tl, {
  num: "06",
  title: "Palette cycle and taper",
  sub: "Repeater copies take colours from a palette; strokes thin toward their ends.",
  caption: "One palette, three ways to cycle it, with the offset animated. On the right the stroke width is a property of the path: pointed, eased or pinched.",
  hot: ["offset animated", "pointed", "eased", "pinched"],
  dur: DUR,
  accent: C.pink,
});
export default film;
