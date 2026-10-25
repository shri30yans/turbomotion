// swarm: cubes burst out of a point and tumble toward the lens. Run: turbomotion sdk render swarm.ts --sheet swarm.png --sheet-only --times 0.1,0.5,1,1.8
import { comp, shot, timeline, light, swarm } from "turbomotion";

const film = comp({ width: 960, height: 540, fps: 30, bg: "#0b0d12" });
const tl = timeline();
const burst = swarm({
  tl, id: "s", n: 18, seed: 5, at: 0.1, dur: 1.2, origin: [480, 270], spread: [380, 200], z: [-500, 150], size: [40, 110],
  art: (i, s) => [{ ellipse: [0, 0, s * 0.26, s * 0.26], fill: i % 2 ? "#ffffff" : "#0b0d12" }],
});
film.timeline({ shots: [shot({ id: "swarm", dur: 2, layers: [light({ kind: "ambient" }), light({ kind: "directional", pos: [-300, -400, -600] }), burst] })] });
export default film;
