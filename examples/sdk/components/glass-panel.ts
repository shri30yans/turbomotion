// glassPanel: frosted panels over a bright backdrop. Run: turbomotion sdk render glass-panel.ts --sheet glass.png --sheet-only --times 0.5
import { comp, glassPanel, shape, text } from "turbomotion";

const film = comp({ width: 960, height: 540, fps: 30, dur: 1, bg: "#101420" });
film.add(shape([{ ellipse: [0, 0, 210, 210], fill: { gradient: { type: "radial", center: [0, 0], radius: 210, stops: [[0, "#FF5A1F"], [1, "#101420"]] } } }], { id: "orb1", pos: [300, 250] }));
film.add(shape([{ ellipse: [0, 0, 210, 210], fill: { gradient: { type: "radial", center: [0, 0], radius: 210, stops: [[0, "#3D6BFF"], [1, "#101420"]] } } }], { id: "orb2", pos: [680, 300] }));
const panel = glassPanel({ id: "card", pos: [480, 270], size: [520, 260], radius: 32, tint: "#cfe1ff" });
film.add(panel, text("glass", { id: "word", pos: [480, 290], size: 96, weight: 700, color: "#ffffff", align: "center" }));
export default film;
