// Horizontal bars that grow to their values, each with a label on the left and a counter riding its tip.

import { Layer, shape, text } from "../layers";
import type { Timeline } from "../timeline";
import { Part, THEME, Theme, alpha } from "./util";

export interface Bar {
  label: string;
  value: number;
  color: string;
  /** Seconds this bar takes to grow (default: the race's `dur`); a short one makes a bar that "stops". */
  dur?: number;
}

export interface BarRaceOpts {
  tl: Timeline;
  bars: Bar[];
  at: number;
  dur?: number;
  /** Top-left of the first row. */
  pos?: [number, number];
  /** Length of a full bar (value = `max`). */
  w?: number;
  max?: number;
  rowH?: number;
  barH?: number;
  labelW?: number;
  ease?: string;
  /** A `counter` tick when each bar starts; `false` for none. */
  sfx?: string | false;
  theme?: Partial<Theme>;
  id?: string;
}

/** `barRace({ tl, at: 37, dur: 2.4, bars: [{ label: "turbomotion", value: 373, color: "#ff5a1f" }, ...] })`. */
export function barRace(o: BarRaceOpts): Part {
  const th = { ...THEME, ...o.theme };
  const { tl, bars, at, dur = 2, pos = [160, 360], w = 1100, rowH = 120, barH = 46, labelW = 330, ease = "expo.out", sfx = "counter" } = o;
  const max = o.max ?? Math.max(...bars.map((b) => b.value));
  const id = o.id ?? "race";
  const x0 = pos[0] + labelW;
  const layers: Layer[] = [];
  let end = at;
  bars.forEach((b, i) => {
    const y = pos[1] + i * rowH;
    const d = b.dur ?? dur;
    const len = (b.value / max) * w;
    end = Math.max(end, at + d);
    const grow = { at, dur: d, ease };
    layers.push(
      text(b.label, { id: `${id}-label${i}`, font: th.mono, size: 28, color: th.steel, align: "left", pos: [pos[0], y] }),
      shape([{ rect: [0, -barH / 2, w, barH], radius: barH / 2, fill: alpha(th.paper, 0.06) }], { id: `${id}-track${i}`, pos: [x0, y] }),
      shape([{ rect: [0, -barH / 2, len, barH], radius: barH / 2, fill: b.color }], {
        id: `${id}-bar${i}`, pos: [x0, y], scale: { from: [0, 1], to: [1, 1], ...grow },
      }),
      text({ from: 0, to: b.value, ...grow, decimals: 0, sep: "," }, {
        id: `${id}-value${i}`, font: th.display, weight: 800, size: 56, color: th.paper, align: "left", pos: [{ from: x0 + 28, to: x0 + len + 28, ...grow }, y],
      }),
    );
    if (sfx) tl.hit(at + i * 0.05, sfx, 0.5);
  });
  return { layers, end };
}
