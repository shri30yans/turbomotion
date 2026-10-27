// Chapter 4: property model v2. Four panels: a curved bezier path with auto-orient, roving keys against timed keys on
// the same path, speed/influence handles against a plain ease, and a hold key. Every position is written as keys
// (`motionPath`), so the engine, not this script, finds each frame.

import { motionPath, rect, shape, timeline } from "turbomotion";
import type { PathKey, RovingKey, Vec2 } from "turbomotion";
import { C, border, frame, newFilm, tag } from "./theme";

const DUR = 6.5;
const GO = 1.8; // seconds when the travellers set off
const RUN = 3.8; // and how long they take
const tl = timeline();
const film = newFilm(DUR);

const X = [96, 984];
const Y = [140, 520];
const PW = 840;
const PH = 340;
const panel = (id: string, x: number, y: number, label: string, color: string): void => {
  film.add(rect(PW, PH, { id: `bg-${id}`, fill: C.panel, pos: [x + PW / 2, y + PH / 2] }), border(`bd-${id}`, x, y, PW, PH));
  film.add(tag(`lab-${id}`, label, x + 20, y + 30, color));
};

/** A smooth curve through the points, with explicit bezier handles (a sixth of the way between the neighbours) so the preview matches. */
type Pt = Vec2;
const handles = (pts: Pt[]): Array<{ in: Vec2; out: Vec2 }> =>
  pts.map((p, i) => {
    const a = pts[Math.max(i - 1, 0)];
    const b = pts[Math.min(i + 1, pts.length - 1)];
    const d: Vec2 = [(b[0] - a[0]) / 6, (b[1] - a[1]) / 6];
    return { in: i === 0 ? [0, 0] : [-d[0], -d[1]], out: i === pts.length - 1 ? [0, 0] : d };
  });
/** The same curve as an SVG path, for the faint preview line. */
const pathD = (pts: Pt[]): string => {
  const h = handles(pts);
  let d = `M ${pts[0][0]} ${pts[0][1]}`;
  for (let i = 1; i < pts.length; i++) {
    const [a, b] = [pts[i - 1], pts[i]];
    d += ` C ${a[0] + h[i - 1].out[0]} ${a[1] + h[i - 1].out[1]} ${b[0] + h[i].in[0]} ${b[1] + h[i].in[1]} ${b[0]} ${b[1]}`;
  }
  return d;
};
const preview = (id: string, pts: Pt[], color: string = "#4a5370"): ReturnType<typeof shape> =>
  shape([{ d: pathD(pts), stroke: { color, width: 4, cap: "round", dash: [2, 14] } }], { id });

const dot = (r: number, fill: string): { ellipse: [number, number, number, number]; fill: string } => ({ ellipse: [0, 0, r, r], fill });

// A: bezier path with auto-orient -------------------------------------------------------------------------------
panel("a", X[0], Y[0], "bezier path + auto_orient", C.orange);
const wave: Pt[] = [[160, 420], [330, 250], [520, 400], [700, 250], [880, 330]];
const hw = handles(wave);
const arrowKeys: PathKey[] = wave.map((v, i) => ({
  t: GO + (RUN * i) / (wave.length - 1), v, spatial: "bezier", in: hw[i].in, out: hw[i].out, ease: "none",
}));
film.add(
  preview("a-path", wave),
  shape([{ points: [[-34, -18], [34, 0], [-34, 18], [-16, 0]], closed: true, fill: C.orange }], { id: "arrow", pos: motionPath(arrowKeys, { auto_orient: true }) }),
);

// B: roving keys against timed keys on one path --------------------------------------------------------------------
panel("b", X[1], Y[0], "roving keys: even speed", C.cyan);
const way: Pt[] = [[1060, 440], [1240, 300], [1500, 440], [1730, 300]];
const hb = handles(way);
const spot = (v: Pt, i: number): PathKey => ({ t: 0, v, spatial: "bezier", in: hb[i].in, out: hb[i].out, ease: "none" });
const roving: Array<PathKey | RovingKey> = way.map((v, i) =>
  i === 0 ? { ...spot(v, i), t: GO } : i === way.length - 1 ? { ...spot(v, i), t: GO + RUN } : { roving: true, v, spatial: "bezier", in: hb[i].in, out: hb[i].out },
);
// the same waypoints with times chosen by hand: quick, slow, quick
const times = [GO, GO + 0.5, GO + RUN - 0.5, GO + RUN];
const timed: PathKey[] = way.map((v, i) => ({ ...spot(v, i), t: times[i] }));
film.add(
  preview("b-path", way),
  shape(way.map((p) => ({ ellipse: [p[0], p[1], 7, 7] as [number, number, number, number], fill: C.dim })), { id: "b-way" }),
  shape([dot(16, C.cyan)], { id: "rover", pos: motionPath(roving) }),
  shape([dot(16, C.orange)], { id: "timed", pos: motionPath(timed) }),
  tag("b-key1", "cyan: roving", X[1] + 20, Y[0] + 70, C.cyan),
  tag("b-key2", "orange: same points, timed keys", X[1] + 20, Y[0] + 102, C.orange),
);

// C: speed / influence handles against a plain ease ---------------------------------------------------------------------
panel("c", X[0], Y[1], "speed + influence vs plain ease", C.lime);
const x0 = 170;
const x1 = 860;
const lane = (id: string, y: number, color: string, a: PathKey, b: PathKey): ReturnType<typeof shape> =>
  shape([{ rect: [-17, -17, 34, 34], radius: 6, fill: color }], { id, pos: motionPath([{ ...a, t: GO, v: [x0, y] }, { ...b, t: GO + RUN, v: [x1, y] }]) });
film.add(
  rect(x1 - x0, 2, { id: "c-rail1", fill: C.line, pos: [(x0 + x1) / 2, Y[1] + 130] }),
  rect(x1 - x0, 2, { id: "c-rail2", fill: C.line, pos: [(x0 + x1) / 2, Y[1] + 230] }),
  lane("plain", Y[1] + 130, C.orange, { t: 0, v: [0, 0], ease: "sine.inOut" }, { t: 0, v: [0, 0] }),
  lane("handled", Y[1] + 230, C.lime, { t: 0, v: [0, 0], ease_out: { speed: 0, influence: 92 } }, { t: 0, v: [0, 0], ease_in: { speed: 0, influence: 92 } }),
  tag("c-l1", "ease: sine.inOut", X[0] + 20, Y[1] + 90, C.orange),
  tag("c-l2", "ease_out / ease_in: speed 0, influence 92%", X[0] + 20, Y[1] + 190, C.lime),
);

// D: hold keys -------------------------------------------------------------------------------------------------------------
panel("d", X[1], Y[1], "hold key: a step at each key", C.pink);
const stops = [1080, 1300, 1520, 1740];
const stepT = [GO, GO + 1.1, GO + 2.2, GO + 3.3];
const holdKeys: PathKey[] = stops.map((x, i) => ({ t: stepT[i], v: [x, Y[1] + 190] as Vec2, ...(i < stops.length - 1 ? { ease_out: "hold" as const } : {}) }));
film.add(
  shape(stops.map((x) => ({ rect: [x - 3, Y[1] + 120, 6, 140] as [number, number, number, number], fill: C.line })), { id: "d-ticks" }),
  shape([{ rect: [-30, -30, 60, 60], radius: 8, fill: C.pink }], { id: "stepper", pos: motionPath(holdKeys) }),
  tag("d-l1", "ease_out: hold, one step per second", X[1] + 20, Y[1] + 80, C.pink),
);

frame(film, tl, {
  num: "04",
  title: "Property model v2",
  sub: "Keys with bezier handles, roving keys, speed and influence, hold.",
  caption: "Curved paths bank with the layer; roving keys keep one speed, timed keys do not; handles reshape the ease; hold steps.",
  hot: ["bank", "one speed", "handles", "hold"],
  dur: DUR,
  accent: C.lime,
});
export default film;
