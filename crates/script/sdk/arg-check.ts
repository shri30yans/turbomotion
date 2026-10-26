// Checks on the positional arguments of the layer builders, so a wrong guess names the right form where it was written
// instead of surfacing as an engine schema error about a property the author never typed.

import type { Color } from "./types";

declare const __tm_layer_problems: (type: string, label: string, keys: string) => string;

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

/** Every key of a layer must be one the engine reads for that type; the engine itself says which (and the nearest valid name). */
export function checkLayerKeys(kind: string, id: string, keys: string[]): void {
  const problems: string[] = JSON.parse(__tm_layer_problems(kind, `${kind} layer ${JSON.stringify(id)}`, JSON.stringify(keys)));
  if (problems.length) throw new Error(problems.join("\n"));
}

/** `texture("noise", opts)`: the generator's name is the first argument, not an option. */
export function checkTextureShader(shader: unknown): void {
  if (typeof shader === "string" && shader) return;
  const hint = shader && typeof shader === "object" && !Array.isArray(shader) && "shader" in shader
    ? ` ("shader" is the first argument, not an option: texture("${String((shader as { shader: unknown }).shader)}", { ... }))`
    : "";
  throw new Error(`texture(shader, opts): shader is the name of a generator like "noise" or "gradient" (\`turbomotion comp fx list\` shows them), got ${show(shader)}${hint}`);
}

/** Layer properties a script may still assign after construction: the builder's own state (everything else is written by \`set\`). */
export const LAYER_STATE = ["id", "props", "tweens", "children"];

/** \`layer.effects = [...]\` writes nowhere: a layer's options are the engine's keys, in \`props\`. Name the place that works. */
export function refuseLayerAssignment(kind: string, id: string, key: string): never {
  const where = `${kind} layer ${JSON.stringify(id)}`;
  if (key === "effects" || key === "effect") {
    throw new Error(`${where}: layer.${key} = ... has no effect: a layer's effects are its "fx" key, a list like [{ effect: "blur", radius: 8 }]; write it in the options (text("hi", { fx: [...] })) or later with layer.set({ fx: [...] })`);
  }
  throw new Error(`${where}: layer.${key} = ... has no effect: options are written where the layer is made, or later with layer.set({ ${key}: ... }) (animate with a timeline)`);
}
