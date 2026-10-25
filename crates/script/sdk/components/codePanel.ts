// A mono code panel: syntax colours (as text `runs`), a highlight bar that moves to the current line, and lines that are
// typed or revealed one per `perLine` seconds with a `tick` each.

import { Layer, group, rect, text } from "../layers";
import type { Timeline } from "../timeline";
import type { TextRun } from "../types";
import { Part, THEME, Theme, alpha, monoWidth, shadow } from "./util";

export interface CodePanelOpts {
  tl: Timeline;
  lines: string[];
  /** Seconds at which the first line starts. */
  at: number;
  /** Seconds between lines. */
  perLine?: number;
  /** `type` types each line out; `reveal` slides it in whole. */
  mode?: "type" | "reveal";
  /** Centre of the panel. */
  pos?: [number, number];
  w?: number;
  size?: number;
  /** File name in the panel's tab. */
  title?: string;
  /** Colours: keywords, strings, numbers, comments, plain text. */
  colors?: Partial<Record<"keyword" | "string" | "number" | "comment" | "plain", string>>;
  theme?: Partial<Theme>;
  id?: string;
}

const TOKEN = /(\/\/.*)|("[^"]*"|'[^']*'|`[^`]*`)|(\b\d+(?:\.\d+)?\b)|\b(import|from|export|default|const|let|new|return|await|async|function|type|if|else)\b/g;

/** Syntax `runs` for one line of TypeScript-like code. */
export function highlight(line: string, c: Record<string, string>): TextRun[] {
  const runs: TextRun[] = [];
  for (const m of line.matchAll(TOKEN)) {
    const kind = m[1] ? "comment" : m[2] ? "string" : m[3] ? "number" : "keyword";
    let nth = 0;
    for (let from = 0; ; ) {
      const i = line.indexOf(m[0], from);
      if (i < 0 || i >= m.index!) break;
      nth++;
      from = i + m[0].length;
    }
    runs.push({ match: m[0], nth, color: c[kind] });
  }
  return runs;
}

/** `codePanel({ tl, at: 12.5, perLine: 1, lines: ["const film = comp({ ... });", ...] })`. */
export function codePanel(o: CodePanelOpts): Part {
  const th = { ...THEME, ...o.theme };
  const { tl, lines, at, perLine = 1, mode = "type", pos = [520, 540], w = 880, size = 30, title = "film.ts" } = o;
  const col = { keyword: th.turbo, string: th.volt, number: th.cobalt, comment: th.steel, plain: "#E8E6DF", ...o.colors };
  const id = o.id ?? "code";
  const lineH = size * 1.65;
  const head = 72;
  const h = head + lines.length * lineH + 48;
  const top = -h / 2 + head + 24;
  const left = -w / 2 + 40;
  const codeX = left + monoWidth(3, size);
  const root = group({ id, pos });
  const panel = rect(w, h, { id: `${id}-panel`, parent: root, radius: 22, fill: "#10121A", stroke: { color: th.line, width: 2 }, fx: [shadow(60, 30, 0.45)] });
  const tab = text(title, { id: `${id}-tab`, parent: root, font: th.mono, size: 22, color: th.steel, align: "left", pos: [left, -h / 2 + 36] });
  const bar = rect(w - 24, lineH, { id: `${id}-bar`, parent: root, radius: 10, fill: alpha(th.cobalt, 0.18), pos: [0, top], opacity: 0 });
  const edge = rect(5, lineH - 10, { id: `${id}-edge`, parent: root, radius: 2.5, fill: th.turbo, pos: [-w / 2 + 18, top], opacity: 0 });
  const layers: Layer[] = [root, panel, tab, bar, edge];
  const times: number[] = [];
  lines.forEach((line, i) => {
    const t = at + i * perLine;
    const y = top + i * lineH;
    times.push(t);
    const gutter = text(String(i + 1).padStart(2, " "), { id: `${id}-n${i}`, parent: root, font: th.mono, size, color: alpha(th.steel, 0.55), align: "left", pos: [left, y], opacity: 0 });
    const preset = mode === "type" ? { name: "typewriter", at: t, each: Math.min(0.045, (perLine * 0.8) / Math.max(1, line.length)) } : undefined;
    const code = text(line, { id: `${id}-l${i}`, parent: root, font: th.mono, size, color: col.plain, align: "left", pos: [codeX, y], runs: highlight(line, col), preset, opacity: mode === "type" ? 1 : 0 });
    tl.to(gutter, { opacity: 1, duration: 0.2 }, t);
    if (mode === "reveal") tl.fromTo(code, { opacity: 0, x: codeX + 24 }, { opacity: 1, x: codeX, duration: 0.35, ease: "expo.out" }, t);
    if (i === 0) tl.to([bar, edge], { opacity: 1, duration: 0.2 }, t);
    else tl.to([bar, edge], { y: y, duration: 0.14, ease: "expo.out" }, t);
    tl.hit(t, "tick", 0.6);
    layers.push(gutter, code);
  });
  tl.from(root, { y: pos[1] + 50, duration: 0.6, ease: "expo.out" }, Math.max(0, at - 0.6));
  return { layers, end: at + lines.length * perLine, bar, times };
}
