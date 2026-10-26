// Writing motion paths and handles: checks what the engine would refuse, here, at the line that wrote it.

import type { Handles, MotionPath, PathKey, PathPoint, RovingKey, Side } from "./motion-types";
import type { Tween } from "./timeline";

const where = (what: string, i: number): string => `${what}, key ${i}`;

function checkSide(side: Side | undefined, name: string, at: string): void {
  if (side === undefined || typeof side === "string") return;
  if (!(side.influence >= 0 && side.influence <= 100)) throw new Error(`${at}: ${name} influence is ${side.influence}, a percent from 0 to 100`);
  const speeds = Array.isArray(side.speed) ? side.speed : [side.speed];
  if (!speeds.length || speeds.some((s) => typeof s !== "number" || !Number.isFinite(s))) throw new Error(`${at}: ${name} needs a speed in units per second`);
}

/**
 * A position that follows a curve through its keys, with After Effects' keyframe handles. Keys at a time are `{ t, v }`;
 * a key `{ roving: true, v }` has no time and keeps the speed even between the keys around it.
 * @example
 * import { comp, solid, motionPath } from "turbomotion";
 * const film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: "#10131c" });
 * film.add(solid("#ffd166", { size: [120, 120], pos: motionPath([
 *   { t: 0, v: [300, 800], ease_out: "easy" },
 *   { roving: true, v: [900, 200] },
 *   { t: 3, v: [1600, 800], ease_in: "easy" },
 * ], { auto_orient: true }) }));
 * export default film;
 */
export function motionPath(keys: Array<PathKey | RovingKey>, opts: { spatial?: "auto" | "linear" | "bezier"; auto_orient?: boolean; loop?: MotionPath["loop"] } = {}): MotionPath {
  if (!keys.length) throw new Error("motionPath(): give at least one key");
  let last = -Infinity;
  keys.forEach((k, i) => {
    const at = where("motionPath()", i);
    if (!Array.isArray(k.v) || k.v.length < 2 || k.v.length > 3) throw new Error(`${at}: "v" is [x, y] or [x, y, z]`);
    if ("roving" in k) {
      if (i === 0 || i === keys.length - 1) throw new Error(`${at}: a roving key sits between two keys with times`);
      return;
    }
    if (!(k.t >= last)) throw new Error(`${at}: times must not go back (${k.t}s after ${last}s)`);
    last = k.t;
    checkSide(k.ease_in, "ease_in", at);
    checkSide(k.ease_out, "ease_out", at);
    if (k.ease_in === "hold") throw new Error(`${at}: only a leaving side can hold; write "hold" as ease_out of the key before`);
  });
  const path: MotionPath = { keys };
  if (opts.spatial) path.spatial = opts.spatial;
  if (opts.auto_orient) path.auto_orient = "path";
  if (opts.loop) path.loop = opts.loop;
  return path;
}

/** The checked handles of a tween (`ease_out` of its first key, `ease_in` of its last). */
export function checkHandles(h: Handles, at: string): Handles {
  checkSide(h.out, "handles.out", at);
  checkSide(h.in, "handles.in", at);
  if ((h.in as unknown) === "hold") throw new Error(`${at}: handles.in cannot hold; only the side that leaves a key can`);
  return h;
}

/** `tl.to(layer, { path })` as the layer's `pos`: the first point at the tween's start, the last at its end, the points between roving. */
export function pathPos(tw: Tween, local: (t: number) => number, id: string): MotionPath {
  const { points, auto_orient } = tw.path!;
  const at = `layer "${id}": path`;
  if (points.length < 2) throw new Error(`${at} needs at least two points`);
  const key = (p: PathPoint): { v: number[] } & Record<string, unknown> => (Array.isArray(p) ? { v: p } : { ...p });
  const keys: Array<PathKey | RovingKey> = points.map((p, i) => {
    const k = key(p) as unknown as PathKey & RovingKey;
    if (i === 0) return { ...k, t: local(tw.at), ...(tw.handles?.out !== undefined ? { ease_out: tw.handles.out } : {}) };
    if (i < points.length - 1) return { ...k, roving: true as const };
    return { ...k, t: local(tw.at + tw.dur), ...(tw.handles ? (tw.handles.in !== undefined ? { ease_in: tw.handles.in } : {}) : { ease: tw.ease }) };
  });
  return motionPath(keys, { auto_orient });
}

/**
 * Moves the `path` tween of a layer into its `pos`, and returns the other tweens. A path replaces the layer's own
 * position, so a layer with a path tween takes no x/y/z tween and has no `pos` of its own to start from.
 */
export function takePath(out: Record<string, any>, tweens: Tween[], local: (t: number) => number, id: string): Tween[] {
  const paths = tweens.filter((t) => t.prop === "path");
  if (!paths.length) return tweens;
  if (paths.length > 1) throw new Error(`layer "${id}": one path tween per layer; the second was written at ${paths[1].site}`);
  const rest = tweens.filter((t) => t.prop !== "path");
  const clash = rest.find((t) => ["x", "y", "z"].includes(t.prop));
  if (clash) throw new Error(`layer "${id}": a path tween moves the layer's position, and so does the tween of "${clash.prop}" written at ${clash.site}`);
  if (out.pos !== undefined) throw new Error(`layer "${id}": a path tween sets its position; leave out "pos" (or x/y/z) on the layer`);
  out.pos = pathPos(paths[0], local, id);
  return rest;
}

/**
 * Gives the last tween written into `keys` its handles: the key it starts from leaves with `out`, the key it ends on
 * arrives with `in`. The named ease the end key carried is the one the handles replace.
 */
export function applyHandles(keys: Array<any>, h: Handles): void {
  const object = (k: any): Record<string, unknown> => (Array.isArray(k) ? { t: k[0], v: k[1], ...(k[2] !== undefined ? { ease: k[2] } : {}) } : k);
  const [start, end] = [object(keys[keys.length - 2]), object(keys[keys.length - 1])];
  delete end.ease;
  if (h.out !== undefined) start.ease_out = h.out;
  if (h.in !== undefined) end.ease_in = h.in;
  keys.splice(keys.length - 2, 2, start, end);
}
