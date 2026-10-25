// hand and capsule: our own vector hand, and the pill it is made of. Run: turbomotion sdk render hands.ts --sheet hands.png --sheet-only --times 0.2,1,1.9
import { comp, hand, capsule, timeline } from "turbomotion";

const film = comp({ width: 960, height: 540, fps: 30, dur: 2, bg: "#0b0d12" });
const tl = timeline();
const left = hand({ id: "left", pos: [150, 330], rotation: -25, fill: "#F3F0E8", rim: "#3D6BFF", glow: [61, 107, 255] });
const right = hand({ id: "right", pos: [810, 330], rotation: 205, flip: true, fill: "#F3F0E8", rim: "#FF5A1F", glow: [255, 90, 31] });
tl.to(left, { x: "+=150", duration: 1.6, ease: "expo.out" }, 0.1).to(right, { x: "-=150", duration: 1.6, ease: "expo.out" }, 0.1);
const bar = capsule({ from: [380, 120], to: [580, 120], w: 28, fill: "#C8F031" });
const tilt = capsule({ pos: [420, 440], len: 120, deg: -30, w: 36, fill: "#8F6CFF", stroke: { color: "#ffffff", width: 3 } });
film.add(left, right, bar, tilt);
export default film;
