// Chapter 5: text bounds. A text layer's `box` is fitted to the characters showing (measured from the real shaped text),
// so a typewriter grows its own background; the padding is a keyed value and grows too. Lower-third and caption styles.

import { rect, text, timeline } from "turbomotion";
import { C, FONT, T0, border, frame, newFilm } from "./theme";

const DUR = 5.8;
const tl = timeline();
const film = newFilm(DUR);

// a stand-in picture for the lower third to sit on
film.add(
  rect(1728, 760, {
    id: "scene", pos: [960, 520],
    fill: { gradient: { type: "linear", from: [96, 140], to: [1824, 900], stops: [[0, "#1c2748"], [1, "#0d111d"]] } },
  }),
  border("scene-bd", 96, 140, 1728, 760),
);

// main lower third: typed, padding eases open from [8, 4] to [48, 24]
const typed = text("Measured, not guessed", {
  id: "typed", font: FONT.body, size: 104, color: "#ffffff", pos: [190, 650], align: "left",
  preset: { name: "typewriter", at: T0 + 0.2, each: 0.075 },
  box: { fit: "text", padding: [{ from: 8, to: 48, at: T0 + 0.2, dur: 3, ease: "power2.out" }, { from: 4, to: 24, at: T0 + 0.2, dur: 3, ease: "power2.out" }], radius: 28, fill: C.orange },
});

// a wrapped block with one box per line, typed a little later
const lines = text("Boxes follow\nevery line", {
  id: "lines", font: FONT.body, size: 72, color: "#0b0d12", pos: [1010, 400], align: "left", leading: 1.45,
  preset: { name: "typewriter", at: T0 + 1.6, each: 0.08 },
  box: { fit: "text", padding: [30, 12], radius: 14, fill: C.cyan, per_line: true },
});

// a pill label with a minimum width, so it keeps its left edge while it fills
const pill = text("min_width 520", {
  id: "pill", font: FONT.mono, size: 40, color: "#0b0d12", pos: [190, 835], align: "left",
  preset: { name: "typewriter", at: T0 + 2.6, each: 0.06 },
  box: { fit: "text", padding: [28, 14], radius: 1000, fill: C.lime, min_width: 520, align: "left" },
});
const legend = text("box: { fit: \"text\", padding, radius, per_line, min_width }", { id: "legend", font: FONT.mono, size: 28, color: C.dim, pos: [190, 210], align: "left" });
film.add(typed, lines, pill, legend);
tl.from(legend, { opacity: 0, duration: 0.5 }, T0 + 0.2);

frame(film, tl, {
  num: "05",
  title: "Text bounds",
  sub: "A background box measured from the shaped text, growing with every character.",
  caption: "The box is exactly the characters showing plus the padding, while the padding itself eases open. One box, one box per line, or a minimum width.",
  hot: ["exactly", "padding", "per line", "minimum width"],
  dur: DUR,
  accent: C.orange,
});
export default film;
