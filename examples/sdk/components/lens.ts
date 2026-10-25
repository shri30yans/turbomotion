// lens: a camera lens whose iris opens. Run: turbomotion sdk render lens.ts --sheet lens.png --sheet-only --times 0.1,0.5,1.2
import { comp, lens, timeline } from "turbomotion";

const film = comp({ width: 960, height: 540, fps: 30, dur: 1.6, bg: "#0b0d12" });
const tl = timeline();
film.add(lens({ id: "lens", pos: [480, 270], size: 400, blades: 7, open: { tl, at: 0.2, dur: 0.8, sfx: "click" } }));
export default film;
