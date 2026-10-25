// Relative tween values: `"+=40"` means 40 more than where the property is when the tween starts.
export type Val = number | string | number[];

const RELATIVE = /^([+-])=(-?\d+(?:\.\d+)?)$/;
export const isRelative = (v: unknown): v is string => typeof v === "string" && RELATIVE.test(v);

/** Resolve `"+=n"` / `"-=n"` (alone or per component of a vector) against `cur`; other values pass through. */
export function relative(v: Val, cur: Val | undefined, where: string): Val {
  const one = (x: unknown, base: unknown): unknown => {
    if (!isRelative(x)) return x;
    if (typeof base !== "number") throw new Error(`${where}: "${x}" is relative, but there is no number to start from; give the property a starting value`);
    const m = RELATIVE.exec(x)!;
    return base + (m[1] === "+" ? 1 : -1) * Number(m[2]);
  };
  if (Array.isArray(v)) return v.map((x, i) => one(x, Array.isArray(cur) ? cur[i] : cur) as number);
  return one(v, Array.isArray(cur) ? cur[0] : cur) as Val;
}
