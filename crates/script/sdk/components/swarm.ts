// A swarm of cubes that burst out of a point and tumble toward the lens, or drift in calmly.

import type { Layer } from "../layers";
import { rand } from "../motion";
import type { Timeline } from "../timeline";
import type { Color } from "../types";
import { cube, CUBE_COLORS } from "./cube";
import type { CubePart, Items } from "./cube";
import type { Part } from "./util";

export interface SwarmOpts {
  tl: Timeline;
  id: string;
  n: number;
  seed: number;
  /** Seconds at which the first cube starts to move, and how long each travels. */
  at: number;
  dur: number;
  origin?: [number, number];
  /** Half-extent of the scatter around `origin` (px). */
  spread?: [number, number];
  /** Depth range of the scatter (negative is toward the lens). */
  z?: [number, number];
  /** Cube side range (px). */
  size?: [number, number];
  colors?: Color[];
  /** Art on a cube's first two faces: `(index, side, random)` returns shape items. */
  art?: (index: number, size: number, r: ReturnType<typeof rand>) => Items;
  /** Seconds between cubes. */
  stagger?: number;
  /** Degrees of tumble over the flight (±). */
  spin?: number;
  ease?: string;
  /** A sound on every third cube. */
  sfx?: string;
  /** Start scale (0 grows from a point). */
  from?: number;
  /** Fly in from the scatter to the origin instead of out of it. */
  reverse?: boolean;
}

export interface SwarmPart extends Part {
  roots: Layer[];
  cubes: CubePart[];
}

/** `swarm({ tl, id: "a", n: 9, seed: 3, at: 1.1, dur: 0.8, art: (i, s) => icon(names[i % 4], s * 0.7) })`. */
export function swarm(o: SwarmOpts): SwarmPart {
  const r = rand(o.seed);
  const [ox, oy] = o.origin ?? [960, 540];
  const [sx, sy] = o.spread ?? [320, 240];
  const [z0, z1] = o.z ?? [-900, 200];
  const [s0, s1] = o.size ?? [60, 180];
  const colors = o.colors ?? CUBE_COLORS;
  const cubes: CubePart[] = [];
  for (let i = 0; i < o.n; i++) {
    const size = r.range(s0, s1);
    const fill = colors[Math.floor(r() * colors.length)];
    const art = o.art?.(i, size, r);
    const c = cube({ id: `${o.id}${i}`, size, faces: [{ fill, items: art }, { fill, items: art }, fill], pos: [ox, oy, 0], rot: [r.range(-60, 60), r.range(-90, 90), r.range(-30, 30)] });
    const to = { x: ox + r.range(-sx, sx), y: oy + r.range(-sy, sy), z: r.range(z0, z1), scale: 1 };
    const t0 = o.at + i * (o.stagger ?? 0.012);
    const home = { x: ox, y: oy, z: 0, scale: o.from ?? 0.15 };
    const [a, b] = o.reverse ? [to, home] : [home, to];
    o.tl.fromTo(c.root, a, { ...b, duration: o.dur, ease: o.ease ?? (o.reverse ? "power3.in" : "expo.out") }, t0);
    const spin = o.spin ?? 200;
    o.tl.to(c.root, { rotation: [r.range(-spin, spin), r.range(-spin, spin), null], duration: o.dur + 1, ease: "none" }, t0);
    if (o.sfx && i % 3 === 0) o.tl.hit(t0, o.sfx, 0.35);
    cubes.push(c);
  }
  return { layers: cubes.flatMap((c) => c.layers), roots: cubes.map((c) => c.root), cubes, end: o.at + (o.n - 1) * (o.stagger ?? 0.012) + o.dur };
}
