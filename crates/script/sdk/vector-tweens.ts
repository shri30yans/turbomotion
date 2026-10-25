// The layer's three-dimensional vectors: After Effects' X, Y and Z Rotation, its X, Y and Z Orientation, and the
// point its face turns toward. The engine holds each as a vector of three components, so a tween of one of them
// becomes a tween per component, and the components are put back together when the layer is written out.

import type { Val } from "./relative";

/** The vector properties, and how many components each has. */
export const VECTORS: Record<string, number> = { rotation: 3, orientation: 3, aim: 3 };

/** The tween key for component `index` of `name`: `rotation.1`. */
export const vectorKey = (name: string, index: number): string => `${name}.${index}`;

/** Is this tween key one component of a vector property? */
export const isVectorKey = (key: string): boolean => {
  const dot = key.indexOf(".");
  if (dot < 0) return false;
  const name = key.slice(0, dot);
  const index = key.slice(dot + 1);
  return name in VECTORS && /^\d+$/.test(index) && Number(index) < VECTORS[name];
};

/** The vector property and the component a tween key names. */
export const parseVectorKey = (key: string): { name: string; index: number } => {
  const dot = key.indexOf(".");
  return { name: key.slice(0, dot), index: Number(key.slice(dot + 1)) };
};

/**
 * Split a vector tween into one entry per component.
 *
 * `rotation` is the one vector with a shorthand: a bare number is the Z Rotation, the turn in the layer's own
 * plane. The other vectors are always a triple.
 */
export function flattenVector(name: string, value: unknown, where: string): Record<string, Val> {
  const count = VECTORS[name];
  if (!Array.isArray(value)) {
    if (name !== "rotation") throw new Error(`${where}: "${name}" is a vector of ${count} numbers: give [x, y, z]`);
    return { [vectorKey(name, count - 1)]: value as Val };
  }
  if (value.length !== count) throw new Error(`${where}: "${name}" takes ${count} numbers, got ${value.length}`);
  const out: Record<string, Val> = {};
  value.forEach((part, index) => {
    if (part === null || part === undefined) return;
    out[vectorKey(name, index)] = part as Val;
  });
  return out;
}

/** The value component `index` of the vector `name` starts from. */
export function vectorBase(out: Record<string, any>, name: string, index: number, id: string): Val {
  const value = out[name];
  if (value === undefined) return 0;
  if (typeof value === "number") {
    // a lone number is the Z Rotation; the other components start at nothing turned
    if (index !== VECTORS[name] - 1) return 0;
    return value;
  }
  if (Array.isArray(value)) {
    const part = value[index];
    if (part === undefined) return 0;
    if (typeof part === "number") return part;
  }
  throw new Error(`layer "${id}": "${name}" is already animated or an expression; tween it from the timeline only`);
}

/**
 * The components of the vector `name` as the layer holds them now: a lone number or expression is the last one (the
 * Z Rotation), and an expression in an axis stays an expression instead of turning into 0.
 */
export function vectorComponents(out: Record<string, any>, name: string): Array<number | string> {
  const count = VECTORS[name];
  const current = out[name];
  const zero = new Array<number | string>(count).fill(0);
  if (typeof current === "number" || typeof current === "string") return [...zero.slice(0, count - 1), current];
  if (!Array.isArray(current)) return zero;
  return current.map((n) => (typeof n === "number" || typeof n === "string" ? n : 0));
}

/** Write the tweened components of `name` back into the layer, keeping the components it does not touch. */
export function setVector(out: Record<string, any>, name: string, parts: Map<number, unknown>): void {
  const count = VECTORS[name];
  const comps = vectorComponents(out, name);
  for (const [index, value] of parts) comps[index] = value;
  out[name] = comps;
}

/** Fill the holes of a partial vector against the layer's current components: a hole leaves that axis alone. */
export function fillVector(base: Array<number | string>, value: Array<number | string | null | undefined>): Array<number | string> {
  return value.map((part, index) => (part === null || part === undefined ? base[index] ?? 0 : part));
}

/** The orientation and the point of interest are vectors of three; a mistake here is worth naming. */
export function checkVectors(out: Record<string, any>, id: string): void {
  for (const name of ["rotation", "orientation", "aim"]) {
    const value = out[name];
    if (value === undefined) continue;
    // a lone number or expression is the Z Rotation, the turn in the layer's own plane
    if (name === "rotation" && (typeof value === "number" || typeof value === "string")) continue;
    if (typeof value === "number" || typeof value === "string") throw new Error(`layer "${id}": "${name}" is a vector of three numbers: [x, y, z]`);
    if (!Array.isArray(value)) continue;
    if (value.length !== 3) throw new Error(`layer "${id}": "${name}" is a vector of three numbers: [x, y, z], got ${value.length}`);
    if (value.some((n) => n === null || n === undefined)) throw new Error(`layer "${id}": "${name}" leaves an axis empty; give it all three numbers`);
  }
}
