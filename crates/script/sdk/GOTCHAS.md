# Gotchas

Things that cost earlier agents iterations. Each has the fix next to it.

## Time

- **Shot time is not film time.** A layer inside `shot({ layers })` lives in the shot's own clock, which starts at 0 when
  the shot starts. A timeline you build for that shot (`tl.to(layer, ..., 1.2)`) is in the same shot-local seconds. A layer
  added with `film.add()` lives in film time.
- **`start` is the leading edge of the incoming transition.** Shots run back to back and a transition overlaps two shots by
  its length, so shot B (transition 0.5 s) starts 0.5 s before shot A ends. `shot({ start: 12 })` puts that leading edge at
  12 s and lengthens the shot before it to cover the transition. The new shot is fully in view at `start + transition.dur`.
  `montage()` does this arithmetic for a run of short shots.
- **Hits made on a timeline are shot-local.** `tl.hit(1.2, "boom")`, `sfx:` on a tween and `shot({ hits: tl.hits() })` all
  use seconds of the shot. The film writes them as `"<shotId>.start+<t>"`, which follows the shot when transitions move it.
  A film-level `timeline({ hits })` takes film time (or such a literal). Do not mix the two clocks.
- **A timeline position is the clock the layer lives in, whatever the layer's `start`.** `tl.to(layer, ..., 3)` happens at
  second 3 of the shot even when the layer has `start: 2`; the SDK writes it into the layer's own time (second 1). Times you
  write straight into a layer's options (`enter`, `exit`, `preset.at`, `animators`, counters, `mark`) are not converted:
  the engine counts them from the layer's `start`. A layer whose `start` is not a number cannot be tweened (the SDK
  cannot convert), so give `start` as seconds.
- **`--shot id` always starts at 0.** `sdk render --shot s4 --times 0.5,1` and `sdk inspect --shot s4 --t 0.5` use shot time.
  An isolated shot has no incoming transition and no `film.add()` layers; render the whole film to review those.
- **A transition can cover a shot's last beat.** Put the important motion at least one transition length before the shot ends.

## Space

- **Negative z is toward the lens.** A 3D layer at `z: -400` is closer and larger than one at `z: 0`.
  `tl.from(letters, { z: -400 })` starts 400 px nearer the lens and settles at the layer's own z.
- **1:1 at the z = 0 plane needs the camera `z = -zoom`.** `camera({ pos: [cx, cy, -1000], poi: [cx, cy, 0], zoom: 1000 })`
  shows the plane at z = 0 at its natural pixel size, centred on (cx, cy). Without a camera, a shot with 3D layers gets a
  default one that does this for you.
- **Aim with `poi`.** The camera looks at `poi`; `pos` only places it. Move both for a truck, only `pos` for an orbit.
- **Lights.** A 3D layer is lit only when the film has a light; add `light({ kind: "ambient" })` and a `directional` one.
- **Groups pass transforms, not opacity.** `group()` is a `null` layer: children follow its position, scale and rotation,
  but its `opacity`, `fx` and `start` do not reach them. Fade the children, or use a precomp.
- **A shape item rotates about the layer origin.** Give the item an `anchor` (its pivot), or draw it as a path.
- **Image anchors are top-left**; shapes centre on their position.

## Tweens

- **Overlapping tweens of one property are an error.** Set `overwrite: "auto"` on the later tween to interrupt the first.
- **`from()` ends at the layer's own value**, so give the layer the value it should end on.
- **Vectors and colours tween directly:** `scale: [2, 1]`, `pos: [x, y, z]` (a `null` entry leaves that axis alone), `color: "#f00"`.
- **`tl.set` is an exact step.** The old value holds up to the moment and the new one is read from that frame on (the
  engine's `hold` ease). A set at 0, or before the layer's `start`, is simply the layer's first value.
- **One scale, one or two numbers.** `scale: 2` and `scale: [2, 1]` mix freely between the layer, `from`, `to` and `fromTo`:
  a number against a list means the same on every axis. Lists of different lengths are an error naming the layer.
- **Relative values:** `x: "+=40"` moves 40 px from wherever the property is when the tween starts. Chained relative tweens add up.
- **Shape items** tween through `items`: `{ items: { 0: { trim_end: 1, stroke_width: "+=4" } } }`. A rect's size and corner
  radius are fixed numbers in the schema; animate the item's `scale` (about its `anchor`) instead.

## Files and paths

- **Assets resolve against the film's folder, not where the JSON lands.** `comp({ root: "assets" })` moves the root, relative to
  the script. Fonts, images, audio, blocks and `fx_path` all use it. `svgFile("icons/play.svg")` uses it too.
- **`--sdk-dir` is authoritative.** With it, a missing SDK module is an error, never silently the embedded one.
- **Name fonts by file for portable films:** `comp({ fonts: { Mono: "../lib/fonts/JetBrainsMono-Regular.woff" } })`.
  `measure()` and rendering then agree on widths.
- **Build writes beside the script** (`film.comp.json`). Run `comp check` from the film's folder so relative paths agree.

## Pixel art

`pixelSprite()` and `image(src, { sampling: "nearest" })` keep hard pixel edges. Use whole-number `px` and whole-number positions;
a precomp that enlarges a sprite needs `sampling: "nearest"` too (precomps default to linear). `bleed` hides hairline seams
between runs at fractional positions.

## Cost

- Rendering is rarely the bottleneck; iterations are. Use `sdk render --shot id --sheet-only` and `sdk watch` to iterate.
- `comp check film.comp.json --cost` lists effect surface per composition. Glow and blur on large layers are the expensive part
  (a full-frame glow costs about 5x a small one); put them on small layers, or lower the `radius`.
- `mblur` samples every shot several times: enable it for the final render, not while iterating.
- Many separate layers (a swarm of cubes has seven layers each) cost more than one shape with many items.
