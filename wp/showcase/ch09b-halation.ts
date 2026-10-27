// Chapter 9b: film halation. The same night scene twice, in a linear-light film, the right panel with `film_halation`
// on: only light above the threshold rings red-orange. The grey #666666 lamp is under it and stays unchanged.

import { shape, text, timeline } from "turbomotion";
import type { Layer } from "turbomotion";
import { C, FONT, T0, border, frame, newFilm, place, subComp, tag } from "./theme";

const DUR = 5;
const PW = 840;
const PH = 620;
const tl = timeline();

/** Bright lamps, a lit sign, one dim grey lamp and a headlight sliding past. */
function scene(): Layer[] {
  const t = timeline();
  const lamps = shape(
    [
      { ellipse: [150, 150, 26, 26], fill: "#fff4e0" },
      { ellipse: [150, 300, 20, 20], fill: "#fff4e0" },
      { ellipse: [150, 430, 14, 14], fill: "#fff4e0" },
      { ellipse: [690, 150, 30, 30], fill: "#666666" },
    ],
    { id: "lamps" },
  );
  const sign = text("BAR", { id: "sign", font: FONT.display, size: 120, color: "#ffffff", pos: [PW / 2 - 40, 190], align: "center" });
  const head = shape([{ ellipse: [0, 0, 22, 22], fill: "#ffffff" }], { id: "head", pos: [250, 480] });
  t.to(head, { x: 780, duration: DUR - 0.5, ease: "none" }, 0.2);
  const road = shape([{ rect: [0, 450, PW, 170], fill: "#07080d" }], { id: "road" });
  return [road, lamps, sign, head];
}

const comps = { off: subComp(PW, PH, DUR, scene(), { bg: "#04050a" }), on: subComp(PW, PH, DUR, scene(), { bg: "#04050a" }) };
const film = newFilm(DUR, { color: { linear_light: true }, extra: { comps } });
const X = [96, 984];
film.add(
  place("p-off", "off", X[0], 190),
  place("p-on", "on", X[1], 190, { fx: [{ effect: "film_halation", threshold: 0.6, radius: 40, strength: 0.6, color: [255, 90, 40] }] }),
  border("bd-off", X[0], 190, PW, PH),
  border("bd-on", X[1], 190, PW, PH),
  tag("lab-off", "no halation", X[0], 160, C.dim),
  tag("lab-on", "film_halation: threshold 0.6, radius 40", X[1], 160, C.orange),
  tag("grey-off", "grey lamp stays dark", X[0] + 820, 190 + 300, C.dim, 28, "right"),
  tag("grey-on", "grey lamp stays dark", X[1] + 820, 190 + 300, C.dim, 28, "right"),
);
void T0;

frame(film, tl, {
  num: "9b",
  title: "Film halation",
  sub: "Bright light scatters back through the film base as a red-orange halo.",
  caption: "Computed in a linear-light film: the lamps, sign and headlight ring orange; the dim grey lamp is under the threshold and unchanged.",
  hot: ["ring orange", "unchanged"],
  dur: DUR,
  accent: C.orange,
});
export default film;
