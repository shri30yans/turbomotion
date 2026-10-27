# turbomotion-comp

An After Effects-style composition engine on Skia/Metal. A composition is one JSON file; a frame is a pure function of
time; an agent can write it, check it with numbers, look at it, and extend it with a single shader file.

```
turbomotion comp catalogue                     everything a composition can contain, with a working example of each
turbomotion comp check comp.json               validate, lint, run comp.assert.json, (--determinism) prove frames repeat
turbomotion comp inspect comp.json --t 1.2     what is true at 1.2 s: every layer's bounds, opacity, glyphs, lights
turbomotion comp sheet comp.json --out s.png   a labelled contact sheet of moments (eyes, after the numbers)
turbomotion comp sheet comp.json --out s.png --ref ref.mp4 [--crop x,y,w,h]   the reference frame beside ours at each time
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
`blend`, `enter`/`exit` presets, `along` (motion path), `three_d` with a `material`, and `mblur` (the layer's motion blur switch; the composition's `motion_blur` sets the shutter).

Anything animated is a *property*: a number, an expression string, keys `[[t, value, ease?]]`, a GSAP-style tween
`{"from", "to", "at", "dur", "ease"}`, a loop, or keys plus an expression. Times can be `1.5s`, `45f`, `beat:8`,
`bar:2`, `marker:drop`, `title.end+0.2`, `prev.end`. Variables are `$name`. Expressions have `wiggle`, `noise`,
`ref("id", "prop")`, `beat()`, `amp("audio-id")`, `onset("audio-id")` (audio-reactive), and the layer's `time`,
`index`, `progress`.

Image layers accept `"sampling": "nearest" | "linear"`. Omit it for automatic cubic enlargement and mipmapped
reduction. Precomp layers accept the same field and default to linear filtering. For pixel art inside a scaled
precomp, set `sampling: "nearest"` on both the image and its precomp, then use integer scale and aligned positions.

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
// @param light color #ffd6a0 -- colour of the lights
// @param amount float 1 0 1 -- 0 keeps the original, 1 is fully mapped
half4 main(float2 xy) {
    float4 c = straight(xy);
    return premul(mix(c.rgb, mix(shadow.rgb, light.rgb, luma(c.rgb)), amount), c.a);
}
```

You get `image` (effects), `resolution`, `time` and a standard library (`hash21`, `vnoise`, `fbm`, `worley`,
`worley12`, `rot2`, `luma`, `hsv2rgb`, `sdBox`, `straight`, `premul`). Parameters are bound by name; compile errors
come back with *your* line number and the line itself. `@kind generator` makes a texture layer instead of an effect.
The pack ships looks (halftone, duotone, vhs, holo, light_sweep), generators (noise, clouds, bokeh, floor), hits
(`flash`, `lightning`, `bolt`, `shockwave`, `impact`), optical (`anamorphic`, `shafts`, `haze`, `rain`, `prism`,
`barrel`, `bleach`, `chroma_hold`) and archive (`dropout`, `flicker`, `gate_weave`, `interference`, `static`,
`film_burn`). `comp fx test <name>` sweeps each knob.

## The edit timeline

A root `"timeline"` writes a film as tracks: `shots` (each a comp, layers, a picture or a clip) with a transition
`in` to each, `overlays` placed by shot, `hits` for sound. It expands into ordinary layers before anything else is
read, so `inspect`, `check` and the layout gate see every transition. 29 transitions (push, slide over, whip pan,
pan out to the next shot, zoom through, flood, iris, wipes, dips, glitch) move the outgoing and incoming shots as
one strip and carry a kit sound timed to their midpoint; `speed` keys become a time remap (ramps, freezes); `move`
and `camera` give 2D frame moves and 3D camera moves (orbit, dolly zoom, crane, truck). `mesh` is the one 3D object layer: its `geometry` is `text`, `box`, `sphere`, `torus`, `cylinder` or `extrude`, and `rotation` turns it
about its middle (`"rotation": [0, "time*90", 0]` is a turntable; `examples/comp/19-solids`). `extrude` on a text layer gives flat 3D-ish depth. `comp catalogue | jq .timeline` lists it all; `examples/comp/16-timeline` uses every family.

## Tools that rewrite or produce composition JSON

| Command | What it does |
| --- | --- |
| `comp svg art.svg --width 400 --out art.layer.json` | SVG to a shape layer (`path`, `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon`, `g`, fill/stroke/opacity, `viewBox`, `translate`). Skipped elements are printed as warnings, never dropped silently. Paste the layer into `layers`; animate it with `trim`, `fill`, `morph_to`. |
| `comp reframe comp.json --out vertical.json --size 1080x1920 --mode cover\|contain --focus 0.5` | Wraps a finished composition for another aspect ratio. `cover` crops to fill (follow the subject with `--focus`); `contain` fits the picture over a blurred, darkened copy. The source is never edited; keep the output beside it so relative assets resolve. |
| `comp watch comp.json --out sheet.png` | Re-checks and re-draws the contact sheet whenever the file is saved; a half-typed edit prints its error and the loop keeps going. `--once` does a single pass. |
| `comp render comp.json --out lower_third.mov --codec alpha` | ProRes 4444 with transparency for overlays and mattes (the output must be `.mov`). |

The built-in pack currently lists 65 effects and generators (`comp fx list`), including `bloom`, `halation`, `tilt_shift`, `split_tone`, `edge_glow`, `liquid` and `pixel_dissolve`. Sweep any of them with `comp fx test NAME --out sheet.png`.

## Ownership

`comp` depends only on `turbomotion-core`. It knows no film element; graphics from other crates arrive through
[`pack::Pack`](src/pack.rs), registered by whichever binary hosts the engine. See
[../../docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md).

## Layout

```
schema/    what can be said (strict serde, timeline, blocks, extrude, time literals)      plan/    what it means (resolved names, probed media)
eval/      what is true at t (pure numbers; `inspect`)                 render/  how it looks (Skia/Metal; masks, mattes, fx)
anim/      eases, properties, expressions, noise                       text/    layout, per-character animators, presets
fxlib/     shader plugins and the built-in pack                        media/   image cache, ffmpeg video, probes
audio/     mix and analysis signals                                    check/   lint and assertions
pack.rs    the host-registered graphics interface                      blocks/<job>/  the built-in blocks, compiled in
```

Examples that render: `examples/comp/01-shapes` … `09-look`. Tests: `cargo test -p turbomotion-comp`.

Tokens: `"palette"` is shared ink; `"type"` is text styles per role (size, font, weight, colour); `"image"` is picture treatment; `"surface"` is named plates; `"look"` is finishing on the finished frame. Text takes `"role"`, `"fill": "metal"` or a gradient, `"mark"` for a highlighter sweep across one word, and `preset: "reveal"` for a masked line-by-line open. Layers may `"reflect"` onto the floor. Audio `"src": "kit:card-in"` (also `stamp`, `counter`, `loss`, `reveal`). Check also gates contrast, minimum type size, `hero` frames and PNG `golden`s.
