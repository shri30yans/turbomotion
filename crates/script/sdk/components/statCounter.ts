// A big number that counts up and slams: it lands with a scale overshoot and a `stamp`, a small mono label under it.

import { Layer, text } from "../layers";
import type { Timeline } from "../timeline";
import { Part, THEME, Theme } from "./util";
import { entrance } from "./appear";
import type { AppearOpt } from "./appear";

export interface StatCounterOpts {
  /** An entrance for the whole component (an engine preset such as "fade" or "slide_up", or a spec); see `appear`. */
  appear?: AppearOpt;
  tl: Timeline;
  value: number;
  suffix?: string;
  prefix?: string;
  label: string;
  /** Seconds at which the count starts. */
  at: number;
  /** Seconds the count runs before the slam. */
  count?: number;
  pos?: [number, number];
  size?: number;
  color?: string;
  decimals?: number;
  /** Sound on the slam; `false` for none. */
  sfx?: string | false;
  theme?: Partial<Theme>;
  id?: string;
}

/** `statCounter({ tl, value: 373, suffix: " fps", label: "frames per second", at: 36 })`. */
export function statCounter(o: StatCounterOpts): Part {
  const th = { ...THEME, ...o.theme };
  const { tl, value, at, count = 0.35, pos = [960, 480], size = 240, decimals = 0, sfx = "stamp" } = o;
  const id = o.id ?? `stat-${Math.round(at * 100)}`;
  const slam = at + count;
  const num = text(
    { from: 0, to: value, at, dur: count, ease: "expo.out", decimals, sep: ",", prefix: o.prefix, suffix: o.suffix },
    { id, font: th.display, weight: 800, size, color: o.color ?? th.paper, align: "center", pos, opacity: 0 },
  );
  const caption = text(o.label, { id: `${id}-label`, font: th.mono, size: 26, color: th.steel, align: "center", pos: [pos[0], pos[1] + size * 0.62], opacity: 0 });
  tl.to([num, caption], { opacity: 1, duration: 0.12 }, at);
  tl.fromTo(num, { scale: 1.16 }, { scale: 1, duration: 0.45, ease: "back.out(2.2)" }, slam);
  tl.from(caption, { y: pos[1] + size * 0.62 + 16, duration: 0.5, ease: "expo.out" }, slam);
  if (sfx) tl.hit(slam, sfx);
  const layers: Layer[] = [num, caption];
  return entrance({ layers, end: slam + 0.45, num, caption, slam }, o.appear, at);
}
