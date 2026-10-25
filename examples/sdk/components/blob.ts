// blob: seeded organic shapes that morph into each other. Run: turbomotion sdk render blob.ts --sheet blob.png --sheet-only --times 0.2,0.9,1.6,2.4
import { comp, blob, timeline } from "turbomotion";

const film = comp({ width: 960, height: 540, fps: 30, dur: 2.6, bg: "#0b0d12" });
const tl = timeline();
const a = blob({ id: "a", pos: [260, 270], size: 300, seed: 3, fill: "#3D6BFF", morph: { tl, seed: 11, at: 0.3, dur: 1, back: true } });
const b = blob({ id: "b", pos: [700, 270], size: 260, seed: 7, points: 9, wobble: 0.3, fill: { gradient: { type: "linear", from: [-100, -100], to: [100, 100], stops: [[0, "#FF5A1F"], [1, "#FF57E6"]] } }, morph: { tl, seed: 2, at: 0.6, dur: 1.2 } });
film.add(a, b);
export default film;
