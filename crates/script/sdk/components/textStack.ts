// A stack of words that arrive on cue and lay themselves out. Rows grow downward (or upward from a bottom edge), a row
// that gets too wide wraps, and a centred row re-centres as words join it. Every position comes from the renderer's own
// text shaping, and words already on screen glide to their new place (a FLIP reflow) instead of jumping.

import { Layer, text } from "../layers";
import { measure } from "../measure";
import type { Timeline } from "../timeline";
import type { Ease } from "../types";
import type { Part } from "./util";

/** How one kind of word looks and enters. */
export interface StackRole {
  font?: string;
  size: number;
  weight?: number;
  italic?: boolean;
  color: string;
  tracking?: number;
  /** Set the word in capitals. */
  upper?: boolean;
  /** Characters per second: the word types in. Without it the word enters with `entry` (default `fade_up`). */
  cps?: number;
  entry?: { name: string; [option: string]: unknown };
}

export interface StackItem {
  text: string;
  /** Seconds (timeline clock) at which the word arrives. */
  at: number;
  role: string;
  /** Rows are numbered; a word with the same row as the one before it joins that row, a new number starts a row. */
  row?: number;
}

/** Options for `textStack`: lines of text that reflow (FLIP) as items are added, removed or reordered. */
export interface TextStackOpts {
  tl: Timeline;
  items: StackItem[];
  roles: Record<string, StackRole>;
  /** Left, centre or right edge of the stack (by `align`) and its top (or bottom, with `anchor: "bottom"`). */
  pos: [number, number];
  /** Rows wrap before they get wider than this. */
  width: number;
  align?: "left" | "center" | "right";
  anchor?: "top" | "bottom";
  /** Row height as a share of the tallest word's line box. Display type wants less than 1. Default 0.88. */
  leading?: number;
  /** Space between words on one row, in pixels. Default: a quarter of the smallest type size. */
  gap?: number;
  reflowDur?: number;
  reflowEase?: Ease;
  idPrefix?: string;
}

interface Metrics {
  w: number;
  h: number;
  baseline: number;
}

/** `textStack({ tl, items, roles, pos: [90, 120], width: 560 })`. Returns the word layers in `words` (item order). */
export function textStack(o: TextStackOpts): Part {
  const { tl, items, roles, pos, width } = o;
  const align = o.align ?? "left";
  const leading = o.leading ?? 0.88;
  const reflowDur = o.reflowDur ?? 0.45;
  const reflowEase = o.reflowEase ?? "expo.out";
  const prefix = o.idPrefix ?? "stack";
  if (!items.length) throw new Error("textStack: items is empty");
  const sizes = Object.values(roles).map((r) => r.size);
  const gap = o.gap ?? Math.min(...sizes) * 0.25;

  const label = items.map((it) => {
    const role = roles[it.role];
    if (!role) throw new Error(`textStack: item "${it.text}" uses role "${it.role}"; roles are ${Object.keys(roles).join(", ")}`);
    return role.upper ? it.text.toUpperCase() : it.text;
  });
  const metrics: Metrics[] = items.map((it, i) => {
    const r = roles[it.role];
    const m = measure(label[i], { font: r.font, size: r.size, weight: r.weight, italic: r.italic, tracking: r.tracking });
    return { w: m.width, h: m.height, baseline: m.baseline };
  });
  const rows = items.map((it, i) => (it.row ?? (i === 0 ? 0 : items[i - 1].row ?? 0)));

  /** Where the first `count` words sit once they have all arrived: the left edge and vertical centre of each. */
  const layoutAt = (count: number): Array<[number, number]> => {
    const lines: number[][] = [];
    for (let i = 0; i < count; i++) {
      const last = lines[lines.length - 1];
      const sameRow = last !== undefined && rows[last[last.length - 1]] === rows[i];
      if (sameRow) {
        const lineW = last.reduce((s, k) => s + metrics[k].w, 0) + gap * (last.length - 1);
        if (lineW + gap + metrics[i].w <= width) {
          last.push(i);
          continue;
        }
      }
      lines.push([i]);
    }
    const placed: Array<[number, number]> = Array(count);
    const heights = lines.map((line) => {
      const ascent = Math.max(...line.map((k) => metrics[k].baseline));
      const descent = Math.max(...line.map((k) => metrics[k].h - metrics[k].baseline));
      return { ascent, advance: (ascent + descent) * leading };
    });
    const total = heights.reduce((s, h) => s + h.advance, 0);
    let top = o.anchor === "bottom" ? pos[1] - total : pos[1];
    lines.forEach((line, li) => {
      const lineW = line.reduce((s, k) => s + metrics[k].w, 0) + gap * (line.length - 1);
      let x = align === "left" ? pos[0] : align === "center" ? pos[0] - lineW / 2 : pos[0] - lineW;
      for (const k of line) {
        placed[k] = [x, top + heights[li].ascent - metrics[k].baseline + metrics[k].h / 2];
        x += metrics[k].w + gap;
      }
      top += heights[li].advance;
    });
    return placed;
  };

  const layers: Layer[] = items.map((it, i) => {
    const r = roles[it.role];
    const [x, y] = layoutAt(i + 1)[i];
    const preset = r.cps
      ? { name: "typewriter", at: it.at, dur: Math.max(0.001, (label[i].replace(/\s/g, "").length - 1) / r.cps) }
      : { name: "fade_up", ...(r.entry ?? {}), at: it.at };
    return text(label[i], {
      id: `${prefix}-${i}`, font: r.font, size: r.size, weight: r.weight, italic: r.italic, color: r.color, tracking: r.tracking,
      align: "left", x, y, preset,
    });
  });

  // every arrival can move the words already there: glide them
  for (let k = 1; k < items.length; k++) {
    const before = layoutAt(k);
    const after = layoutAt(k + 1);
    for (let i = 0; i < k; i++) {
      const [dx, dy] = [after[i][0] - before[i][0], after[i][1] - before[i][1]];
      if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
      tl.to(layers[i], { x: after[i][0], y: after[i][1], duration: reflowDur, ease: reflowEase, overwrite: "auto" }, items[k].at);
    }
  }
  const end = Math.max(...items.map((it, i) => it.at + (roles[it.role].cps ? label[i].length / (roles[it.role].cps as number) : 0.7)));
  return { layers, end, words: layers, height: layoutAt(items.length).reduce((m, [, y], i) => Math.max(m, y + metrics[i].h / 2), 0) - pos[1] };
}
