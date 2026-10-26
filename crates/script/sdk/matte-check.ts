// An adjustment layer applies to what is below it, limited by its own `masks`. The engine does not read a `matte` on an
// adjustment layer, and a matte source that is hidden is drawn nowhere, so such a film would apply the adjustment to the
// whole frame without saying so. Refuse it where the film is written, naming the layers.

import type { Layer } from "./layers";

/** The id a `matte` names as its source: `matte: { source: "hole" }` (a layer object is accepted as its id). */
function matteSource(matte: unknown): string | undefined {
  const source = matte && typeof matte === "object" ? (matte as { source?: unknown }).source : undefined;
  if (typeof source === "string") return source;
  return source && typeof source === "object" && typeof (source as { id?: unknown }).id === "string" ? (source as { id: string }).id : undefined;
}

/** Throw when an adjustment layer takes a matte from a layer in the same list that is `hidden`. */
export function checkAdjustmentMattes(layers: Layer[], where: string): void {
  const byId = new Map(layers.map((l) => [l.id, l]));
  for (const l of layers) {
    if (l.kind !== "adjustment" || l.props.matte === undefined) continue;
    const id = matteSource(l.props.matte);
    if (id !== undefined && byId.get(id)?.props.hidden === true) {
      throw new Error(`${where}: adjustment layer "${l.id}" takes its matte from "${id}", which is hidden, so nothing would limit the adjustment and it would cover the whole frame. The engine does not apply a matte to an adjustment layer: limit it with its own "masks" (adjustment layer { masks: [{ path: ... }] }), or put the effect on a layer of its own that can take a matte`);
    }
  }
}
