// A run of short shots, each a word over a field, with the start/overlap arithmetic done for you. A shot's `start` is
// the leading edge of its incoming transition, so each shot is sized to hold `hold` seconds fully in view between the
// transition into it and the transition out.

import { shot, Shot } from "../comp";
import { solid, text } from "../layers";
import type { Layer } from "../layers";
import { timeline } from "../timeline";
import type { Color, Layerish, TransitionSpec } from "../types";
import { THEME } from "./util";

export interface MontageItem {
  word: string;
  /** What is behind the word: a colour, or any layers (an `aura`, a picture, a card wall). */
  field?: Color | Layerish;
  /** Extra layers above the field and below the word. */
  leaves?: Layerish;
  /** Seconds fully in view (the montage's `hold`). */
  hold?: number;
  /** A sound at the moment the shot is fully in. */
  sfx?: string;
}

export interface MontageOpts {
  items: MontageItem[];
  id?: string;
  /** Film time at which the first shot starts (leave out to follow the previous shot). */
  at?: number;
  hold?: number;
  /** The transition into every shot (the first included), 0.2 s dissolve by default. */
  transition?: TransitionSpec;
  size?: number;
  font?: string;
  weight?: number;
  color?: Color;
  /** The film's size, for the field and the word's centre (1920 x 1080). */
  frame?: [number, number];
  /** Pop each word in (true). */
  pop?: boolean;
}

export interface Montage {
  shots: Shot[];
  /** Film time each shot starts (its transition's leading edge) and when it is fully in view. */
  starts: number[];
  inView: Array<[number, number]>;
  /** When the last shot ends. */
  end: number;
  words: Layer[];
}

/** `film.timeline({ shots: [...intro, ...montage({ items: [{ word: "fast", field: "#FF5A1F" }, ...], hold: 0.5 }).shots] })`. */
export function montage(o: MontageOpts): Montage {
  const { items, id = "m", hold = 0.5, transition = { type: "dissolve", dur: 0.2 }, size = 220, frame = [1920, 1080] } = o;
  const pos: [number, number] = [frame[0] / 2, frame[1] / 2];
  if (!items.length) throw new Error("montage(): give at least one item { word, field?, leaves? }");
  const lead = transition.dur ?? 0.2;
  const [shots, starts, inView, words]: [Shot[], number[], Array<[number, number]>, Layer[]] = [[], [], [], []];
  let start = o.at ?? 0;
  let end = 0;
  items.forEach((item, i) => {
    const h = item.hold ?? hold;
    if (!(h > 0)) throw new Error(`montage(): item ${i} ("${item.word}") needs hold above 0, got ${h}`);
    // every shot but the last dissolves into the next one, so it lasts a transition longer
    const dur = lead + h + (i === items.length - 1 ? 0 : lead);
    const word = text(item.word, { id: `${id}${i}-word`, font: o.font ?? THEME.display, weight: o.weight ?? 800, size, color: o.color ?? THEME.paper, align: "center", pos });
    const tl = timeline();
    if (o.pop ?? true) tl.from(word, { scale: 0.7, opacity: 0, duration: 0.3, ease: "back.out(2)" }, lead);
    if (item.sfx) tl.hit(lead, item.sfx);
    const field = typeof item.field === "string" ? [solid(item.field, { id: `${id}${i}-field`, size: frame, pos })] : item.field;
    shots.push(shot({ id: `${id}${i}`, dur, in: transition, ...(i === 0 && o.at !== undefined ? { start: o.at } : {}), hits: tl.hits(), layers: [field, item.leaves, word] }));
    starts.push(start);
    inView.push([start + lead, start + lead + h]);
    words.push(word);
    end = start + dur;
    start += lead + h; // the next shot's transition begins as this one's hold ends
  });
  return { shots, starts, inView, end, words };
}
