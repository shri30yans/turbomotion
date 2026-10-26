// A glass prompt card with a typed command: mono text, a caret that follows the typed end and blinks on beats, a `type`
// sound per character and an optional Enter (the card presses on a spring; the caret becomes a dot you can swell).

import { Layer, group, rect, shape, text } from "../layers";
import type { Timeline } from "../timeline";
import { Part, THEME, Theme, alpha, monoWidth, shadow } from "./util";
import { entrance } from "./appear";
import type { AppearOpt } from "./appear";

export interface PromptBoxOpts {
  /** An entrance for the whole component (an engine preset such as "fade" or "slide_up", or a spec); see `appear`. */
  appear?: AppearOpt;
  tl: Timeline;
  text: string;
  /** Seconds (shot-local) at which the first character is typed. */
  at?: number;
  /** Characters per second. */
  cps?: number;
  pos?: [number, number];
  w?: number;
  h?: number;
  /** Mono font size in pixels. */
  size?: number;
  /** The prompt glyph before the text. */
  prompt?: string;
  /** When Enter is pressed (the card presses, the caret turns into `dot`). Omit for no Enter. */
  enterAt?: number;
  /** The caret blinks once per `beat` seconds. */
  beat?: number;
  /** When the card rises in (default: 0.9 s before `at`). */
  rise?: number;
  caretColor?: string;
  theme?: Partial<Theme>;
  id?: string;
}

/** `promptBox({ tl, text: "turbomotion render launch.ts", at: 0.9, cps: 14, enterAt: 3.2 })`. */
export function promptBox(o: PromptBoxOpts): Part {
  const th = { ...THEME, ...o.theme };
  const { tl, text: typed, at = 0.8, cps = 14, pos = [960, 540], w = 1280, h = 230, size = 56, prompt = "$", enterAt, beat = 0.5 } = o;
  const id = o.id ?? "prompt";
  const rise = o.rise ?? Math.max(0, at - 0.9);
  const caretColor = o.caretColor ?? th.turbo;
  const each = 1 / cps;
  const chars = [...typed];
  const cw = monoWidth(1, size);
  const x0 = -w / 2 + 72 + cw * (prompt.length + 1);
  // the typewriter preset staggers per visible character: a space takes no time of its own
  const visible = chars.filter((c) => c !== " ").length;
  const end = at + visible * each;

  const root = group({ id, pos });
  const card = rect(w, h, {
    id: `${id}-card`, parent: root, radius: 40,
    fill: { gradient: { type: "linear", from: [0, -h / 2], to: [0, h / 2], stops: [[0, "#ffffffe6"], [1, "#ffffff8c"]] } },
    stroke: { color: "#ffffff", width: 2.5 },
    fx: [shadow(48, 28, 0.16)],
  });
  const rim = rect(w - 4, h - 4, { id: `${id}-rim`, parent: root, radius: 38, stroke: { color: alpha(th.ink, 0.1), width: 1.5 } });
  const glyph = text(prompt, { id: `${id}-glyph`, parent: root, font: th.mono, size, color: th.steel, align: "left", pos: [x0 - cw * (prompt.length + 1), 0] });
  const body = text(typed, {
    id: `${id}-text`, parent: root, font: th.mono, size, color: th.ink, align: "left", pos: [x0, 0],
    preset: { name: "typewriter", at, each },
  });
  const caret = rect(size * 0.13, size * 1.12, { id: `${id}-caret`, parent: root, radius: size * 0.065, fill: caretColor, pos: [x0, 0], opacity: 0 });
  const caretEnd = x0 + chars.length * cw;
  const dot = shape([{ ellipse: [0, 0, size * 0.3, size * 0.3], fill: caretColor }], { id: `${id}-dot`, pos: [pos[0] + caretEnd, pos[1]], scale: 0 });

  tl.from(root, { y: pos[1] + 70, duration: 0.7, ease: "expo.out" }, rise);
  tl.from([card, rim, glyph], { opacity: 0, duration: 0.5, ease: "power2.out" }, rise);

  // the caret: blinks on beats until typing, steps with every character, blinks on after
  const half = beat / 2;
  const blink = (from: number, to: number): void => {
    for (let t = from, on = true; t < to - 1e-6; t += half, on = !on) tl.set(caret, { opacity: on ? 1 : 0 }, t);
  };
  blink(rise + 0.3, at - half);
  tl.set(caret, { opacity: 1 }, at);
  let typedCount = 0;
  chars.forEach((c, k) => {
    if (c === " ") return; // the caret hops over a space together with the next character
    const t = at + typedCount++ * each;
    tl.set(caret, { x: x0 + (k + 1) * cw }, t); // a set is exact: the caret moves on the frame the character appears
    tl.hit(t, "type");
  });
  const stop = enterAt ?? end + 2 * beat;
  blink(end + half, stop);

  if (enterAt !== undefined) {
    tl.set(caret, { opacity: 0 }, enterAt);
    tl.hit(enterAt, "click");
    tl.to(root, { scale: 0.97, duration: 0.07, ease: "power2.out" }, enterAt);
    tl.to(root, { scale: 1, duration: 0.5, ease: "spring(0.3)" }, enterAt + 0.07);
    tl.to(dot, { scale: 1, duration: 0.2, ease: "back.out(2)" }, enterAt);
  }
  const layers: Layer[] = [root, card, rim, glyph, body, caret, dot];
  return entrance({ layers, end, caret, dot, root, card, enterAt: enterAt ?? null, caretEnd: [pos[0] + caretEnd, pos[1]] }, o.appear, at);
}
