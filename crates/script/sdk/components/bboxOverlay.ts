// HUD bounding boxes with mono labels (`name · 1240×180 · ok`). A box has a state (ok / clash / fixed) shown in a token
// colour, and may change state and move at a later time (a clash that is fixed).

import { Layer, group, rect, text } from "../layers";
import type { Timeline } from "../timeline";
import { Part, THEME, Theme } from "./util";
import { entrance } from "./appear";
import type { AppearOpt } from "./appear";

export type BoxState = "ok" | "clash" | "fixed";

export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
  label: string;
  state?: BoxState;
  /** Later: the box changes state and may move by (dx, dy). */
  change?: { at: number; state: BoxState; dx?: number; dy?: number; dur?: number };
}

export interface BboxOverlayOpts {
  /** An entrance for the whole component (an engine preset such as "fade" or "slide_up", or a spec); see `appear`. */
  appear?: AppearOpt;
  tl: Timeline;
  boxes: Box[];
  at: number;
  /** Seconds between boxes appearing. */
  step?: number;
  /** Draws no `· w×h · state` after the label when false. */
  detail?: boolean;
  theme?: Partial<Theme>;
  id?: string;
}

const text3 = (b: Box, state: BoxState, detail: boolean): string => (detail ? `${b.label} · ${Math.round(b.w)}×${Math.round(b.h)} · ${state}` : b.label);

/** Box colours: ok = cobalt, clash = turbo (the red of this palette), fixed = volt. */
export const stateColor = (th: Theme, s: BoxState): string => ({ ok: th.cobalt, clash: th.turbo, fixed: th.volt })[s];

/** `bboxOverlay({ tl, at: 1, boxes: [{ x: 120, y: 80, w: 1240, h: 180, label: "title" }] })`; `x`,`y` are the top-left. */
export function bboxOverlay(o: BboxOverlayOpts): Part {
  const th = { ...THEME, ...o.theme };
  const { tl, boxes, at, step = 0.18, detail = true } = o;
  const id = o.id ?? "bbox";
  const layers: Layer[] = [];
  let end = at;
  boxes.forEach((b, i) => {
    const t = at + i * step;
    const g = group({ id: `${id}${i}`, pos: [b.x + b.w / 2, b.y + b.h / 2] });
    const states: BoxState[] = b.change ? [b.state ?? "ok", b.change.state] : [b.state ?? "ok"];
    const parts = states.flatMap((s, k) => {
      const c = stateColor(th, s);
      return [
        rect(b.w, b.h, { id: `${id}${i}-box${k}`, parent: g, stroke: { color: c, width: 3 }, opacity: k === 0 ? 1 : 0 }),
        text(text3(b, s, detail), { id: `${id}${i}-tag${k}`, parent: g, font: th.mono, size: 20, color: c, align: "left", pos: [-b.w / 2, -b.h / 2 - 22], opacity: k === 0 ? 1 : 0 }),
      ];
    });
    tl.from(parts.slice(0, 2), { opacity: 0, duration: 0.25, ease: "power2.out" }, t);
    tl.from(g, { scale: 1.04, duration: 0.3, ease: "expo.out" }, t);
    tl.hit(t, "tick", 0.5);
    if (b.change) {
      const c = b.change;
      tl.to(parts.slice(0, 2), { opacity: 0, duration: 0.12 }, c.at);
      tl.to(parts.slice(2), { opacity: 1, duration: 0.12 }, c.at);
      if (c.dx || c.dy) tl.to(g, { x: g.props.pos[0] + (c.dx ?? 0), y: g.props.pos[1] + (c.dy ?? 0), duration: c.dur ?? 0.6, ease: "expo.inOut" }, c.at + 0.1);
      end = Math.max(end, c.at + (c.dur ?? 0.6) + 0.1);
    }
    layers.push(g, ...parts);
    end = Math.max(end, t + 0.3);
  });
  return entrance({ layers, end }, o.appear, at);
}
