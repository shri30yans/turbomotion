// Chapter 9a: Add Grain. One dusk scene four times: clean, then the three film presets. The grey band at the bottom of
// each panel is flat #808080 so the grain's strength can be read off it.

import { shape, solid, timeline } from "turbomotion";
import type { Layer } from "turbomotion";
import { C, T0, border, frame, newFilm, place, subComp, tag } from "./theme";

const DUR = 5;
const PW = 408;
const PH = 600;
const tl = timeline();

/** Sky, a rising sun, two hills and the flat grey band. */
function scene(): Layer[] {
  const t = timeline();
  const sky = shape([{ rect: [0, 0, PW, PH], fill: { gradient: { type: "linear", from: [0, 0], to: [0, PH], stops: [[0, "#26325c"], [0.55, "#e08a5a"], [1, "#f5c27a"]] } } }], { id: "sky" });
  const sun = shape([{ ellipse: [0, 0, 62, 62], fill: "#fff1c9" }], { id: "sun", pos: [PW / 2, 350] });
  t.from(sun, { y: 430, duration: 4, ease: "sine.out" }, 0);
  const hills = shape(
    [
      { d: `M 0 470 C 90 410 170 430 250 460 S 380 440 ${PW} 430 L ${PW} ${PH} L 0 ${PH} Z`, fill: "#1a1f33" },
      { d: `M 0 510 C 100 480 220 500 ${PW} 485 L ${PW} ${PH} L 0 ${PH} Z`, fill: "#0e1220" },
    ],
    { id: "hills" },
  );
  const band = solid("#808080", { id: "band", size: [PW, 70], pos: [PW / 2, PH - 35] });
  return [sky, sun, hills, band];
}

/** Name, preset, and the grain's standard deviation read off the flat grey band in the render (levels of 255). */
const presets: Array<[string, string | null, string]> = [
  ["clean", null, "no grain"],
  ["fine_35mm", "fine_35mm", "std 6.2 levels"],
  ["coarse_16mm", "coarse_16mm", "std 11.0 levels"],
  ["vintage_8mm", "vintage_8mm", "std 17.9 levels"],
];
const comps: Record<string, Record<string, unknown>> = {};
presets.forEach(([name]) => (comps[name] = subComp(PW, PH, DUR, scene())));
const film = newFilm(DUR, { extra: { comps } });

presets.forEach(([name, preset, std], i) => {
  const x = 96 + i * (PW + 32);
  film.add(
    place(`p-${name}`, name, x, 190, preset ? { fx: [{ effect: "add_grain", preset, seed: 3 }] } : {}),
    border(`bd-${name}`, x, 190, PW, PH),
    tag(`lab-${name}`, name, x, 160, preset ? C.pink : C.dim),
    tag(`std-${name}`, std, x, 830, C.text),
  );
});
void T0;

frame(film, tl, {
  num: "9a",
  title: "Add Grain",
  sub: "Seeded film grain with presets for size, softness and colour.",
  caption: "Same picture, same seed, three presets. Grain gets coarser, stronger and more colourful from 35 mm to 16 mm to 8 mm.",
  hot: ["coarser", "stronger", "more colourful"],
  dur: DUR,
  accent: C.pink,
});
export default film;
