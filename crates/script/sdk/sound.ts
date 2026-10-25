// Sound: audio layers, and hits (sound-kit cues placed on moments). Hits made on a timeline are shot-local seconds;
// `film.timeline` turns them into the engine's `"<shotId>.start+<t>"` literal, so the engine, which knows where each
// shot really starts once transitions overlap, puts them on the right frame.

import { Layer } from "./layers";
import type { AudioOpts, Hit } from "./types";

const r6 = (n: number): number => Math.round(n * 1e6) / 1e6;

/** A sound on the timeline: `audio("launch/music/bed.wav", { id: "music", volume: 0.8, fade_out: 1 })`. */
export function audio(src: string, o: AudioOpts = {}): Layer {
  if (typeof src !== "string" || !src) throw new Error('audio(src, opts): src is a file path or a "kit:<cue>" sound');
  return new Layer("audio", { src, ...o });
}

/** A hit at film time: `hit(26, "boom")` or `hit("drop.start+0.2", "riser", 0.5)`. */
export const hit = (at: number | string, cue: string, volume?: number): Hit => {
  if (typeof cue !== "string" || !cue) throw new Error('hit(at, cue): cue is a sound-kit name like "pop", "tick" or "boom"');
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
