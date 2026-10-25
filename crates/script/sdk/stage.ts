// Smart animate. Describe where each named layer sits in a few poses, then glide between poses: the layers are matched
// by name, only the properties that differ are tweened, and every layer lands on the same clock. This is what makes a
// scene feel like one object changing rather than one picture replacing another.
//
//   const s = stage({ title, clock }, {
//     start: { title: { x: 360, y: 300, opacity: 0 }, clock: { x: 360, y: 800, scale: 0.6 } },
//     shown: { title: { opacity: 1 }, clock: { y: 640, scale: 1 } },
//   }).start("start");
//   s.go(tl, "shown", { at: 0.4, dur: 0.7, ease: "expo.out", stagger: 0.06 });
//
// A layer a pose does not mention keeps its place. To make one disappear, give it `opacity: 0`. A group's opacity
// reaches everything parented to it (the engine's null layers do not pass opacity down, so the stage does).

import type { Layer } from "./layers";
import type { Timeline } from "./timeline";
import type { Ease, Seconds } from "./types";

/** The properties a pose can set; the same names `tl.to` animates. */
export interface Pose {
  x?: number;
  y?: number;
  z?: number;
  scale?: number;
  /** After Effects' X, Y and Z Rotation: one number is the Z Rotation, three turn it about each axis. */
  rotation?: number | [number, number, number];
  opacity?: number;
  size?: number;
  color?: string;
  /** After Effects' X, Y and Z Orientation: the layer's own pose. */
  orientation?: [number, number, number];
  /** The point the layer's face turns toward, in the parent's space. */
  aim?: [number, number, number];
}
type PoseProp = keyof Pose;
const POSE_PROPS: PoseProp[] = ["x", "y", "z", "scale", "rotation", "opacity", "size", "color", "orientation", "aim"];
/** What an unset property means, so a pose can leave out anything that is at rest. */
const REST: Partial<Pose> = { scale: 1, rotation: 0, opacity: 1 };

/** A pose's rotation as the three axes the engine keeps: a lone number is the Z Rotation. */
function axes(rotation: Pose["rotation"]): [number, number, number] | undefined {
  if (rotation === undefined) return undefined;
  return typeof rotation === "number" ? [0, 0, rotation] : rotation;
}

/** Is the pose's value for `key` different from where the layer is now? */
function differs(key: PoseProp, target: Pose, from: Pose): boolean {
  const want = target[key];
  if (want === undefined) return false;
  if (key === "rotation") {
    const to = axes(want);
    const have = axes(from.rotation) ?? [0, 0, 0];
    if (!to) return false;
    return to.some((v, i) => v !== have[i]);
  }
  const have = from[key];
  if (Array.isArray(want)) return want.some((v, i) => v !== (Array.isArray(have) ? have[i] : undefined));
  return (want ?? REST[key]) !== (have ?? REST[key]);
}

export interface GoOptions {
  /** Seconds, in the timeline's clock, at which the glide starts. */
  at: Seconds;
  dur?: Seconds;
  /** One ease for everything, or one per property: `{ x: "expo.out", opacity: "power2.out" }` (others use `default`). */
  ease?: Ease | (Partial<Record<PoseProp, Ease>> & { default?: Ease });
  /** Seconds between the starts of consecutive layers, in the order the pose lists them (or `order`). */
  stagger?: Seconds;
  /** Layer names in the order they should start, for a stagger; names not listed follow in pose order. */
  order?: string[];
}

/** Named poses for layers; `go` tweens between them. */
export class Stage {
  private now = new Map<string, Pose>();

  constructor(private readonly layers: Record<string, Layer>, private readonly poses: Record<string, Record<string, Pose>>) {
    for (const [pose, members] of Object.entries(poses)) {
      for (const name of Object.keys(members)) {
        if (!layers[name]) throw new Error(`stage: pose "${pose}" names a layer "${name}" that was not given; layers are ${Object.keys(layers).join(", ")}`);
      }
    }
  }

  /** Put every layer the pose mentions where the pose says, with no motion. Call once, before any `go`. */
  start(pose: string): this {
    for (const [name, p] of Object.entries(this.pose(pose))) {
      const layer = this.layers[name];
      const props: Record<string, unknown> = {};
      if (p.x !== undefined || p.y !== undefined || p.z !== undefined) {
        const pos = [p.x ?? 0, p.y ?? 0];
        if (p.z !== undefined) pos.push(p.z);
        props.pos = pos;
      }
      for (const k of ["scale", "rotation", "opacity", "size", "color", "orientation", "aim"] as const) if (p[k] !== undefined) props[k] = p[k];
      const { opacity, ...own } = props;
      layer.set(own);
      if (opacity !== undefined) for (const l of fadeTargets(layer)) l.set({ opacity });
      this.now.set(name, { ...p });
    }
    return this;
  }

  /** Glide from where the layers are now to `pose`. Returns the time (timeline clock) at which the last layer lands. */
  go(tl: Timeline, pose: string, o: GoOptions): Seconds {
    const dur = o.dur ?? 0.6;
    const members = this.pose(pose);
    const names = [...(o.order ?? []).filter((n) => n in members), ...Object.keys(members).filter((n) => !(o.order ?? []).includes(n))];
    let end = o.at;
    names.forEach((name, i) => {
      const from = this.now.get(name);
      if (!from) throw new Error(`stage.go("${pose}"): layer "${name}" has no starting pose; call start() with a pose that places it first`);
      const target = members[name];
      const changed = POSE_PROPS.filter((k) => differs(k, target, from));
      const at = o.at + (o.stagger ?? 0) * i;
      for (const [ease, props] of groupByEase(changed, o.ease)) {
        const vars: Record<string, unknown> = { duration: dur, ease };
        for (const k of props) if (k !== "opacity") vars[k] = target[k];
        if (Object.keys(vars).length > 2) tl.to(this.layers[name], vars, at);
        if (props.includes("opacity")) tl.to(fadeTargets(this.layers[name]), { opacity: target.opacity, duration: dur, ease }, at);
      }
      if (changed.length) end = Math.max(end, at + dur);
      this.now.set(name, { ...from, ...target });
    });
    return end;
  }

  private pose(name: string): Record<string, Pose> {
    const p = this.poses[name];
    if (!p) throw new Error(`stage: no pose "${name}"; poses are ${Object.keys(this.poses).join(", ")}`);
    return p;
  }
}

/** The layers whose opacity makes `layer` fade: itself, or for a group every layer under it. */
function fadeTargets(layer: Layer): Layer[] {
  return layer.kind === "null" ? layer.flat().filter((l) => l.kind !== "null") : [layer];
}

function groupByEase(props: PoseProp[], ease: GoOptions["ease"]): Array<[Ease, PoseProp[]]> {
  const fallback = (typeof ease === "string" ? ease : ease?.default) ?? "expo.out";
  const groups = new Map<Ease, PoseProp[]>();
  for (const k of props) {
    const e = typeof ease === "object" && ease[k] ? (ease[k] as Ease) : fallback;
    groups.set(e, [...(groups.get(e) ?? []), k]);
  }
  return [...groups];
}

/** Smart Animate: name poses for a set of layers, then `go("pose", { at, dur })` moves every layer to its new state. */
export const stage = (layers: Record<string, Layer>, poses: Record<string, Record<string, Pose>>): Stage => new Stage(layers, poses);
