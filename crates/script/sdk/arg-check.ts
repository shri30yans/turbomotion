// Checks on the positional arguments of the layer builders, so a wrong guess names the right form where it was written
// instead of surfacing as an engine schema error about a property the author never typed.

import type { Color } from "./types";

/** What was passed, short enough for a message. */
export function show(v: unknown): string {
  if (v === null) return "null";
  if (Array.isArray(v)) return `a list of ${v.length}`;
  if (typeof v === "object") return `an object with keys ${Object.keys(v as object).slice(0, 5).join(", ")}`;
  return `${typeof v} ${JSON.stringify(v)}`;
}

const isColor = (v: unknown): boolean =>
  typeof v === "string" || (Array.isArray(v) && (v.length === 3 || v.length === 4) && v.every((n) => typeof n === "number"));

/** `solid("#101828")`: the colour is the first argument, not an option. */
export function checkSolidColor(color: unknown): asserts color is Color {
  if (isColor(color)) return;
  const hint = color && typeof color === "object" && !Array.isArray(color) && "color" in color
    ? ` ("color" is the first argument, not an option: solid("${String((color as { color: unknown }).color)}", { ... }))`
    : "";
  throw new Error(`solid(color, opts): color must be a colour like "#101828" or [r, g, b], got ${show(color)}${hint}`);
}

/** `text("Hello", opts)` or `text({ from: 0, to: 100 }, opts)`: the words (or a counter or swap) come first. */
export function checkTextContent(content: unknown): void {
  if (typeof content === "string") return;
  const counter = content && typeof content === "object" && !Array.isArray(content) && ("from" in content || "to" in content);
  if (counter) return;
  const hint = content && typeof content === "object" && !Array.isArray(content) && "text" in content
    ? ' (the words are the first argument, not an option: text("Hello", { size: 80 }))'
    : typeof content === "number" ? " (write String(n), or a counter { from, to } to count up)" : "";
  throw new Error(`text(content, opts): content must be a string like "Hello", a counter { from: 0, to: 100, dur: 1 } or a swap { from: "A", to: "B" }, got ${show(content)}${hint}`);
}

/** `rect(260, 340, opts)`: width and height in pixels. */
export function checkRectSize(w: unknown, h: unknown): void {
  const ok = (n: unknown) => typeof n === "number" && Number.isFinite(n) && n >= 0;
  if (ok(w) && ok(h)) return;
  throw new Error(`rect(w, h, opts): w and h are the size in pixels, like rect(260, 340, { fill: "#ff6b3d" }); got ${show(w)} and ${show(h)}`);
}

/** `group(opts, children)`: options first, then the layers that follow it. */
export function checkGroupOpts(o: unknown): void {
  if (o === undefined || (o && typeof o === "object" && !Array.isArray(o) && typeof (o as { toJSON?: unknown }).toJSON !== "function")) return;
  throw new Error(`group(opts, children): opts is an options object like { id: "rig", pos: [960, 540] } and the layers go second: group({ id: "rig" }, [a, b]); got ${show(o)}`);
}
