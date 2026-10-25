// montage: a run of word shots whose start and overlap maths is done for you. Run: turbomotion sdk render montage.ts --sheet montage.png --sheet-only --times 0.4,0.8,1.1,1.8
import { comp, montage } from "turbomotion";

const film = comp({ width: 960, height: 540, fps: 30, bg: "#0b0d12" });
const m = montage({
  frame: [960, 540], size: 150, hold: 0.55, transition: { type: "dissolve", dur: 0.2 },
  items: [{ word: "fast", field: "#FF5A1F" }, { word: "small", field: "#3D6BFF", sfx: "tick" }, { word: "yours", field: "#1D2029" }],
});
film.timeline({ shots: m.shots });
export default film;
