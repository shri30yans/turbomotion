// A probe for the effect `wobble` (effects/wobble.sksl): one text layer under it, rendered as a contact sheet.
//
//   turbomotion sdk render film.ts --sheet sheet.png --times 0.5,1.5 --sheet-only
//
// Scripts and tools reach the engine through the binary named by TURBOMOTION_BIN (default: turbomotion/target/release/turbomotion).
// `fx_path` and `block_path` are folders beside this file: put more plugins in effects/ and more blocks in blocks/.
import { comp, solid, text } from "turbomotion";

const film = comp({ width: 1920, height: 1080, fps: 30, dur: 2, bg: "#1a1d29", fx_path: ["effects"], block_path: ["blocks"] });

film.add(
  // `fx` is the key for a layer's effects, a list of { effect, ...parameters }. Animate a parameter with a tween: { amount: [[0, 0], [2, 1]] }.
  text("wobble", {
    id: "probe",
    size: 240,
    weight: 800,
    color: "#ffffff",
    align: "center",
    pos: [960, 540],
    fx: [{ effect: "wobble", amount: 0.8 }],
  }),
  // solid(color, opts): the colour comes first.
  solid("#ff5a1f", { id: "rule", size: [640, 8], pos: [960, 700] }),
);

export default film;
