// svg, pixelSprite and shape-item tweens. Run: turbomotion sdk render artwork.ts --sheet artwork.png --sheet-only --times 0.2,0.8,1.6
import { comp, svg, pixelSprite, shape, timeline } from "turbomotion";

const film = comp({ width: 960, height: 540, fps: 30, dur: 2, bg: "#0b0d12" });
const tl = timeline();
const bolt = svg("M13 2 L4 14 H11 L9 22 L20 9 H13 Z", { id: "bolt", pos: [200, 270], width: 180, fill: "#C8F031" });
tl.from(bolt, { scale: 0, rotation: -40, duration: 0.7, ease: "back.out(2)" }, 0.1);
const mark = svg('<svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" fill="none" stroke="#3D6BFF" stroke-width="1"/><path d="M3 5 L5 7 L8 3" fill="none" stroke="#ffffff" stroke-width="1"/></svg>', { id: "mark", pos: [480, 270], width: 180 });
const ring = shape([{ ellipse: [0, 0, 70, 70], stroke: { color: "#FF5A1F", width: 8, cap: "round" }, trim: { end: 0 } }], { id: "ring", pos: [480, 270] });
tl.to(ring, { items: { 0: { trim_end: 1, stroke_width: "+=8" } }, duration: 1, ease: "power2.inOut" }, 0.4);
const sprite = pixelSprite([
  "..aaaa..", ".aaaaaa.", "aabaabaa", "aaaaaaaa", "a.aaaa.a", "a.a..a.a",
], { a: "#FF5A1F", b: "#0b0d12" }, { id: "inv", px: 14, pos: [760, 270] });
tl.to(sprite, { pos: [760, "+=40"], duration: 1.4, ease: "sine.inOut" }, 0.2);
film.add(bolt, mark, ring, sprite);
export default film;
