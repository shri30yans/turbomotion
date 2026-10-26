// A property that can be one number or a list of them (`scale: 2` or `scale: [2, 1]`) must have one shape on every key.
// A scalar tween against a per-axis value is read as "the same on every axis", so the two always merge.

import type { Val } from "./relative";
import type { Tween } from "./timeline";

/** The properties with a per-axis form. */
const PER_AXIS = ["scale"];

const lengthOf = (v: Val | undefined): number | undefined => (Array.isArray(v) ? v.length : undefined);

const spread = (v: Val | undefined, n: number): Val | undefined => (v === undefined || Array.isArray(v) ? v : (Array<number | string>(n).fill(v) as Val));

/** `base` and the tweens of `prop` with every value written for the same number of axes. */
export function matchAxes(prop: string, tweens: Tween[], base: Val | undefined, id: string): { tweens: Tween[]; base: Val | undefined } {
  if (!PER_AXIS.includes(prop)) return { tweens, base };
  const counts = new Set([base, ...tweens.flatMap((t) => [t.from, t.to])].map(lengthOf).filter((n): n is number => n !== undefined));
  if (counts.size > 1) throw new Error(`layer "${id}": "${prop}" is animated with lists of different lengths (${[...counts].join(" and ")}); use one length`);
  const [axes] = counts;
  if (axes === undefined) return { tweens, base };
  return { tweens: tweens.map((t) => ({ ...t, from: spread(t.from, axes), to: spread(t.to, axes) })), base: spread(base, axes) };
}
