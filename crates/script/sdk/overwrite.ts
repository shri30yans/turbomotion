// Interrupted properties retain their exact easing via the engine's scalar expression form.
import { relative } from "./relative";
import type { LayerTime } from "./compile";
import type { Tween } from "./timeline";
declare const __tm_ease: (ease: string, progress: number) => number;
declare const __tm_color: (value: string) => string;
type Value = number | string | number[];
interface Segment { at: number; end: number; from: number[]; to: number[]; ease: string }
const numeric = (value: Value, color: boolean): number[] => {
  if (color) return JSON.parse(__tm_color(JSON.stringify(value)));
  const values = Array.isArray(value) ? value : [value];
  if (!values.every((v) => typeof v === "number" && Number.isFinite(v))) throw new Error("overwrite:auto needs finite numeric or colour values");
  return values as number[];
};
const expand = (values: number[], count: number): number[] => values.length === 1 ? Array(count).fill(values[0]) : values;
const at = (segment: Segment, time: number): number[] => {
  const progress = segment.end === segment.at ? (time >= segment.at ? 1 : 0) : __tm_ease(segment.ease, (time - segment.at) / (segment.end - segment.at));
  return segment.from.map((value, i) => value + (segment.to[i] - value) * progress);
};

/** Compile only when an author opts into overwrite; the ordinary key/tween path remains compact. */
/** Segment times are on the timeline's clock; the expressions it writes use the layer's own time (`local`). */
export function overwritten(tweens: Tween[], base: Value | undefined, id: string, prop: string, local: LayerTime): unknown {
  const color = prop === "color";
  const segments: Segment[] = [];
  for (const tween of tweens) {
    const previous = segments[segments.length - 1];
    const overlap = previous && tween.at < previous.end - 1e-9;
    if (overlap && tween.overwrite !== "auto") throw new Error(`layer "${id}": two tweens of "${prop}" overlap; set overwrite: "auto" on the later tween or change its start`);
    const live = previous ? at(previous, tween.at) : base;
    const where = `layer "${id}": "${prop}"`;
    const fromValue = tween.from === undefined ? live : relative(tween.from, live, where);
    const toValue = tween.to === undefined || fromValue === undefined ? base : relative(tween.to, fromValue, where);
    if (fromValue === undefined || toValue === undefined) throw new Error(`layer "${id}": "${prop}" needs start and end values`);
    let from = Array.isArray(fromValue) && previous && tween.from === undefined ? fromValue : numeric(fromValue, color);
    let to = numeric(toValue, color);
    const count = Math.max(from.length, to.length);
    from = expand(from, count); to = expand(to, count);
    if (from.length !== to.length) throw new Error(`layer "${id}": "${prop}" tween dimensions must agree`);
    // Validate the name even when the tween is never interrupted.
    __tm_ease(tween.ease, 0);
    segments.push({ at: tween.at, end: tween.at + tween.dur, from, to, ease: tween.ease });
  }
  const count = Math.max(...segments.map((segment) => segment.from.length));
  const expressions = Array.from({ length: count }, (_, component) => {
    let expression = String(expand(segments[0].from, count)[component]);
    for (const segment of segments) {
      const from = expand(segment.from, count)[component];
      const to = expand(segment.to, count)[component];
      const [start, end] = [local(segment.at), local(segment.end)];
      const after = segment.end === segment.at ? String(to) : `ease(${JSON.stringify(segment.ease)},time,${start},${end},${from},${to})`;
      expression = `(time<${start}?${expression}:${after})`;
    }
    return expression;
  });
  return color ? { channels: expressions } : expressions.length === 1 ? expressions[0] : expressions;
}
