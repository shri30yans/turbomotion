// Small deterministic helpers: seeded random numbers, ranges, interpolation, grids.

export function range(n: number): number[];
export function range(start: number, end: number, step?: number): number[];
export function range(a: number, b?: number, step = 1): number[] {
  const [start, end] = b === undefined ? [0, a] : [a, b];
  if (step === 0) throw new Error("range(): step must not be 0");
  const out: number[] = [];
  for (let v = start; step > 0 ? v < end : v > end; v += step) out.push(v);
  return out;
}

export const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;
export const clamp = (v: number, lo: number, hi: number): number => Math.min(hi, Math.max(lo, v));
export const deg = (rad: number): number => (rad * 180) / Math.PI;

export interface Rand {
  (): number;
  range(lo: number, hi: number): number;
  int(n: number): number;
  pick<T>(items: readonly T[]): T;
  shuffle<T>(items: readonly T[]): T[];
}

/** A seeded generator (mulberry32). The same seed always gives the same numbers, on every machine. */
export function rand(seed: number): Rand {
  let s = seed >>> 0;
  const next = (): number => {
    s = (s + 0x6d2b79f5) | 0;
    let t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  const r = next as Rand;
  r.range = (lo, hi) => lo + (hi - lo) * next();
  r.int = (n) => Math.floor(next() * n);
  r.pick = (items) => {
    if (!items.length) throw new Error("rand.pick(): the list is empty");
    return items[Math.floor(next() * items.length)];
  };
  r.shuffle = (items) => {
    const a = [...items];
    for (let i = a.length - 1; i > 0; i--) {
      const j = Math.floor(next() * (i + 1));
      [a[i], a[j]] = [a[j], a[i]];
    }
    return a;
  };
  return r;
}

export interface Cell {
  i: number;
  col: number;
  row: number;
  /** Centre of the cell, with the whole grid centred on 0,0. */
  x: number;
  y: number;
}

/** `cols` x `rows` cells of size `cell` (a number, or [width, height]), row by row, centred on the origin. */
export function grid(cols: number, rows: number, cell: number | [number, number]): Cell[] {
  const [cw, ch] = typeof cell === "number" ? [cell, cell] : cell;
  const out: Cell[] = [];
  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      out.push({ i: row * cols + col, col, row, x: (col - (cols - 1) / 2) * cw, y: (row - (rows - 1) / 2) * ch });
    }
  }
  return out;
}

/** Darken a `#rrggbb` colour by `amount` (0..1); other colour forms come back unchanged. */
export function darken(color: string, amount = 0.55): string {
  const m = /^#([0-9a-f]{6})$/i.exec(color);
  if (!m) return color;
  const n = parseInt(m[1], 16);
  const c = [n >> 16, (n >> 8) & 255, n & 255].map((v) => Math.round(v * (1 - amount)));
  return "#" + c.map((v) => v.toString(16).padStart(2, "0")).join("");
}
