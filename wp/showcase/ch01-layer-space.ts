// Chapter 1: layer-space effects. The same two scenes run in two panels: `effect_space: "screen"` (the default, left)
// and `"layer"` (right). Each panel is a precomp, and a precomp may name its own effect space.

import { solid, timeline } from "turbomotion";
import { C, T0, border, frame, newFilm, place, subComp, tag } from "./theme";

const DUR = 6;
const PW = 840;
const PH = 300;
const tl = timeline();

/** Two blurred squares, the right one growing from scale 1 to 2; or a lamp sliding in from outside the panel. */
function scene(kind: "scale" | "offscreen", space: "screen" | "layer"): Record<string, unknown> {
  const t = timeline();
  if (kind === "scale") {
    const sq1 = solid(C.cyan, { id: "sq1", size: [100, 100], pos: [250, 150], fx: [{ effect: "blur", radius: 14 }] });
    const sq2 = solid(C.cyan, { id: "sq2", size: [100, 100], pos: [610, 150], scale: 2, fx: [{ effect: "blur", radius: 14 }] });
    t.from(sq2, { scale: 1, duration: 2.2, ease: "power2.inOut" }, T0 + 0.3);
    return subComp(PW, PH, DUR, [sq1, sq2], { effect_space: space });
  }
  const lamp = solid("#ffffff", { id: "lamp", size: [160, 160], pos: [-90, 150], fx: [{ effect: "glow", radius: 40, strength: 2 }] });
  t.from(lamp, { x: -330, duration: 2.4, ease: "power2.inOut" }, T0 + 0.3);
  return subComp(PW, PH, DUR, [lamp], { effect_space: space });
}

const comps = {
  scale_screen: scene("scale", "screen"),
  scale_layer: scene("scale", "layer"),
  off_screen: scene("offscreen", "screen"),
  off_layer: scene("offscreen", "layer"),
};

const film = newFilm(DUR, { extra: { comps } });
const X = [96, 984];
const Y = [215, 565];
film.add(
  place("p-scale-screen", "scale_screen", X[0], Y[0]),
  place("p-scale-layer", "scale_layer", X[1], Y[0]),
  place("p-off-screen", "off_screen", X[0], Y[1]),
  place("p-off-layer", "off_layer", X[1], Y[1]),
  border("b1", X[0], Y[0], PW, PH),
  border("b2", X[1], Y[0], PW, PH),
  border("b3", X[0], Y[1], PW, PH),
  border("b4", X[1], Y[1], PW, PH),
  tag("col-screen", 'effect_space: "screen"  (default)', X[0], 172, C.orange),
  tag("col-layer", 'effect_space: "layer"', X[1], 172, C.cyan),
  tag("row-scale", "scale 1 | scale 2", X[0] + 16, Y[0] + 26, C.dim, 28),
  tag("row-scale2", "scale 1 | scale 2", X[1] + 16, Y[0] + 26, C.dim, 28),
  tag("row-off", "lamp starts outside the panel", X[0] + 16, Y[1] + 26, C.dim, 28),
  tag("row-off2", "lamp starts outside the panel", X[1] + 16, Y[1] + 26, C.dim, 28),
);

frame(film, tl, {
  num: "01",
  title: "Layer-space effects",
  sub: "Effects run on the layer's own pixels, then the layer is placed.",
  caption: "Layer space: the blur doubles with the layer's scale, and light from outside the frame bleeds in. Screen space: neither.",
  hot: ["doubles", "bleeds in"],
  dur: DUR,
});
export default film;
