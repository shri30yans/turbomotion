// dollyZoom: the vertigo shot. The middle card keeps its size while the depth stretches. Run: turbomotion sdk render dolly-zoom.ts --sheet dolly.png --sheet-only --times 0.1,1,2
import { comp, shot, timeline, camera, light, box, dollyZoom } from "turbomotion";

const film = comp({ width: 960, height: 540, fps: 30, bg: "#0b0d12" });
const tl = timeline();
const cam = camera({ pos: [480, 270, -800], poi: [480, 270, 0], zoom: 800 });
dollyZoom(cam, { tl, at: 0.2, dur: 1.8, amount: 0.45 });
const cubes = [-250, -120, 0, 400, 900].map((z, i) => box({ size: 90 }, { color: ["#3D6BFF", "#FF5A1F", "#C8F031", "#8F6CFF", "#FF57E6"][i], pos: [480 + (i - 2) * 150, 270, z], rotation: [-16.4703, 24.0929, 6.8817] }));
film.timeline({ shots: [shot({ id: "vertigo", dur: 2.2, layers: [cam, light({ kind: "ambient" }), light({ kind: "directional", pos: [-300, -400, -600] }), ...cubes] })] });
export default film;
