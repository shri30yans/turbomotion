// A generic browser window: three dots, a URL pill, a spinner and a skeleton page. No real brand marks.

import { Layer, group, shape, text } from "../layers";
import type { Timeline } from "../timeline";
import { Part, THEME, Theme, alpha, shadow } from "./util";

export interface BrowserWindowOpts {
  tl: Timeline;
  url?: string;
  at?: number;
  pos?: [number, number];
  w?: number;
  h?: number;
  /** Slides up from this many pixels below when it enters. */
  slide?: number;
  theme?: Partial<Theme>;
  id?: string;
}

/** `browserWindow({ tl, url: "localhost:3000", at: 0.4 })`: `root` is the group to move or break apart. */
export function browserWindow(o: BrowserWindowOpts): Part {
  const th = { ...THEME, ...o.theme };
  const { tl, url = "localhost:3000", at = 0, pos = [960, 560], w = 1240, h = 700, slide = 260 } = o;
  const id = o.id ?? "browser";
  const bar = 88;
  const r = 26;
  const top = -h / 2;
  const root = group({ id, pos });
  const child = (items: Array<Record<string, unknown>>, name: string, x = 0, y = 0, extra: Record<string, unknown> = {}): Layer =>
    shape(items, { id: `${id}-${name}`, parent: root, pos: [x, y], ...extra });
  const body = child([{ rect: [-w / 2, top, w, h], radius: r, fill: "#12141C", stroke: { color: th.line, width: 2 } }], "body", 0, 0, { fx: [shadow(70, 36, 0.5)] });
  const d = `M ${-w / 2} ${top + bar} L ${-w / 2} ${top + r} Q ${-w / 2} ${top} ${-w / 2 + r} ${top} L ${w / 2 - r} ${top} Q ${w / 2} ${top} ${w / 2} ${top + r} L ${w / 2} ${top + bar} Z`;
  const head = child([{ d, fill: "#181B25" }], "head");
  const dots = child([-1, 0, 1].map((k) => ({ ellipse: [-w / 2 + 52 + (k + 1) * 34, top + bar / 2, 9, 9], fill: alpha(th.steel, 0.7 - 0.15 * (k + 1)) })), "dots");
  const pillW = w - 330;
  const pill = child([{ rect: [-pillW / 2, -23, pillW, 46], radius: 23, fill: th.ink, stroke: { color: th.line, width: 2 } }], "pill", 0, top + bar / 2);
  const address = text(url, { id: `${id}-url`, parent: root, font: th.mono, size: 24, color: th.steel, align: "left", pos: [-pillW / 2 + 28, top + bar / 2] });
  const spin = child([{ ellipse: [0, 0, 14, 14], stroke: { color: th.steel, width: 4, cap: "round" }, trim: { start: 0, end: 0.3 } }], "spin", w / 2 - 58, top + bar / 2,
    { rotation: { keys: [[0, 0], [1, 360, "none"]], loop: "cycle" } });
  const page = child([
    { rect: [-w / 2 + 70, top + bar + 54, 420, 46], radius: 12, fill: "#1D2029" },
    { rect: [-w / 2 + 70, top + bar + 130, w - 140, 250], radius: 18, fill: "#171A23" },
    ...[0, 1, 2].map((k) => ({ rect: [-w / 2 + 70, top + bar + 420 + k * 56, (w - 140) * (1 - 0.18 * k), 22], radius: 11, fill: "#1D2029" })),
  ], "page");
  tl.from(root, { y: pos[1] + slide, duration: 0.7, ease: "expo.out" }, at);
  tl.from([body, head, dots, pill, address, spin, page], { opacity: 0, duration: 0.4 }, at);
  return { layers: [root, body, head, dots, pill, address, spin, page], end: at + 0.7, root, spinner: spin, size: [w, h] };
}
