// Shared look of the showcase films: palette, bundled fonts, and the chapter frame
// (header, caption, title card, fade-out) that every chapter wears.

import { comp, raw, rect, text, timeline } from "turbomotion";
import type { CompOpts, Film, Layer } from "turbomotion";

export const W = 1920;
export const H = 1080;
export const FPS = 30;

/** Seconds the title card holds before the chapter's content takes over. */
export const T0 = 1.5;

export const C = {
  bg: "#0b0d12",
  panel: "#10141d",
  line: "#2a3144",
  text: "#f2f4f8",
  dim: "#8a93a6",
  orange: "#ff6b3d",
  cyan: "#35d0ff",
  lime: "#c8f031",
  violet: "#8f6cff",
  pink: "#ff5c9a",
} as const;

/** Bundled families (motions/lib/fonts). Variable fonts render at their default instance, so no weights are asked of them. */
const FONT_DIR = "../../../motions/lib/fonts/";
export const FONT = { display: "Unbounded", body: "Archivo", mono: "JetBrains Mono", monoBold: "JetBrains Mono Bold" } as const;
const fontFiles: Record<string, string> = {
  [FONT.display]: `${FONT_DIR}Unbounded/Unbounded-Variable.ttf`,
  [FONT.body]: `${FONT_DIR}Archivo/Archivo-Variable.ttf`,
  [FONT.mono]: `${FONT_DIR}JetBrainsMono-Regular.woff`,
  [FONT.monoBold]: `${FONT_DIR}JetBrainsMono-Bold.woff`,
};

export type Tl = ReturnType<typeof timeline>;

/** A 1920x1080 30 fps film on the showcase background, with the bundled fonts declared. */
export function newFilm(dur: number, extra: Partial<CompOpts> = {}): Film {
  return comp({ width: W, height: H, fps: FPS, dur, bg: C.bg, fonts: fontFiles, ...extra });
}

/** A sub-composition (precomp source) as JSON: a panel's own world, with its own colour or effect-space settings if given. */
export function subComp(w: number, h: number, dur: number, layers: Layer[], extra: Partial<CompOpts> = {}): Record<string, unknown> {
  const film = comp({ width: w, height: h, fps: FPS, dur, bg: C.panel, ...extra });
  film.add(...layers);
  return film.toJSON() as Record<string, unknown>;
}

/** A precomp layer with its top-left corner at (x, y). */
export function place(id: string, name: string, x: number, y: number, extra: Record<string, unknown> = {}): Layer {
  return raw({ type: "precomp", id, comp: name, anchor: [0, 0], pos: [x, y], ...extra });
}

/** A hairline frame around a panel. */
export function border(id: string, x: number, y: number, w: number, h: number, color: string = C.line): Layer {
  return rect(w, h, { id, pos: [x + w / 2, y + h / 2], stroke: { color, width: 2 } });
}

/** A small monospace label whose left edge is x and whose middle is y. */
export function tag(id: string, words: string, x: number, y: number, color: string = C.dim, size = 28, align: "left" | "center" | "right" = "left"): Layer {
  return text(words, { id, font: FONT.mono, size, color, pos: [x, y], align });
}

/** Colour runs for the words of a caption that carry the number to look at. */
export type Hot = string[];

export interface FrameOpts {
  /** "01", "9a": shown huge on the title card and small in the header. */
  num: string;
  title: string;
  /** One line under the title on the card. */
  sub: string;
  /** What to look for, one or two lines at the bottom. */
  caption: string;
  /** Words of the caption to light up. */
  hot?: Hot;
  /** When the caption fades in (default just after the card). */
  captionAt?: number;
  dur: number;
  accent?: string;
  /** false: no title card (a chapter that continues the one before it); the header and caption show at once. */
  card?: boolean;
}

/**
 * Adds the chapter's frame to the film, above whatever content was added before it: header, caption, a title card that
 * holds for `T0` seconds and lifts away, and a fade to the background over the last quarter second.
 */
export function frame(film: Film, tl: Tl, o: FrameOpts): void {
  const accent = o.accent ?? C.orange;
  const card = o.card ?? true;
  const t0 = card ? T0 : 0.2;
  const header = text(`${o.num}   ${o.title.toUpperCase()}`, {
    id: "hdr", font: FONT.mono, size: 28, color: C.dim, pos: [96, 84], align: "left", tracking: 2,
    runs: [{ match: o.num, color: accent }],
  });
  const caption = text(o.caption, {
    id: "caption", font: FONT.body, size: 34, color: C.text, pos: [96, 975], align: "left", width: 1728,
    runs: (o.hot ?? []).map((m) => ({ match: m, color: C.lime })),
  });
  const rule = rect(1728, 2, { id: "rule", fill: C.line, pos: [960, 918] });
  const at = o.captionAt ?? t0 + 0.2;
  tl.from(header, { opacity: 0, duration: 0.4 }, t0);
  tl.from(rule, { opacity: 0, duration: 0.4 }, t0);
  tl.from(caption, { opacity: 0, duration: 0.5 }, at);

  // title card: opaque, so the content underneath may already be running
  const panel = rect(W, H, { id: "card", fill: C.bg, pos: [960, 540] });
  const bar = rect(140, 10, { id: "card-bar", fill: accent, pos: [166, 470] });
  const num = text(o.num, { id: "card-num", font: FONT.display, size: 250, color: accent, pos: [96, 330], align: "left" });
  const title = text(o.title, { id: "card-title", font: FONT.display, size: 80, color: C.text, pos: [96, 590], align: "left", width: 1728 });
  const sub = text(o.sub, { id: "card-sub", font: FONT.body, size: 40, color: C.dim, pos: [96, 720], align: "left", width: 1728 });
  const parts = [bar, num, title, sub];
  if (card) {
    parts.forEach((l, i) => tl.from(l, { opacity: 0, duration: 0.35, ease: "power2.out" }, 0.1 + i * 0.1));
    tl.to(num, { y: 300, duration: 0.5, ease: "power3.out" }, 0.1);
    [panel, ...parts].forEach((l) => tl.to(l, { opacity: 0, duration: 0.35, ease: "power2.in" }, T0 - 0.35));
  }

  const fade = rect(W, H, { id: "fade", fill: C.bg, pos: [960, 540], opacity: 0 });
  tl.to(fade, { opacity: 1, duration: 0.25, ease: "none" }, o.dur - 0.25);
  film.add(header, rule, caption);
  if (card) film.add(panel, ...parts);
  film.add(fade);
}
