// A five second launch sting: MOTION tumbles in letter by letter, then a tilted wall of cards pops in and the camera
// dollies toward the hero card. Build:  turbomotion sdk build examples/sdk/launch/film.ts
import { comp, shot, timeline, text, rect, group, camera, light, letters3d, grid, rand } from "turbomotion";

const film = comp({ width: 1920, height: 1080, fps: 30, bg: "#0b0d12" });
const FONT = "Avenir Next";

// ---- shot 1: the word -----------------------------------------------------------------------------------------
const letters = letters3d("MOTION", {
  pos: [960, 540, 0],
  font: FONT,
  weight: 800,
  size: 240,
  depth: 70,
  bevel: 6,
  color: "#f4f1ea",
  side: "#b8442c",
  tracking: 8,
  material: { specular: 0.7, shininess: 40 },
});

const intro = timeline();
intro.from(letters, { rotation: [-100, 60, null], opacity: 0, z: -400, duration: 1.0, ease: "back.out(1.4)", stagger: 0.07 }, 0.2);

const hook = shot({
  id: "hook",
  dur: 2.6,
  bg: "#0b0d12",
  camera: { move: "orbit", deg: 24 },
  layers: [
    light({ kind: "ambient", color: "#7f8cb0", intensity: 0.55 }),
    light({ kind: "point", pos: [500, 160, -900], color: "#ffd9a0", intensity: 1.4, radius: 2600 }),
    letters,
  ],
});

// ---- shot 2: the wall -----------------------------------------------------------------------------------------
const paint = rand(11);
const palette = ["#ff6b3d", "#3dd6ff", "#b7ff6b", "#ffd23d", "#c77dff", "#ff4d8d"];
const words = ["GO", "NEW", "4K", "3D", "FAST", "LIVE", "RUST", "FX"];
const wall = group({ id: "wall", three_d: true, pos: [960, 500, 0], rotation: [26.3922, -18.0578, 8.7447] });

const HERO = 20; // column 4, row 2: the card the camera dollies toward
const tiles = grid(8, 5, [236, 164]);
// put the wall's origin on the hero card, so aiming the camera at the wall aims it at the hero
const cells = tiles.map((c) => ({ ...c, x: c.x - tiles[HERO].x, y: c.y - tiles[HERO].y }));
const cards = cells.map((c) =>
  rect(c.i === HERO ? 300 : 208, c.i === HERO ? 190 : 136, {
    id: `card${c.i}`,
    fill: c.i === HERO ? "#ffffff" : paint.pick(palette),
    radius: 18,
    three_d: true,
    pos: [c.x, c.y, c.i === HERO ? -140 : 0],
    parent: wall,
    material: { specular: 0.8, shininess: 40 },
    extra: c.i === HERO ? { lit: false } : undefined,
  }),
);
const labelled = cells.filter((c) => c.i === HERO || paint() < 0.35);
const labels = labelled.map((c) =>
  text(c.i === HERO ? "LAUNCH" : paint.pick(words), {
    id: `label${c.i}`,
    font: FONT,
    weight: 800,
    size: c.i === HERO ? 54 : 46,
    color: "#0b0d12",
    align: "center",
    three_d: true,
    pos: [c.x, c.y, c.i === HERO ? -146 : -6],
    parent: wall,
  }),
);

const cam = camera({ id: "cam", pos: [960, 540, -2700], poi: [960, 540, -100], zoom: 2000 });
const pop = timeline();
pop.from(cards, { scale: 0, opacity: 0, rotation: [null, 70, null], duration: 0.55, ease: "back.out(1.7)", stagger: { each: 0.03, from: "center" } }, 0.1);
pop.from(labels, { opacity: 0, duration: 0.3, stagger: { each: 0.03, from: "center" } }, 0.6);
pop.to(cam, { z: -1450, duration: 2.7, ease: "sine.inOut" }, 0.2);

const wallShot = shot({
  id: "wall",
  dur: 3.0,
  in: { type: "zoom_through", dur: 0.6 },
  bg: "#0b0d12",
  layers: [
    light({ kind: "ambient", color: "#8a96b8", intensity: 0.6 }),
    light({ kind: "point", pos: [300, -200, -1200], color: "#ffe2b0", intensity: 1.2, radius: 4000 }),
    cam,
    wall,
    cards,
    labels,
  ],
});

film.timeline({ shots: [hook, wallShot] });
export default film;
