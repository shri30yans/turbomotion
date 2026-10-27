// Chapter 3: all 37 blend modes on one standard backdrop, over two pages. Each cell is its own precomp (a blend mode
// needs something to blend with): a gradient ground with a grey bar, then a disc and a square at 85% in the mode.

import { shape, text, timeline } from "turbomotion";
import type { BlendMode, Layer } from "turbomotion";
import { C, T0, FONT, border, frame, newFilm, place, subComp, tag } from "./theme";

const DUR = 6.5;
const CW = 320;
const CH = 140;
const tl = timeline();

/** The engine's blend modes, in the order of docs/reference/blend-modes.md. */
const MODES: BlendMode[] = [
  "normal", "dissolve", "dancing_dissolve", "multiply", "darken", "color_burn", "darker_color", "linear_burn", "classic_color_burn", "add",
  "screen", "lighten", "color_dodge", "lighter_color", "classic_color_dodge", "overlay", "soft_light", "hard_light", "linear_light", "vivid_light",
  "pin_light", "hard_mix", "difference", "exclusion", "classic_difference", "subtract", "divide", "hue", "saturation", "color",
  "luminosity", "stencil_alpha", "stencil_luma", "silhouette_alpha", "silhouette_luma", "alpha_add", "luminescent_premul",
];

/** The 17 modes the engine had before this programme; the other 20 are new and their labels are violet. */
const ORIGINAL = new Set<string>(["normal", "add", "screen", "multiply", "overlay", "soft_light", "hard_light", "darken", "lighten", "color_dodge", "color_burn", "difference", "exclusion", "hue", "saturation", "color", "luminosity"]);

/** The ground, then the source in `mode`. */
function cell(mode: BlendMode): Record<string, unknown> {
  const ground = shape([{ rect: [0, 0, CW, CH], fill: { gradient: { type: "linear", from: [0, 0], to: [CW, 0], stops: [[0, "#1d2b64"], [1, "#f8cdda"]] } } }], { id: "ground" });
  const bar = shape([{ rect: [0, CH - 45, CW, 45], fill: "#808080" }], { id: "bar" });
  const source = shape(
    [{ ellipse: [-40, 0, 65, 65], fill: "#e8a33d" }, { rect: [-5, -42, 84, 84], fill: "#3ddc97" }],
    { id: "source", pos: [CW / 2, CH / 2], blend: mode, opacity: 0.85 },
  );
  return subComp(CW, CH, DUR, [ground, bar, source], { bg: C.bg });
}

const comps: Record<string, Record<string, unknown>> = {};
MODES.forEach((m) => (comps[`m_${m}`] = cell(m)));

const film = newFilm(DUR, { extra: { comps } });
const PAGE = 20;
const PITCH_X = CW + 32;
const PITCH_Y = 184;
const pages: Layer[][] = [[], []];
MODES.forEach((mode, i) => {
  const page = Math.floor(i / PAGE);
  const k = i % PAGE;
  const x = 96 + (k % 5) * PITCH_X;
  const y = 132 + Math.floor(k / 5) * PITCH_Y;
  pages[page].push(
    place(`cell-${mode}`, `m_${mode}`, x, y),
    border(`bd-${mode}`, x, y, CW, CH),
    tag(`lab-${mode}`, mode, x, y + CH + 22, ORIGINAL.has(mode) ? C.text : C.violet),
  );
});
for (const l of pages.flat()) film.add(l);

// page 1 fades out as page 2 comes in
const FLIP = 3.9;
pages[0].forEach((l) => tl.to(l, { opacity: 0, duration: 0.3 }, FLIP - 0.3));
pages[1].forEach((l) => tl.from(l, { opacity: 0, duration: 0.3 }, FLIP));
const count = text("modes 1 to 20 of 37", { id: "count", font: FONT.mono, size: 28, color: C.violet, pos: [1824, 84], align: "right" });
const count2 = text("modes 21 to 37 of 37", { id: "count2", font: FONT.mono, size: 28, color: C.violet, pos: [1824, 84], align: "right", opacity: 0 });
tl.to(count, { opacity: 0, duration: 0.2 }, FLIP - 0.3);
tl.to(count2, { opacity: 1, duration: 0.2 }, FLIP);
film.add(count, count2);

frame(film, tl, {
  num: "03",
  title: "37 blend modes",
  sub: "Every mode of the table, on one backdrop, from normal to dissolve to luminescent premul.",
  caption: "Violet labels are the 20 new modes. Look at dissolve (a dither of the layer's own opacity) and at the stencil and silhouette cut-outs.",
  hot: ["20 new modes", "dissolve", "stencil", "silhouette"],
  dur: DUR,
  accent: C.violet,
});
void T0;
export default film;
