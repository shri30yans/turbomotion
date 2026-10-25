// Tweening the items of a shape layer: `tl.to(ring, { items: { 0: { trim_end: 1, stroke_width: 6 } } })`. The engine
// animates an item's trim, offset, rotation, scale, opacity, stroke and fill; a rect's size and corner radius are
// plain numbers in the schema, so animate the item's `scale` (about its `anchor`) instead.
import type { Val } from "./relative";

/** Item property -> where it lives in the item JSON, and its value when the item does not set it. */
const ITEM: Record<string, { path: string[]; rest?: Val }> = {
  trim_start: { path: ["trim", "start"], rest: 0 },
  trim_end: { path: ["trim", "end"], rest: 1 },
  trim_offset: { path: ["trim", "offset"], rest: 0 },
  offset: { path: ["offset"], rest: [0, 0] },
  rotation: { path: ["rotation"], rest: 0 },
  scale: { path: ["scale"], rest: 1 },
  opacity: { path: ["opacity"], rest: 1 },
  morph: { path: ["morph"], rest: 0 },
  stroke_width: { path: ["stroke", "width"] },
  stroke_opacity: { path: ["stroke", "opacity"], rest: 1 },
  dash_offset: { path: ["stroke", "dash_offset"], rest: 0 },
  stroke_color: { path: ["stroke", "color"] },
  fill_opacity: { path: ["fill", "opacity"], rest: 1 },
  fill_color: { path: ["fill", "color"] },
};
export const ITEM_PROPS = Object.keys(ITEM);
const FIXED = ["rect", "radius", "width", "height", "ellipse", "d"];

/** The tween key for item `index`'s property: `items.2.trim_end`. */
export const itemKey = (index: number | string, name: string): string => `items.${index}.${name}`;

export const isItemKey = (key: string): boolean => key.startsWith("items.");

function parse(key: string): { index: number; name: string } {
  const [, index, name] = key.split(".");
  return { index: Number(index), name };
}

/** Flatten `{ 0: { trim_end: 1 } }` into `{ "items.0.trim_end": 1 }`, rejecting what the schema cannot animate. */
export function flattenItems(items: unknown, where: string): Record<string, Val> {
  if (typeof items !== "object" || items === null || Array.isArray(items)) throw new Error(`${where}: items must be an object keyed by item number, like { 0: { trim_end: 1 } }`);
  const out: Record<string, Val> = {};
  for (const [index, vars] of Object.entries(items)) {
    if (!/^\d+$/.test(index)) throw new Error(`${where}: item keys are numbers (the item's place in the shape), got "${index}"`);
    for (const [name, value] of Object.entries(vars as Record<string, Val>)) {
      if (FIXED.includes(name)) throw new Error(`${where}: an item's "${name}" is fixed in the schema and cannot be tweened; animate the item's scale (about its anchor) or its trim instead`);
      if (!ITEM[name]) throw new Error(`${where}: "${name}" is not an animatable item property; animatable: ${ITEM_PROPS.join(", ")}`);
      out[itemKey(index, name)] = value;
    }
  }
  return out;
}

/** What item `key` holds now in the layer JSON (`out.items`), or its resting value; undefined when it has neither. */
export function itemBase(out: Record<string, any>, key: string, id: string): Val | undefined {
  const { index, name } = parse(key);
  const item = out.items?.[index];
  if (!item) throw new Error(`layer "${id}": it has no item ${index} to animate (${out.items?.length ?? 0} items)`);
  const { path, rest } = ITEM[name];
  let v: any = item;
  for (const step of path) v = v !== null && typeof v === "object" && !Array.isArray(v) ? v[step] : undefined;
  // a bare colour is the item's whole fill
  if (v === undefined && name === "fill_color" && (typeof item.fill === "string" || Array.isArray(item.fill))) v = item.fill;
  return v === undefined ? rest : v;
}

/** Write `value` at the item property, copying along the way so the layer's own items stay untouched. */
export function setItem(out: Record<string, any>, key: string, value: unknown): void {
  const { index, name } = parse(key);
  const items = [...out.items];
  let item = { ...items[index] };
  const [head, ...rest] = ITEM[name].path;
  if (head === "fill" && (typeof item.fill === "string" || Array.isArray(item.fill))) item.fill = { color: item.fill };
  let cursor: Record<string, any> = item;
  for (const step of [head, ...rest].slice(0, -1)) cursor = cursor[step] = { ...(cursor[step] ?? {}) };
  cursor[[head, ...rest].slice(-1)[0]] = value;
  items[index] = item;
  out.items = items;
}
