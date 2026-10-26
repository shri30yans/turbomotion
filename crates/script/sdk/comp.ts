// The film: size, frame rate, layers, and the edit timeline of shots, transitions, overlays and hits.

import { Layer, describe, flatten } from "./layers";
import { configureMeasure } from "./measure";
import { inShot } from "./sound";
import type { CompOpts, OverlayOpts, ShotOpts, TimelineOpts } from "./types";

/** A shot of the edit timeline, built by `shot()`. */
export class Shot {
  constructor(readonly opts: ShotOpts) {
    flatten(opts?.layers, `shot "${opts?.id}" layers`);
    if (!opts || typeof opts.id !== "string" || !opts.id) throw new Error('shot({ id, dur, layers }): every shot needs an "id"');
    if (!(opts.dur > 0)) throw new Error(`shot "${opts.id}": "dur" must be a number of seconds above 0, got ${describe(opts.dur)}`);
  }

  toJSON(): Record<string, unknown> {
    const { layers, extra, hits: _hits, ...rest } = this.opts;
    const out: Record<string, unknown> = { ...rest, ...(extra ?? {}) };
    const flat = flatten(layers, `shot "${this.opts.id}" layers`);
    checkIds(flat, `shot "${this.opts.id}"`);
    if (flat.length) out.layers = flat;
    return out;
  }
}

/** One shot of the edit: its layers, how it comes in (`in`), a 2D `move`, a 3D `camera` move. */
export const shot = (opts: ShotOpts): Shot => new Shot(opts);

function checkIds(layers: Layer[], where: string): void {
  const seen = new Set<string>();
  for (const l of layers) {
    if (seen.has(l.id)) throw new Error(`${where}: two layers share the id "${l.id}"; give one of them a different id`);
    seen.add(l.id);
  }
  for (const l of layers) {
    const p = l.props.parent;
    if (typeof p === "string" && !seen.has(p)) throw new Error(`${where}: layer "${l.id}" has parent "${p}", which is not in the same list`);
  }
}

/** A composition being built: add layers, shots, audio and markers, and it becomes the JSON the engine renders. */
export class Film {
  private items: unknown[] = [];
  private edit?: TimelineOpts;

  constructor(readonly opts: CompOpts) {
    for (const k of ["width", "height", "fps"] as const) {
      if (!(typeof opts?.[k] === "number" && opts[k] > 0)) throw new Error(`comp({ width, height, fps }): "${k}" must be a positive number, got ${describe(opts?.[k])}`);
    }
    configureMeasure(opts.fonts, opts.root);
    // Film options are read once, when comp({...}) is called. A property assigned afterwards would be written nowhere,
    // so it is refused where it is written, naming the place it belongs.
    return new Proxy(this, {
      set(film, key, value, receiver) {
        if (key !== "edit") {
          throw new Error(`film.${String(key)} = ... has no effect: the film's options are fixed by comp({ ... }); put it there, like comp({ width, height, fps, ${String(key)}: ... })`);
        }
        return Reflect.set(film, key, value, receiver);
      },
    });
  }

  /** Add layers (or arrays of layers, or groups) to the film's own layer list, bottom first. */
  add(...layers: unknown[]): this {
    flatten(layers, "film.add()"); // fail here, at the line that passed the wrong thing
    this.items.push(...layers);
    return this;
  }

  /** Write the film as shots with transitions, overlays and sound hits. */
  timeline(edit: TimelineOpts): this {
    if (!edit || !Array.isArray(edit.shots) || !edit.shots.length) throw new Error("film.timeline({ shots: [...] }): give at least one shot() made with shot({ id, dur, layers })");
    const ids = new Set<string>();
    for (const s of edit.shots) {
      if (!(s instanceof Shot)) throw new Error(`film.timeline(): shots must come from shot({...}), got ${describe(s)}`);
      if (ids.has(s.opts.id)) throw new Error(`film.timeline(): two shots share the id "${s.opts.id}"`);
      ids.add(s.opts.id);
    }
    for (const o of edit.overlays ?? []) if (!ids.has(o.shot)) throw new Error(`overlay refers to shot "${o.shot}", which does not exist (shots: ${[...ids].join(", ")})`);
    this.edit = edit;
    return this;
  }

  toJSON(): Record<string, unknown> {
    const { extra, ...rest } = this.opts;
    const out: Record<string, unknown> = { ...rest, ...(extra ?? {}) };
    const layers = flatten(this.items, "film.add()");
    checkIds(layers, "the film");
    if (layers.length) out.layers = layers;
    if (this.edit) {
      const { overlays, shots, hits, ...tl } = this.edit;
      // shot-local hits become "<id>.start+<t>" literals, which the engine resolves once transitions have overlapped the shots
      const all = [...(hits ?? []), ...(shots as Shot[]).flatMap((s) => inShot(s.opts.id, s.opts.hits))];
      out.timeline = { ...tl, shots, ...(overlays ? { overlays: overlays.map(overlay) } : {}), ...(all.length ? { hits: all } : {}) };
    }
    if (!out.layers && !out.timeline) throw new Error("the film is empty: add layers with film.add(...) or shots with film.timeline({ shots })");
    return out;
  }
}

function overlay(o: OverlayOpts): Record<string, unknown> {
  const { layer, ...place } = o;
  const layers = flatten(layer, `overlay on "${o.shot}"`);
  if (layers.length !== 1) throw new Error(`overlay on "${o.shot}": give exactly one layer (use a precomp or block for more)`);
  return { ...place, ...layers[0].toJSON() };
}

/** The film: `comp({ width, height, fps, bg, ... })`, then `film.add(layers)`, `film.shot(...)` and `export default film`. */
export const comp = (opts: CompOpts): Film => new Film(opts);
