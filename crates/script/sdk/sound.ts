// Sound: audio layers, and hits (sound-kit cues placed on moments). Hits made on a timeline are shot-local seconds;
// `film.timeline` turns them into the engine's `"<shotId>.start+<t>"` literal, so the engine, which knows where each
// shot really starts once transitions overlap, puts them on the right frame.
//
// The kit (`comp catalogue` -> `sound_kit`) has ~230 synthesised sounds in families. A cue names one of them:
//   "whoosh"            any whoosh; the engine uses a different one each time (and a touch of pitch and gain)
//   "whoosh/airy"       one of the airy whooshes        "whoosh/airy-01"   exactly that file
//   "pluck/C4"          a pitched voice at a note (pluck, bell, marimba, stab, bass)
// `melody` and `drumPattern` turn a list of notes or a step pattern into hits, so a bed can be sequenced from kit voices.

import { Layer } from "./layers";
import type { AudioOpts, Hit } from "./types";

const r6 = (n: number): number => Math.round(n * 1e6) / 1e6;

/** A sound on the timeline: `audio("launch/music/bed.wav", { id: "music", volume: 0.8, fade_out: 1 })`. */
export function audio(src: string, o: AudioOpts = {}): Layer {
  if (typeof src !== "string" || !src) throw new Error('audio(src, opts): src is a file path or a "kit:<cue>" sound');
  return new Layer("audio", { src, ...o });
}

/**
 * A hit at film time: `hit(26, "boom")`, `hit("drop.start+0.2", "riser/quick", 0.5)`, `hit(3, "pluck/C4")`.
 * A family name ("whoosh", "kick", "ui") varies on its own: neighbouring hits of the same cue get different sounds.
 */
export const hit = (at: number | string, cue: string, volume?: number): Hit => {
  if (typeof cue !== "string" || !cue) throw new Error('hit(at, cue): cue is a sound-kit name like "pop", "whoosh/airy", "pluck/C4" or "kick"');
  return volume === undefined ? { at, cue } : { at, cue, volume };
};

/** Shot-local hits as film-time literals: `at: 0.8` in shot "deep" becomes `"deep.start+0.8"`. */
export function inShot(id: string, hits: Hit[] = []): Hit[] {
  return hits.map((h) => {
    if (typeof h.at !== "number") throw new Error(`shot "${id}": hit "${h.cue}" has at ${JSON.stringify(h.at)}; inside a shot, "at" is seconds from the shot's start`);
    const t = r6(h.at);
    return { ...h, at: `${id}.start${t < 0 ? "-" : "+"}${Math.abs(t)}` };
  });
}

/** `at` moved later by `t` seconds; `at` may be a literal such as `"drop.start+0.2"` or `"beat:8"`. */
function later(at: number | string, t: number): number | string {
  if (typeof at === "number") return r6(at + t);
  if (t === 0) return at;
  const m = /^(.*?)([+-]\d+(?:\.\d+)?)?$/.exec(at.trim());
  const sum = r6(Number(m?.[2] ?? 0) + t);
  return `${m?.[1] ?? at}${sum < 0 ? "-" : "+"}${Math.abs(sum)}`;
}

const NOTE = /^[A-Ga-g][#b]*-?\d$/;
const checkNote = (n: string, what: string): string => {
  if (!NOTE.test(n)) throw new Error(`${what}: "${n}" is not a note name like "C4", "F#3" or "Bb2"`);
  return n[0].toUpperCase() + n.slice(1);
};

/** One step of a melody: a note, several notes at once (a chord), or a rest (`null`, `"."`, `"-"`, `0`). */
export type Step = string | string[] | null | number | { note: string | string[]; volume?: number };

export interface MelodyOpts {
  /** Film time (or shot-local seconds inside `shot({ hits })`) of the first step. A time literal works outside shots. */
  at: number | string;
  /** Seconds between steps (default 0.25), or give `bpm` and `div`. */
  step?: number;
  /** Beats per minute and steps per beat (default 2): `step = 60 / bpm / div`. */
  bpm?: number;
  div?: number;
  /** A pitched kit voice: "pluck" (default), "bell", "marimba", "stab", "bass". */
  voice?: string;
  volume?: number;
  /** Steps are shifted by this many semitones (the engine retunes the nearest rendered note). */
  transpose?: number;
  /** Play the whole sequence this many times (default 1). */
  loops?: number;
  /** Late odd steps by this fraction of a step, 0 to 0.5. */
  swing?: number;
}

const stepSeconds = (o: { step?: number; bpm?: number; div?: number }, def: number, who: string): number => {
  const s = o.step ?? (o.bpm ? 60 / o.bpm / (o.div ?? 2) : def);
  if (!(s > 0) || !Number.isFinite(s)) throw new Error(`${who}: step must be a positive number of seconds (or give bpm)`);
  return s;
};

const NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
const PC: Record<string, number> = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 };
const midi = (n: string): number => PC[n[0]] + [...n.slice(1, -1)].reduce((a, c) => a + (c === "#" ? 1 : c === "b" ? -1 : 0), 0) + 12 * (Number(n.slice(-1)) + 1);
const nameOf = (m: number): string => NAMES[((m % 12) + 12) % 12] + (Math.floor(m / 12) - 1);

/** `transpose("C4", 7)` is `"G4"`. */
export const transpose = (note: string, semitones: number): string => nameOf(midi(checkNote(note, "transpose")) + Math.round(semitones));

/** The notes of a scale from `root` ("C3") upward: `scale("C3", "minor", 2)` is two octaves of C minor, ending on the top root. */
export function scale(root: string, mode: "major" | "minor" | "major-pentatonic" | "minor-pentatonic" | "chromatic" = "major", octaves = 1): string[] {
  const steps = { major: [0, 2, 4, 5, 7, 9, 11], minor: [0, 2, 3, 5, 7, 8, 10], "major-pentatonic": [0, 2, 4, 7, 9], "minor-pentatonic": [0, 3, 5, 7, 10], chromatic: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] }[mode];
  if (!steps) throw new Error(`scale: mode ${mode} is major, minor, major-pentatonic, minor-pentatonic or chromatic`);
  const base = midi(checkNote(root, "scale"));
  const out: string[] = [];
  for (let o = 0; o < octaves; o++) for (const s of steps) out.push(nameOf(base + 12 * o + s));
  out.push(nameOf(base + 12 * octaves));
  return out;
}

/**
 * A tune from a kit voice: `melody(["C4", "Eb4", "G4", null, "C5"], { at: 2, step: 0.25, voice: "pluck" })` emits one hit per
 * note (`pluck/C4` ...). Rests are `null`; an array is a chord. Notes outside the voice's two octaves are reported by the engine.
 */
export function melody(notes: Step[], o: MelodyOpts): Hit[] {
  const voice = o.voice ?? "pluck";
  const dt = stepSeconds(o, 0.25, "melody");
  const swing = Math.min(0.5, Math.max(0, o.swing ?? 0));
  const out: Hit[] = [];
  const loops = Math.max(1, Math.floor(o.loops ?? 1));
  for (let l = 0; l < loops; l++) {
    notes.forEach((s, i) => {
      if (s === null || s === 0 || s === "." || s === "-") return;
      const idx = l * notes.length + i;
      const vol = typeof s === "object" && !Array.isArray(s) ? s.volume : undefined;
      const raw = typeof s === "object" && !Array.isArray(s) ? s.note : s;
      for (const n of Array.isArray(raw) ? raw : [raw]) {
        if (typeof n !== "string") throw new Error(`melody: step ${i} is ${JSON.stringify(s)}; use a note name, an array of names, or null for a rest`);
        const note = o.transpose ? transpose(n, o.transpose) : checkNote(n, `melody step ${i}`);
        out.push(hit(later(o.at, r6(idx * dt + (idx % 2 ? swing * dt : 0))), `${voice}/${note}`, vol ?? o.volume));
      }
    });
  }
  return out;
}

export interface DrumOpts {
  at: number | string;
  /** Seconds per character, or `bpm` with `div` characters per beat (default 4: sixteenth notes). */
  step?: number;
  bpm?: number;
  div?: number;
  /** One kit cue for every hit ("kick", "snare/tight", "hat/closed"), or a map from pattern character to cue: `{ x: "kick/punchy-01", o: "hat/closed" }`. */
  voice?: string | Record<string, string>;
  volume?: number;
  /** Play the pattern this many times (default 1). */
  loops?: number;
  /** Late odd steps by this fraction of a step, 0 to 0.5. */
  swing?: number;
}

/**
 * Hits from a step pattern: `drumPattern("x...x...x...x.x.", { at: 4, bpm: 120, voice: "kick" })`. `x` hits, `X` accents,
 * `o` is a soft hit (half volume), `.`, `-` and spaces rest, `|` is a bar line (ignored). Use a map for several drums at once.
 * Use one call per drum so each line reads like a row in a drum machine.
 */
export function drumPattern(pattern: string, o: DrumOpts): Hit[] {
  if (typeof pattern !== "string") throw new Error('drumPattern(pattern, opts): pattern is a string like "x...x..."');
  const dt = stepSeconds({ ...o, div: o.div ?? 4 }, 0.125, "drumPattern");
  const swing = Math.min(0.5, Math.max(0, o.swing ?? 0));
  const voice = o.voice ?? "kick";
  const map = typeof voice === "string" ? { x: voice } : voice;
  const chars = [...pattern].filter((c) => c !== "|" && c !== " ");
  const out: Hit[] = [];
  const loops = Math.max(1, Math.floor(o.loops ?? 1));
  for (let l = 0; l < loops; l++) {
    chars.forEach((c, i) => {
      if (c === "." || c === "-") return;
      const own = map[c];
      const cue = own ?? (c === "X" || c === "o" ? map.x : undefined);
      if (!cue) throw new Error(`drumPattern: "${c}" at step ${i} has no voice; the voices are ${Object.keys(map).join(", ")} (use x for a hit, . for a rest)`);
      const k = own ? 1 : c === "X" ? 1.3 : c === "o" ? 0.5 : 1;
      const idx = l * chars.length + i;
      const v = o.volume === undefined && k === 1 ? undefined : r6((o.volume ?? 0.45) * k);
      out.push(hit(later(o.at, r6(idx * dt + (idx % 2 ? swing * dt : 0))), cue, v));
    });
  }
  return out;
}
