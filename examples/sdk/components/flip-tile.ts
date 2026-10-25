// flipTile: a two-sided card that turns over. Run: turbomotion sdk render flip-tile.ts --sheet flip.png --sheet-only --times 0.2,0.8,1.1,1.6,2.4
import { comp, shot, timeline, light, flipTile } from "turbomotion";

const film = comp({ width: 960, height: 540, fps: 30, bg: "#0b0d12" });
const tl = timeline();
const tiles = [0, 1, 2].map((i) => flipTile({
  tl, id: `t${i}`, size: [200, 260], pos: [220 + i * 260, 270], at: 0.6 + i * 0.25, sfx: "tick",
  front: { fill: "#1D2029", label: "?", labelColor: "#8A93A6" },
  back: { fill: ["#3D6BFF", "#FF5A1F", "#C8F031"][i], label: ["A", "B", "C"][i], labelColor: i === 2 ? "#0b0d12" : "#ffffff" },
}));
film.timeline({ shots: [shot({ id: "flip", dur: 2.6, hits: tl.hits(), layers: [light({ kind: "ambient" }), ...tiles] })] });
export default film;
