// A cut-out subject that is alive: a picture (or any layer) standing on the ground with a contact shadow, moving by itself.
// Idle motion is a looping expression on the *body*, so the group around it stays free for the film's own moves (a stage
// glides the group between poses, a timeline enters it) and the two never fight over one property.
//
//   const hand = cutout({ id: "hand", src: "assets/hand.png", size: [300, 290], pivot: "base",
//                         idle: { float: { amp: 12, period: 3.2 }, sway: { deg: 3, period: 4.1 } },
//                         shadow: { width: 260, y: 150 }, ring: { rx: 230 } });
//   shot({ layers: [hand] });                 // pos, scale, rotation and opacity of `hand.root` are yours to move

import { Layer, group, image, raw, shape } from "../layers";
import type { Part } from "./util";

/** A looping wave: one full cycle every `period` seconds, offset by `phase` (0..1 of a cycle). */
export interface Wave {
  period: number;
  phase?: number;
}

export interface Idle {
  /** Bobs up and down by `amp` pixels. */
  float?: Wave & { amp: number };
  /** Rocks by `deg` degrees about the pivot. */
  sway?: Wave & { deg: number };
  /** Swells by `amount` (0.02 is 2 %). */
  breathe?: Wave & { amount: number };
  /** Drifts sideways by `amp` pixels. */
  drift?: Wave & { amp: number };
  /** Rocks in depth by `x` and `y` degrees. Needs a camera in the shot (`camera({ pos, poi, zoom })`). */
  tilt?: Wave & { x: number; y: number };
}

/** Options for `cutout`: an image or built layer that lives by itself (float, sway, breathe, drift, tilt). */
export interface CutoutOpts {
  id: string;
  /** An image file, or a layer you built (a group of shapes) to move as the body. */
  src: string | Layer;
  /** The picture's size on screen at scale 1. Required for an image. */
  size?: [number, number];
  /** Where the group sits. A stage can take over from here. */
  pos?: [number, number];
  /** What the body rocks about: its centre, or its `base` (feet), or its `top`. */
  pivot?: "center" | "base" | "top";
  idle?: Idle;
  /** A soft ellipse under the subject that shrinks as the subject rises. `y` is its offset from the group's centre. */
  shadow?: { width: number; y: number; color?: string; blur?: number };
  /** A thin tilted ring around the subject. */
  ring?: { rx: number; y?: number; flatten?: number; color?: string; width?: number; tilt?: number };
  /** Effects on the picture, e.g. a duotone that pulls it into the film's palette. */
  fx?: Array<Record<string, unknown>>;
}

const wave = (w: Wave): string => `sin((time/${w.period}+${w.phase ?? 0})*tau)`;

/** `cutout({ id, src, size, idle, shadow, ring })`: `root` is the group to place and move; `body` is the picture. */
export function cutout(o: CutoutOpts): Part {
  const { id, idle = {} } = o;
  const [w, h] = o.size ?? [0, 0];
  if (typeof o.src === "string" && !o.size) throw new Error(`cutout "${id}": an image needs size: [w, h]`);
  const pivotY = o.pivot === "base" ? h / 2 : o.pivot === "top" ? -h / 2 : 0;
  // an image measures its anchor from its top-left corner; a built layer (shapes) from its own centre
  const anchor = typeof o.src === "string" ? [w / 2, h / 2 + pivotY] : [0, pivotY];

  const float = idle.float ? `${-idle.float.amp}*${wave(idle.float)}` : "0";
  const bodyProps = {
    id: `${id}-body`,
    pos: [idle.drift ? `${idle.drift.amp}*${wave(idle.drift)}` : 0, idle.float ? float : 0],
    ...(idle.sway ? { rotation: `${idle.sway.deg}*${wave(idle.sway)}`, anchor } : {}),
    ...(idle.breathe ? { scale: `1+${idle.breathe.amount}*${wave(idle.breathe)}` } : {}),
    ...(idle.tilt ? {
      three_d: true,
      rot_x: `${idle.tilt.x}*${wave(idle.tilt)}`,
      rot_y: `${idle.tilt.y}*sin((time/${idle.tilt.period}+${(idle.tilt.phase ?? 0) + 0.25})*tau)`,
    } : {}),
    ...(o.fx ? { fx: o.fx } : {}),
  };
  const body = typeof o.src === "string" ? image(o.src, { ...bodyProps, size: [w, h], fit: "contain" }) : o.src.set(bodyProps);

  const parts: Layer[] = [];
  let shadow: Layer | undefined;
  let ring: Layer | undefined;
  if (o.shadow) {
    const s = o.shadow;
    // the shadow's colour carries its strength in its alpha, so a group fade never overrides it
    shadow = shape([{ ellipse: [0, 0, s.width / 2, s.width * 0.08], fill: s.color ?? "#1d1a6366" }], {
      id: `${id}-shadow`, pos: [0, s.y], fx: [{ effect: "blur", radius: s.blur ?? s.width * 0.05 }],
      ...(idle.float ? { scale: `1+0.08*${wave(idle.float)}` } : {}),
    });
    parts.push(shadow);
  }
  if (o.ring) {
    const r = o.ring;
    ring = raw({
      type: "shape", id: `${id}-ring`, pos: [0, r.y ?? 0], rotation: r.tilt ?? -12,
      items: [{ ellipse: [0, 0, r.rx, r.rx * (r.flatten ?? 0.3)], stroke: { color: r.color ?? "#ffffffcc", width: r.width ?? 3, cap: "round" } }],
    });
    parts.push(ring);
  }
  parts.push(body);
  const root = group({ id, ...(o.pos ? { pos: o.pos } : {}) }, parts);
  return { layers: [root], end: 0, root, body, shadow, ring };
}
