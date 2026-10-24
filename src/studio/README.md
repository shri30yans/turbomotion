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

## Module map

| Area | Modules |
| --- | --- |
| Data | `model` (film schema), `geo` (outlines, Mercator view, great circles, globe projection), `media` (photographs, characters) |
| Services | `stage` (the bundle every element draws with), `shaders` + `shader` + `shaders/*.sksl`, `font` + `typeset`, `cache`, `atlas` (map masks and globe textures) |
| Drawing | `layer`, `anim`, `fx` (tilt, extrusion, reflection, halo, sprites), `background`, `grade`, `noise` |
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
