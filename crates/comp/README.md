# turbomotion-comp

An After Effects-style composition engine on Skia/Metal. A composition is one JSON file; a frame is a pure function of
time; an agent can write it, check it with numbers, look at it, and extend it with a single shader file.

```
turbomotion comp catalogue                     everything a composition can contain, with a working example of each
turbomotion comp check comp.json               validate, lint, run comp.assert.json, (--determinism) prove frames repeat
turbomotion comp inspect comp.json --t 1.2     what is true at 1.2 s: every layer's bounds, opacity, glyphs, lights
turbomotion comp sheet comp.json --out s.png   a labelled contact sheet of moments (eyes, after the numbers)
turbomotion comp still comp.json --out dir --times 0.5,2
turbomotion comp render comp.json --out out.mp4   segment-parallel; audio is mixed and muxed
turbomotion comp fx list | fx test <name> --out sheet.png      shader plugins and their knobs
turbomotion comp blocks                        parametrised sub-compositions
```

## What a composition contains

Layers, bottom first. Kinds: `solid`, `shape` (rect, ellipse, polygon, star, SVG path; fill, gradient, stroke, dashes,
trim, morph, repeat), `text` (per-character animators, presets, counters, scramble), `image` (Ken Burns, depth
parallax, EXIF-correct, mipmapped), `video` (streamed through ffmpeg), `audio`, `precomp` (nested compositions and
blocks), `null`, `camera`, `light`, `adjustment`, `particles`, `texture` (generator shaders), `pack` (graphics owned
by another crate). Every layer also takes transform, `parent`, `masks`, `matte` (alpha/luma, inverted), `fx`,
`blend`, `enter`/`exit` presets, `along` (motion path), `three_d` with a `material`, and `mblur`.

Anything animated is a *property*: a number, an expression string, keys `[[t, value, ease?]]`, a GSAP-style tween
`{"from", "to", "at", "dur", "ease"}`, a loop, or keys plus an expression. Times can be `1.5s`, `45f`, `beat:8`,
`bar:2`, `marker:drop`, `title.end+0.2`, `prev.end`. Variables are `$name`. Expressions have `wiggle`, `noise`,
`ref("id", "prop")`, `beat()`, `amp("audio-id")`, `onset("audio-id")` (audio-reactive), and the layer's `time`,
`index`, `progress`.

## Working like an agent

1. `catalogue` for the vocabulary. Write `comp.json`; reuse a block (`{"type": "precomp", "block": "lower_third",
   "vars": {...}}`) before inventing.
2. `check` — schema errors name the valid keys; lint catches off-screen or unsafe text, low contrast, upscaled photos,
   text clipped by its nested comp, dead layers; `comp.assert.json` turns intent into tests (`visible_by`,
   `stays_in_frame`, `before`, `moves`, `no_overlap`, `hidden_at`, `keeps_moving`).
3. `inspect --t` for exact positions. `sheet` for a look.
4. A new look? Write `effects/<name>.sksl` (below); `fx test <name>` sweeps each knob across its range.

### A shader plugin is one file

```glsl
// @effect duotone
// @summary Maps brightness onto two colours.
// @param shadow color #101030 -- colour of the darks
// @param amount float 1 0 1 -- 0 keeps the original, 1 is fully mapped
half4 main(float2 xy) {
    float4 c = straight(xy);
    return premul(mix(c.rgb, mix(shadow.rgb, #ffd6a0, luma(c.rgb)), amount), c.a);
}
```

You get `image` (effects), `resolution`, `time` and a standard library (`hash21`, `vnoise`, `fbm`, `worley`,
`worley12`, `rot2`, `luma`, `hsv2rgb`, `sdBox`, `straight`, `premul`). Parameters are bound by name; compile errors
come back with *your* line number and the line itself. `@kind generator` makes a texture layer instead of an effect.
The pack ships halftone, duotone, posterize, kaleidoscope, glitch_slices, vhs, light_sweep, ripple, mirror and the
generators noise, clouds, cells, stripes, grid, gradient, light_leak.

## Ownership

`comp` depends only on `turbomotion-core`. It knows no film element; graphics from other crates arrive through
[`pack::Pack`](src/pack.rs) (the studio's globe, maps and charts as `studio.element`). See
[../../docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md).

## Layout

```
schema/    what can be said (strict serde, blocks, time literals)      plan/    what it means (resolved names, probed media)
eval/      what is true at t (pure numbers; `inspect`)                 render/  how it looks (Skia/Metal; masks, mattes, fx)
anim/      eases, properties, expressions, noise                       text/    layout, per-character animators, presets
fxlib/     shader plugins and the built-in pack                        media/   image cache, ffmpeg video, probes
audio/     mix and analysis signals                                    check/   lint and assertions
pack.rs    the host-registered graphics interface                      blocks/  built-in parametrised compositions
```

Examples that render: `examples/comp/01-shapes` … `08-packs`. Tests: `cargo test -p turbomotion-comp`.
