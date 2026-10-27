// Chapter 2: colour pipeline. The same three scenes are computed on display numbers (left) and in linear light (right).
// The film is linear light and each cell is a precomp: the left-hand ones say `color: { linear_light: false }` for
// themselves. (The other way round, a linear-light precomp in a display film, is not converted back: see README.)
// The strip underneath is the darks ramp: 33 small solids, levels 0 to 32.

import { rect, shape, solid, timeline } from "turbomotion";
import type { CompOpts } from "turbomotion";
import { C, T0, frame, newFilm, place, subComp, tag } from "./theme";

const DUR = 6;
const CW = 250;
const CH = 340;
const BLACK = "#000000";
const tl = timeline();

/** Black on the left, white on the right, blurred across the join. */
const edge = (): Array<ReturnType<typeof solid>> => [
  solid("#ffffff", { id: "white", size: [400, 500], pos: [CW / 2 + 200, CH / 2], fx: [{ effect: "blur", radius: 30 }] }),
];

/** Left half: 50% white over black. Right half: black and white 2 px stripes, which average to half the light. */
const half = (): Array<ReturnType<typeof solid> | ReturnType<typeof shape>> => {
  const stripes = Array.from({ length: 31 }, (_, i) => ({ rect: [CW / 2 + 2 + i * 4, 0, 2, CH] as [number, number, number, number], fill: "#ffffff" }));
  return [solid("#ffffff", { id: "half", size: [CW / 2, CH], pos: [CW / 4, CH / 2], opacity: 0.5 }), shape(stripes, { id: "stripes" })];
};

/** Two soft discs added together. */
const glow = (): Array<ReturnType<typeof shape>> => [
  shape([{ ellipse: [0, 0, 80, 80], fill: C.orange }], { id: "a", pos: [100, 150], blend: "add", fx: [{ effect: "blur", radius: 18 }] }),
  shape([{ ellipse: [0, 0, 80, 80], fill: C.cyan }], { id: "b", pos: [160, 210], blend: "add", fx: [{ effect: "blur", radius: 18 }] }),
];

/** Grey solids of level 0, 1, 2 .. 32, each 24 px wide. */
const ramp = (): Array<ReturnType<typeof solid>> =>
  Array.from({ length: 33 }, (_, i) => {
    const hex = i.toString(16).padStart(2, "0");
    return solid(`#${hex}${hex}${hex}`, { id: `l${i}`, size: [24, 90], pos: [12 + i * 24, 45] });
  });

const mode = (light: boolean): Partial<CompOpts> => ({ bg: BLACK, color: { linear_light: light } });
const cell = (layers: unknown[], light: boolean, extra: Partial<CompOpts> = {}): Record<string, unknown> => subComp(CW, CH, DUR, layers as never, { ...mode(light), ...extra });
const strip = (light: boolean): Record<string, unknown> => subComp(33 * 24, 90, DUR, ramp(), mode(light));

const comps: Record<string, Record<string, unknown>> = {};
const make = (key: string, build: () => unknown[], extra: Partial<CompOpts> = {}): void => {
  comps[`${key}_d`] = cell(build(), false, extra);
  comps[`${key}_l`] = cell(build(), true, extra);
};
// the edge is wider than its cell, so its blur runs on the layer's own pixels
make("edge", edge, { effect_space: "layer" });
make("half", half);
make("glow", glow);
comps.ramp_d = strip(false);
comps.ramp_l = strip(true);

const film = newFilm(DUR, { color: { linear_light: true }, extra: { comps } });
const X = [96, 984];
const sets: Array<[string, string, number, string]> = [
  ["d", "display numbers (default)", X[0], C.orange],
  ["l", "color.linear_light: true", X[1], C.cyan],
];
const labels: Record<string, string[]> = {
  d: ["edge: 131", "50%: 128", "add"],
  l: ["edge: 190", "50%: 188", "add"],
};
for (const [k, head, x0, col] of sets) {
  film.add(tag(`head-${k}`, head, x0, 172, col));
  ["edge", "half", "glow"].forEach((name, i) => {
    const x = x0 + i * (CW + 20);
    film.add(place(`${name}-${k}`, `${name}_${k}`, x, 215), rect(CW, CH, { id: `bd-${name}-${k}`, pos: [x + CW / 2, 215 + CH / 2], stroke: { color: C.line, width: 2 } }));
    film.add(tag(`lab-${name}-${k}`, labels[k][i], x, 585, C.text));
  });
  film.add(place(`ramp-${k}`, `ramp_${k}`, x0, 655), tag(`ramp-lab-${k}`, "darks ramp: levels 0 to 32", x0, 785, C.dim));
}

frame(film, tl, {
  num: "02",
  title: "Colour pipeline",
  sub: "Blend, blur and add on light instead of on encoded numbers.",
  caption: "Same layers, two pipelines: 50% white reads 188 in linear light (128 as display numbers) and a blurred edge is 59 levels brighter.",
  hot: ["188", "59 levels"],
  dur: DUR,
  accent: C.cyan,
});
void T0;
export default film;
