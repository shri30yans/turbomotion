// The vertigo shot: the camera travels while its lens zooms the other way, so the subject plane stays the same size
// and the depth behind it stretches.

import type { Layer } from "../layers";
import type { Timeline } from "../timeline";

export interface DollyZoomOpts {
  tl: Timeline;
  at: number;
  dur: number;
  /** How far the camera moves in, as a fraction of its distance (0.45). Negative pulls out. */
  amount?: number;
  /** The camera's distance to the plane that stays put, also its starting `zoom` (the camera's own `zoom`, else 1000). */
  distance?: number;
  ease?: string;
}

/** `dollyZoom(cam, { tl, at: 0.5, dur: 2, amount: 0.4 })`. Give the camera `pos: [cx, cy, -distance]` and `zoom: distance` first. Returns the end time. */
export function dollyZoom(cam: Layer, o: DollyZoomOpts): number {
  const k = o.amount ?? 0.45;
  if (!(k > -1 && k < 1) || k === 0) throw new Error(`dollyZoom(): amount must be between -1 and 1 and not 0, got ${k}`);
  const given = cam.props.zoom;
  const d = o.distance ?? (typeof given === "number" ? given : 1000);
  const pos = cam.props.pos;
  if (!Array.isArray(pos)) cam.set({ pos: [960, 540, -d] });
  o.tl.fromTo(cam, { z: -d, zoom: d }, { z: -d * (1 - k), zoom: d * (1 - k), duration: o.dur, ease: o.ease ?? "power2.inOut" }, o.at);
  return o.at + o.dur;
}
