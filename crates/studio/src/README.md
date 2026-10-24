# `turbomotion studio`: the motion-graphics film renderer

Renders word-anchored motion-graphics films (kinetic type, charts, maps, a globe, photo plates, 17 transitions, a
film-look post stack) from a compiled `film.json`. It is one subcommand of the `turbomotion` crate and shares its GPU
context, encoder pipe and segment-parallel scheduler with the shot renderer.

```
turbomotion studio render <dir> --out silent.mp4 [--from F --to F --workers N]
turbomotion studio stills <dir> --out DIR --times 12.0,45.5
```

`<dir>` holds `film.json`, `geo.json`, `plates/` and `chars/`. They are written by the studio plan compiler, a Python
pipeline (Pillow, OpenCV, numpy) that resolves narration-anchored timing into seconds. That pipeline also serves as the
parity reference and lives with the films, outside this repository; `model.rs` is the schema.

## What goes in

`film.json` is the compiled plan: every anchor ("word 12 of block b05_1") is already resolved to seconds, every
element is a plain record. It is parsed strictly (`model.rs`): an unknown element kind, an unknown parameter or a
timeline with gaps is an error at load, never a silent default. `geo.json` carries the simplified country and state
outlines (India with its official boundary, PoK and Aksai Chin included), `plates/` the photographs, `chars/` the
cut-out characters.

## How a frame is made

```
render.rs      Renderer: one per worker, owns the GPU surface, the Stage and the grain textures
  scene.rs       backdrop -> live elements in plan order -> camera move and shake      (one scene = one image)
  transitions/   blends the outgoing and incoming scene while a transition runs        (17 named transitions)
  post.rs        grade, bloom, chromatic aberration, vignette, grain, fades in ONE shader pass
```

Everything is composed in half-float, premultiplied `Layer`s (`layer.rs`). Elements build a layer (text, panel, chart,
map…) and land it with `anim::blit`, which applies the entrance, exit, shadow and rotation shared by all of them.

## Effects: one library for any layer

Any element, and any whole scene, can carry an `fx` list. Each effect takes the layer built so far and returns a new
one, in order; every number can be animated with `[[seconds, value], ...]` keys; an element can also set a `blend`
mode (`add`, `screen`, `multiply`, `overlay`, `difference`, ...).

```json
{"k": "title", "text": "HELLO", "t0": 2.0,
 "fx": [{"effect": "glow", "radius": 30, "color": [255, 200, 120]},
        {"effect": "blur", "radius": [[2.0, 24], [3.0, 0]]},
        {"effect": "rgb_split", "distance": 6}],
 "blend": "screen"}
```

`turbomotion studio effects` prints the catalogue as JSON (name, what it does, a complete example), generated from the
same registry the renderer parses with (`effects/mod.rs`), so it cannot drift. 26 effects today: blur, directional
blur, radial blur, glow, shadow, outline, brightness, contrast, saturation, hue, exposure, gamma, invert, tint, grade,
wave, twirl, bulge, pixelate, chromatic aberration, RGB split, grain, scanlines, vignette, opacity, transform.
Unknown effects, unknown parameters and out-of-order keys are errors at load. `examples/studio-effects/` renders every
effect on a title ([gallery](../../docs/studio-effects.jpg)).

Adding an effect is a parameter struct plus one function in a family file (`effects/blur.rs`, `color.rs`, `distort.rs`,
`edge.rs`, `texture.rs`, `place.rs`) and one line in the `registry!` list.

Effects run on the element's own full-frame layer, so pixel parameters are plain pixels and centre points
(`x`, `y`) are fractions of the frame. Effects are not part of the Python parity reference.

## General motion-graphics features live in `comp`

Masks and track mattes, nested compositions, video and audio, shape and path layers, per-character text animation,
particles, 3D cameras and lights, expressions and shader plugins belong to the composition engine
(`crates/comp`, `turbomotion comp`), not to this film renderer. Film elements (globe, maps, charts, panels) can sit on
a composition's timeline through the `studio.element` pack (`src/pack.rs`). Still missing here: a cross-platform
backend (Metal only today).

## Module map

| Area | Modules |
| --- | --- |
| Data | `model` (film schema), `geo` (outlines, Mercator view, great circles, globe projection), `media` (photographs, characters) |
| Services | `stage` (the bundle every element draws with), `shaders` + `shader` + `shaders/*.sksl`, `font` + `typeset`, `cache`, `atlas` (map masks and globe textures) |
| Drawing | `layer`, `anim`, `fx` (tilt, extrusion, reflection, halo, sprites), `background`, `grade`, `noise` |
| Effects | `effects/` (registry, one file per family, `shaders/effects/*.sksl`) |
| Elements | `elements/{text, panels, surface, marks, charts, maps, photo}` — one function per element kind, dispatched in `elements/mod.rs` |
| Frame | `scene`, `transitions/`, `post`, `render`, `cli` |
| Reference parity | `rng` (numpy's MT19937, bit-exact), `pyfmt` (Python number formatting), `ease`, `geom` (OpenCV's matrix conventions), `path` |

Dependencies point downward: elements use services and drawing helpers, never each other; shaders know nothing of Rust.
A new element kind is: a struct in `model.rs`, one function in an `elements/` file, one line in `elements/mod.rs`.

## Parity with the Python reference

the Python studio pipeline (Pillow / OpenCV / numpy) is kept as the oracle. The port reproduces its conventions rather than
approximating them: OpenCV pixel-centre geometry and warps, `INTER_AREA` resampling, numpy's random streams (film grain,
paper fibre, glitch slices), Python `int()` truncation and banker's rounding where they matter, and its colour grade.
Measured over all 62 scenes of Film 1: mean SSIM 0.975, worst 0.936; every transition ≥ 0.97 at 20/50/80 % progress.

Known, intentional differences:

* **Text edges.** Pillow lays out with FreeType's hinted advances (±1 px per glyph); Skia is unhinted. Advances are
  rounded to match, so lines break and align identically, but glyph edges differ at the sub-pixel level. This is the
  whole gap in the lowest-scoring (text-only) scenes.
* **Anti-aliasing.** Where the reference draws aliased PIL lines (globe arcs, map arcs) the native renderer anti-aliases.
* **Two-stage warps.** The reference resamples twice in a few transitions (shift, then blur); the shaders sample once.
* **Map views** are exact; the reference quantises its cached land mask to 0.01° (visible only at high zoom).
* **Line chart.** The reference pasted the plot 20 px away from its own head glow; fixed in the reference.

## Rules the code keeps

* The size of every uniform buffer is checked against the shader's declarations at run time (`Program::data`), so a
  shader and its Rust caller that drift apart fail loudly instead of drawing garbage.
* Noise is stored as `0.5 + 0.1·n` so it survives layers that clamp; additive light uses the `add_clip` blender and
  coverage masks the `union` blender, because half-float layers do not clamp by themselves.
* Frames are pure functions of time: `Renderer::draw(frame)` has no state that depends on the previous frame, so
  segments render in parallel and any frame can be re-rendered alone.
