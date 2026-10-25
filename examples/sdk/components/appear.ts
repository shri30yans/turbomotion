// appear: an entrance for a whole component, on any component that takes `appear`. Run: turbomotion sdk render appear.ts --sheet appear.png --sheet-only --times 0.3,0.7,1.2,2
import { comp, statCounter, barRace, timeline } from "turbomotion";

const film = comp({ width: 960, height: 540, fps: 30, dur: 2.4, bg: "#0b0d12" });
const theme = { mono: "Menlo", monoBold: "Menlo", text: "Avenir Next", display: "Avenir Next Condensed" }; // installed system faces
const tl = timeline();
const stat = statCounter({ tl, value: 373, suffix: " fps", label: "frames per second", at: 0.4, pos: [480, 150], size: 120, theme, appear: { preset: "slide_up", dur: 0.5 } });
const race = barRace({ tl, at: 0.9, dur: 1.2, pos: [120, 300], w: 700, rowH: 70, barH: 30, labelW: 200, bars: [{ label: "ours", value: 373, color: "#FF5A1F" }, { label: "theirs", value: 60, color: "#3D6BFF" }], theme, appear: { preset: "fade", stagger: 0.12 } });
film.add(stat, race);
export default film;
