// A row of thin rounded bars that grow in and then pulse like an audio level, driven by expressions (sines plus the beat).

// imported from the package root (not "../layers") so the file also works as a plain relative import before it is registered in loader.rs
import { Layer, shape, rand } from "turbomotion";
import type { Timeline } from "turbomotion";
import { Part, THEME, Theme, r3 } from "./util";

export interface WaveformOpts {
  tl: Timeline;
  /** Seconds at which the bars start to grow (staggered across the row). */
  at: number;
  /** Centre of the row's baseline. */
  pos?: [number, number];
  w?: number;
  /** Height of the tallest bar. */
  h?: number;
  bars?: number;
  color?: string;
  seed?: number;
  /** Pulse speed in rad/s. */
  speed?: number;
  /** Extra kick on every beat, 0..1. */
  beatKick?: number;
  /** Beat rate (2 at 120 BPM) and shot-local time of a beat; `beat()` needs a bpm that precomps do not have. */
  beatsPerSec?: number;
  beatOffset?: number;
  theme?: Partial<Theme>;
  id?: string;
  parent?: Layer | string;
}

/** `waveform({ tl, at: 4.5, pos: [0, 200], w: 700, h: 90, parent: preview })`: one layer per bar, anchored on the baseline. */
export function waveform(o: WaveformOpts): Part {
  const th = { ...THEME, ...o.theme };
  const { tl: _tl, at, pos = [960, 800], w = 700, h = 90, bars = 32, seed = 5, speed = 7, beatKick = 0.3, beatsPerSec = 2, beatOffset = 0 } = o;
  const id = o.id ?? "wave";
  const rnd = rand(seed);
  const step = w / (bars - 1);
  const bw = Math.max(4, step * 0.38);
  const layers = Array.from({ length: bars }, (_, i) => {
    const u = i / (bars - 1);
    const env = 0.35 + 0.65 * Math.sin(Math.PI * u) ** 0.8;
    const p1 = r3(rnd.range(0, 6.28)), p2 = r3(rnd.range(0, 6.28));
    const t0 = r3(at + i * 0.012);
    const grow = `ease("expo.out", time, ${t0}, ${r3(t0 + 0.55)}, 0, 1)`;
    const level = `min(1, ${r3(env)} * (0.3 + 0.7 * abs(sin(time * ${speed} + ${p1}) * sin(time * ${r3(speed * 0.37)} + ${p2}))) + ${beatKick} * pow(1 - fract((time - ${beatOffset}) * ${beatsPerSec}), 3) * ${r3(env)})`;
    return shape([{ rect: [-bw / 2, -h, bw, h], radius: bw / 2, fill: o.color ?? th.turbo }], {
      id: `${id}-${i}`, pos: [r3(pos[0] - w / 2 + i * step), pos[1]], parent: o.parent as any, scale: [1, `${grow} * ${level}`] as any,
    });
  });
  return { layers, end: at + 0.55 + bars * 0.012 };
}
