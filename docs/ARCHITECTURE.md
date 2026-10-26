# turbomotion architecture

This document is the contract for where code lives and why. If a change does not fit it, change this file first.

## First principles

A motion tool has one job: turn *a description of what should happen over time* into pixels, **deterministically**.
Everything else is in service of three people: the author who writes the description, the renderer that draws it, and,
in this repository, an **agent** that writes descriptions and code and has to find out by itself whether the result is
good. Four principles follow.

1. **A frame is a pure function of time.** `frame(t) = draw(evaluate(plan, t))`. No state carried from the previous
   frame. This is what makes parallel segments, single-frame stills, scrubbing and golden tests possible, and it is the
   only reason an agent can re-render one moment after changing one number.
2. **Say it in data; write code only for new pixels.** The engine sees JSON: a frame is a pure function of that
   document. Authors write **Python or TypeScript** (a project's kit, the SDK) that emits the JSON. Code inside the renderer is needed
   only when the *look itself* is new (a new distortion, a new texture, a new generator), and that code is a small
   shader, not a Rust change.
3. **Verifiable without eyes, then with eyes.** The agent must be able to ask "where is the title at 1.2 s, and is it
   inside the safe area?" and get a number back (`inspect`, `check`, assertions), and only then look at a picture
   (`sheet`). Pictures are expensive and ambiguous; numbers are cheap and exact.
4. **Ownership follows change.** Things that change for the same reason live together. The platform changes when the
   GPU or the codec changes. A globe changes when a film needs a globe. They must never share a crate.

## Ownership

```
turbomotion (crates/cli)     the command line, nothing else
├── turbomotion-script       TypeScript authoring: strip, run on QuickJS, emit a composition; embeds the SDK
├── turbomotion-comp         compositions: schema → plan → eval → render, the editor's model of time
└── turbomotion-core         the platform; knows nothing about films, compositions or any domain
```

`core` ← `comp` ← `script` ← `cli`. Nothing in this repository knows about a film, a channel, a style or a brand.
Domain graphics (a globe, a chart, a generator of tiles) arrive from outside: as a **pack** registered by a host
binary (below), as **blocks** and **shader plugins** found on named search paths, or as JSON a project generates.

| Crate | Owns | Must not contain |
| --- | --- | --- |
| `core` | GPU context, encoder, segment-parallel scheduler, SkSL runner, half-float `Layer`, easing maths, the effects library, colour grade | fonts, timelines, scenes, anything named after a film |
| `comp` | the composition schema, time and property evaluation (keys, GSAP eases, expressions), the layer tree, masks and mattes, text animators, media, audio mix, 3D camera and lights, particles, shader plugins, `check`/`inspect`/`sheet`, the built-in blocks (`crates/comp/blocks/`) | film-specific elements, project paths, anything that finds files by walking up folders |
| `script` | TypeScript stripping, deterministic QuickJS execution, the SDK (`crates/script/sdk/`), the `sdk` CLI | rendering or text shaping (those are `comp`) |

Rule of thumb for a new feature: *would a project about something else want it?* Yes → `core` or `comp`. No → it
belongs in `motions/`, as a block, a plugin or a generator.

### Transitions are continuous by construction

A transition is a pair of per-layer modifiers (`eval/transit`): the outgoing and incoming shot read the same eased
progress, one as `h = e`, the other as `h = 1 - e`. A transition is smooth when the two sides agree on geometry and
speed at every instant, so each shot preset states an invariant and a test holds it: pushes and pans keep both shots one
strip; `cross_zoom` keeps a constant scale ratio (one exponential zoom, no jump in speed where one replaces the other);
`window` keeps the incoming card and the portal it opens in the outgoing shot the same rectangle. `timeline.mblur`
adds true sub-frame motion blur on top of any of them. Add a transition by writing its invariant first.

### Where the engine looks for files

An explicit list, in this order (a later entry replaces an earlier one of the same name):

| Kind | Order |
| --- | --- |
| blocks | built-ins → `TURBOMOTION_BLOCK_PATH` → the composition's `"block_path"` → its own `blocks/` |
| shader plugins | `TURBOMOTION_FX_PATH` → the composition's `"fx_path"` → its own `effects/` |
| SDK | `--sdk-dir`, otherwise the modules compiled into the binary |
| media, fonts | relative to the composition's `"root"` (default: its folder) |

A path that does not exist is an error naming the setting that listed it. There is no ancestor search and no lookup
beside the executable: a build must behave the same wherever the checkout is.

### Inside `comp`: one direction, four stages

```
schema/   what can be said          serde types, strict (unknown field = error with the valid names)
   ↓
plan/     what it means             load + probe assets, resolve time literals ("beat:8", "title.end+0.2"),
                                    parents, mattes, nested comps, fonts; reject the impossible, once, with a path
   ↓
eval/     what is true at time t    Plan × t → Frame: every layer's matrix, opacity, text-unit offsets, trim range,
                                    camera, lights. Pure numbers. No GPU. Serialisable: this is `inspect`.
   ↓
render/   how it looks              Frame → pixels on the GPU: painters per layer kind, masks, mattes, effects, blend
```

Each stage has its own tests: schema errors are strings, plan resolution is tables, evaluation is numbers, rendering
is goldens. A bug is found in the earliest stage that can express it.

#### The frame graph

`render/graph/` turns one composition frame into passes over named images: paint a layer, run its effects, build its
masks and mattes, multiply, wipe, composite, snapshot, adjust. A pass reads images and writes one; the builder walks
back from the picture that is wanted, so emission order is a topological order and a layer nobody reads (hidden, matte
source only, fully transparent) is never drawn. An image is dropped after its last reader, and a layer that is a matte,
a clip and an effect input at once is drawn once. The composite is the one image modified in place. A new construct
(shadows, a depth buffer, colour conversion) is a new `Stage` and the builder code that emits it, not surgery on a loop.
`turbomotion comp graph <comp.json> --t <seconds>` prints the passes, their reads and writes, and the most images
alive at once.

Effects that read their own past (`@input name history=N`) re-render the layer's chain at `t - k/fps` through the same
graph (`render/history.rs`): `out(t, 0) = input(t)`, `out(t, d) = effect(input(t), out(t-1/fps, d-1) ..)`, so a frame
is still a pure function of time and a seek equals playing from frame 0.

What that costs, with `N` history frames and recursion `D` deep. The cache (`Cache`, 512 MiB, oldest dropped first) keeps
every `(time, level)` picture, so playing forward adds per frame: one more render of the effect's input chain (the
frame drawn as `t` last time is drawn again as `t - 1/fps`, because the main render does not hand its inputs to the cache)
and `D - 1` runs of the effect's shader, whatever `N` is. The first frame after a seek has nothing cached and does
`N*D` input renders and `N*D*(D+1)/2` shader runs. A 1080p half-float picture is 16 MiB, so about 32 pictures fit; an
effect that needs more than that (`N*D` large, big frames) still draws the same pixels but recomputes what was evicted.

Supporting modules: `anim/` (eases, keyframes, expression language, noise), `media/` (image cache, ffmpeg video
decoder, probes), `audio/` (mix and audio-reactive signals), `fxlib/` (shader plugins and the built-in effect and
texture pack), `text/` (layout, per-character animators, named presets; a character the designed font does not have falls back to another installed face and is painted in the same colour as the rest of the line), `check/` (lint and assertions),
`schema/blocks.rs` (parametrised sub-compositions; built-ins in `crates/comp/blocks/<job>/`, more from `block_path`), `schema/tokens.rs` (palette, type roles, image treat, surface recipes), `pack.rs` (the Pack interface), `catalogue.rs`.

## How an agent makes an effect

There are four levels. The agent should always use the lowest one that works.

| Level | What the agent writes | Rebuild? | Example |
| --- | --- | --- | --- |
| 1. Compose | a `comp.json` using the catalogue | no | masked line reveal, parallax photo, kick-synced camera shake |
| 2. Template | a TypeScript function in a project's library (`motions/lib`) that returns layers, or block JSON on a `block_path` | no | `term_card(term, what, why, example)`, `scene_bug` |
| 3. New pixels | a shader plugin: `effects/<name>.sksl` + a manifest that types its uniforms | no | VHS tracking, halftone, caustics, any distortion |
| 4. New capability | Rust in `comp` or `core`, reviewed | yes | a new layer kind |

Level 3 is the one that matters for quality. A plugin is two small files:

```
effects/vhs.sksl      uniform float amount; uniform float time; half4 main(float2 xy) { ... }
effects/vhs.json      {"name": "vhs", "kind": "effect",
                       "uniforms": {"amount": {"type": "float", "default": 0.4, "min": 0, "max": 1, "animatable": true}}}
```

The renderer binds `time`, `resolution`, the input layer (`image`) and a standard prelude (hash, value/simplex/worley
noise, fbm, rotation, sdf shapes, luma, HSV, blend functions, film-grain, lens distortion) so the agent writes ten
lines, not two hundred. Skia reports compile errors with line and column; `check` surfaces them with the offending
line. `turbomotion comp fx test vhs --on photo.jpg` renders the plugin at 0, 25, 50, 75 and 100 % of each uniform's
range as one contact sheet, so the agent sees what the knob does before using it. From then on the effect is used
exactly like a built-in: `{"effect": "vhs", "amount": [[0, 0], [1, 0.8]]}`.

Generators (a texture, a background, a light leak) are the same thing with `"kind": "generator"`: no input layer, the
shader paints the whole layer.

The header of a plugin is its whole declaration (`fxlib::Descriptor`; platform effects produce one too, and the
reference pages are written from descriptors only). A platform effect declares its parameters once, in the `registry!` of
`crates/core/src/effects/mod.rs`: name, kind, default, range and doc. `Effect::build` checks a use against that
declaration (a stray name is an error that lists the valid ones, a missing one takes its default, a value is held in its
range as a plugin's is), and the descriptor is written from the same lines, so the struct, the plan and the reference
cannot disagree. Palette names (`"color": "accent"`) are looked up only in the parameters an effect declares as colours. Besides `@param`, it can declare `@param name choice default a,b,c`
(written as a name), `@input name layer` (another layer's picture, named by id in the use, fitted `center`, `stretch` or
`screen`: displacement maps, mattes), `@input name history=N depth=D` (the effect's own past: trails, datamosh),
`@pass name scale=S` (several passes; each sees the earlier ones by name, sampled bilinearly without mipmaps, so a pass far below half size aliases fine detail) and `@example`. Every `*.sksl` in
`crates/comp/src/fxlib/builtin/` is a shipped plugin, found by `build.rs`: adding one is adding a file.

Every effect instance, built-in or plugin, also takes a `composite` (`fxlib::composite`): its result is laid over its
input only inside a mask (a luma band or the edges of the input, its alpha, another layer), with a blend mode and an
amount. It is one implementation in `render/composite.rs`, so a shader never re-implements "keep the original".

## How an agent brings its own images

An image is the most common thing an agent will supply (generated, downloaded, cut out) and the easiest to make look
cheap. The platform therefore treats images as *assets with a pipeline*, not as file paths.

1. **Probe, once, at plan time.** Size, alpha, colour space, EXIF orientation, whether it is a cut-out. A path that does
   not exist, a 200 px image asked to fill 1080p, or a JPEG with an alpha expectation is reported by `check`, with a fix.
2. **Sample with quality.** Downscales use mipmaps with a cubic filter, never bilinear (bilinear shimmers on slow
   zooms); layers are composited in half-float so grades and glows do not band.
3. **Move the camera, not the picture.** An `image` layer has `fit` (`cover`/`contain`), a `focus` point so crops keep
   the subject, and a `camera` block (`from`/`to` zoom and centre, eased) which is a Ken Burns move with sub-pixel
   sampling and optional motion blur.
4. **Depth.** `parallax: {depth: "depth.png", strength: 0.04}` displaces by a depth map for a 2.5D move. The agent can
   supply the depth map (any monocular-depth model) or leave it out.
5. **Cut-outs.** A PNG with alpha or a separate `matte` image becomes a layer that can sit between text and
   background, cast a soft shadow and take a rim light.
6. **Make it belong.** `treat: ["grain", "grade:warm", "vignette"]` applies the same finishing to every image in a
   film; the cheapest way to make mixed sources look like one world.

## How the agent knows whether the result is good

The agent loop is write → `check` → `inspect` → `sheet` → fix. Each step answers a different question:

* **`check`**: is it *valid and sane*? Schema, unknown names with "did you mean", missing assets, shader compile
  errors, text wider than its box, layers that are never visible, elements outside the 90 % action-safe box, text
  below a contrast ratio, durations that are zero, cycles in parents, a matte that matters but is not found,
  a generator that paints past a `layout: backdrop` plate (`paints-past-plate`), anything a nested composition
  itself cuts off. `"clip_to": "plate"` keeps a layer inside another layer's alpha without hiding the plate
  (a track matte would). Nested comps smaller than the parent resolve their own type scale from *their* height,
  so a card does not inherit 140 px display type from a 1080p film.
* **`inspect --t 1.2 [--layer title]`**: what is *true* at that instant (position, scale, opacity, bounding box,
  which characters are visible). This is the stage-3 `Frame` printed as JSON.
* **assertions** (`comp.assert.json` beside the composition): `appearsBy`, `staysInFrame`, `before`, `keepsMoving`,
  checked on the evaluated frames. Intent becomes a test.
* **`sheet --times 0,0.5,1,...`**: a labelled contact sheet of stills, for the questions only eyes can answer.
* **determinism**: a frame redraws byte-identically; `check --determinism` proves it.

Measuring the agent system itself: the work-package gates in `docs/BUILD_PLAN.md` (section 8) are scored by the same machinery:
was `check` clean first time, how many iterations to a passing assertion set, does the final sheet match the checklist.

## Packs: domain graphics outside the platform

A globe or a map is a legitimate thing to put in a composition but it is not a composition feature. `comp` defines one
small interface:

```rust
pub trait Pack {
    fn kind(&self) -> &'static str;                                   // "studio.globe"
    fn paint(&self, ctx: &PackCtx, params: &serde_json::Value, t: f64) -> anyhow::Result<Layer>;
}
```

A layer `{"type": "pack:studio.globe", ...}` calls the registered pack. A host can implement `Pack` for its elements
without `comp` knowing they exist. Packs are registered by the binary. This keeps the globe where it belongs and still
lets one timeline contain it.

## What to build next

`docs/BUILD_PLAN.md` is the single plan: the ranked problems, the architecture decisions that extend this contract (effect
bounds and kernels, a per-pixel mesh pipeline, depth-composited 3D, property model v2, the colour pipeline, the parity ledger),
and every work package with its steps, files and tests. If a change there conflicts with this file, change this file first.
