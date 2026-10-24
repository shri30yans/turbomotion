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
2. **Say it in data; write code only for new pixels.** Almost everything an editor does (cut, move, fade, mask, split a
   title into letters, stagger, shake the camera) is *composition*, and composition is JSON. Code is needed only when
   the *look itself* is new (a new distortion, a new texture, a new generator), and that code is a small shader, not a
   Rust change.
3. **Verifiable without eyes, then with eyes.** The agent must be able to ask "where is the title at 1.2 s, and is it
   inside the safe area?" and get a number back (`inspect`, `check`, assertions), and only then look at a picture
   (`sheet`). Pictures are expensive and ambiguous; numbers are cheap and exact.
4. **Ownership follows change.** Things that change for the same reason live together. The platform changes when the
   GPU or the codec changes. A globe changes when a film needs a globe. They must never share a crate.

## Ownership

```
turbomotion (bin)            the command line, nothing else
├── turbomotion-comp         compositions: schema → plan → eval → render, the editor's model of time
├── turbomotion-studio       compiled word-anchored films; owns globe, maps, charts, photo plates, panels
├── turbomotion-shots        the original shot renderer; owns its shots, paper, fonts, assets
└── turbomotion-core         the platform; knows nothing about films, shots or compositions
```

`core` ← everything else. Nothing depends on `studio` or `shots` except the binary. `comp` never imports a film
element; if a film needs to appear inside a composition it is offered through the `Pack` interface (below).

| Crate | Owns | Must not contain |
| --- | --- | --- |
| `core` | GPU context, encoder, segment-parallel scheduler, SkSL runner, half-float `Layer`, easing maths, the effects library, colour grade | fonts, timelines, scenes, maps, globe, charts, anything named after a film |
| `comp` | the composition schema, time and property evaluation (keys, GSAP eases, expressions), the layer tree, masks and mattes, text animators, media, audio mix, 3D camera and lights, particles, user shader plugins, `check`/`inspect`/`sheet` | film-specific elements, Python-parity quirks |
| `studio` | the compiled-film model, film elements, the 17 film transitions, parity helpers (`rng`, `pyfmt`), geography | generic effects (they are in `core`), composition features |
| `shots` | shot types, paper, pins, its own fonts and assets | anything `studio` or `comp` could share |

Rule of thumb for a new feature: *would a film about something else want it?* Yes → `core` or `comp`. No → the crate
of the film it serves.

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

Supporting modules: `anim/` (eases, keyframes, expression language, noise), `media/` (image cache, ffmpeg video
decoder, probes), `audio/` (mix and audio-reactive signals), `fxlib/` (shader plugins and the built-in effect and
texture pack), `text/` (layout, per-character animators, named presets), `check/` (lint and assertions),
`schema/blocks.rs` (parametrised sub-compositions), `pack.rs` (the Pack interface), `catalogue.rs`.

## How an agent makes an effect

There are four levels. The agent should always use the lowest one that works.

| Level | What the agent writes | Rebuild? | Example |
| --- | --- | --- | --- |
| 1. Compose | a `comp.json` using the catalogue | no | masked line reveal, parallax photo, kick-synced camera shake |
| 2. Template | a parametrised sub-composition ("block") with declared variables, stored in `blocks/` | no | lower third with `{name, role}`, stat counter |
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
  below a contrast ratio, durations that are zero, cycles in parents, a matte that matters but is not found.
* **`inspect --t 1.2 [--layer title]`**: what is *true* at that instant (position, scale, opacity, bounding box,
  which characters are visible). This is the stage-3 `Frame` printed as JSON.
* **assertions** (`comp.assert.json` beside the composition): `appearsBy`, `staysInFrame`, `before`, `keepsMoving`,
  checked on the evaluated frames. Intent becomes a test.
* **`sheet --times 0,0.5,1,...`**: a labelled contact sheet of stills, for the questions only eyes can answer.
* **determinism**: a frame redraws byte-identically; `check --determinism` proves it.

Measuring the agent system itself (not the renderer): `bench/briefs/*.md` are written briefs ("a masked line reveal
that lands on the beat", "a photo with parallax and grain", "a new VHS effect"). A run is scored by the same
machinery: did `check` pass first time, how many iterations to a passing assertion set, did the final sheet match the
brief's checklist. A good system makes the first draft valid and the third draft good; if agents need ten
iterations, the catalogue, the error messages or the defaults are wrong, not the agent.

## Packs: domain graphics outside the platform

A globe or a map is a legitimate thing to put in a composition but it is not a composition feature. `comp` defines one
small interface:

```rust
pub trait Pack {
    fn kind(&self) -> &'static str;                                   // "studio.globe"
    fn paint(&self, ctx: &PackCtx, params: &serde_json::Value, t: f64) -> anyhow::Result<Layer>;
}
```

A layer `{"type": "pack:studio.globe", ...}` calls the registered pack. `studio` can implement `Pack` for its elements
without `comp` knowing they exist. Packs are registered by the binary. This keeps the globe where it belongs and still
lets one timeline contain it.

## What else to build, in order of leverage

1. The four-stage spine with `inspect` and `check` (the agent's eyes).
2. Shader plugins with the prelude and `fx test` (the agent's hands for new looks).
3. Image pipeline: probe, mipmapped sampling, Ken Burns, parallax, treat (the agent's own media).
4. Text engine: per-character animators, GSAP-style staggers, masked line reveals, scramble, counters.
5. Blocks library with thumbnails (reuse beats invention).
6. Audio: mix graph, beat/onset detection, `beat:N` time literals (fast-paced editing lives on the beat grid).
7. Motion blur by temporal sampling, 3D camera and lights, particles.
8. Alpha export, vertical presets, SVG and Lottie import.
9. A watch mode that re-renders the current still on every save, and later a GUI over the same `Frame`.
10. A second GPU backend behind `core::gpu` (Metal is the only one today and is the only place that names it).
