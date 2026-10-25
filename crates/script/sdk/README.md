# turbomotion SDK

Write a film in TypeScript. The binary strips the types (oxc), runs the script once on an embedded QuickJS, and the
script's default export becomes the composition JSON that `turbomotion comp` renders. Building and rendering need no Node or browser. Optional type checking invokes an installed TypeScript compiler.

```
turbomotion sdk build  film.ts [--out film.comp.json] [--sdk-dir sdk] [--shot hook]
turbomotion sdk render film.ts [--out film.mp4] [--sheet sheet.png --times 0.5,2,4] [--sheet-only] [--workers 3]
```

`render` writes `film.comp.json` beside the script and records the script folder as its asset root, then calls the same code as
`comp sheet` and `comp render`.

## A film

```ts
import { comp, shot, timeline, light, letters3d } from "turbomotion";

const film = comp({ width: 1920, height: 1080, fps: 30, bg: "#0b0d12" });
const letters = letters3d("MOTION", { pos: [960, 540, 0], size: 240, depth: 70, color: "#f4f1ea" });

const tl = timeline();
tl.from(letters, { rot_x: -100, rot_y: 60, opacity: 0, z: -400, duration: 1, ease: "back.out(1.4)", stagger: 0.07 }, 0.2);

film.timeline({ shots: [
  shot({ id: "hook", dur: 2.6, camera: { move: "orbit", deg: 24 }, layers: [light({ kind: "ambient" }), letters] }),
] });
export default film;
```

Working example: `examples/sdk/launch/film.ts`.

## API

| Area | Names |
| --- | --- |
| Film | `comp({width, height, fps, bg?, dur?, palette?, fonts?, look?})` returns a `Film`: `film.add(...layers)`, `film.timeline({shots, overlays?, hits?, transition?})` |
| Layers | `text(str, o)`, `rect(w, h, {fill, radius, stroke})`, `shape(items, o)`, `image(src, o)`, `solid(color, o)`, `group(o, children)` (a `null` parent; children follow it), `camera(o)`, `light({kind, ...})`, `raw({type, ...})` for any other kind |
| 3D | every solid is one `mesh` layer with a `geometry`: `mesh(geometry, opts)`, `text3d(str, {font, weight, size, depth, bevel, color, side, material, ...})`, `box`, `sphere`, `torus`, `cylinder`, `extrude({d, size})`; `letters3d(word, {pos, tracking})` returns one text mesh per letter, placed from `measure()`; `spin(deg_per_s, from?)` is a turntable expression for `rot_y` / `rot_x` |
| Layer options | `id, pos \| x,y,z, anchor, scale, rotation, rot_x, rot_y, opacity, three_d, material, extrude, parent, start, dur, blend, fx, masks, matte, enter, exit, extra` (`extra` merges any engine key) |
| Timeline | `timeline({ease?, duration?})`: `.to(target, vars, pos?)`, `.from`, `.fromTo(target, from, to, pos?)`, `.set`, `.addLabel`, `.duration()`. `vars`: `x y z opacity rotation rot_x rot_y scale size intensity zoom color` plus `duration ease delay stagger` (`0.07` or `{each, from: "start"\|"end"\|"center"}`). Positions: seconds, `"+=0.5"`, `"-=0.2"`, `"<"`, `">"`, a label. Default position is the end of the timeline; default ease `power1.out`; any engine ease name works |
| Shots | `shot({id, dur, layers, in, move, camera, bg, speed, fx})`; `in` is a transition name or `{type, dur, ease?, portal?, radius?}`. `film.timeline({ shots, mblur: true })` samples every shot across the shutter, so transitions and moves carry true motion blur. `window` opens the next shot as a rounded card (`portal: [cx, cy, width]`, `radius`) while the last zooms toward it; `cross_zoom` is a constant-speed dolly |
| Sound | `audio(src, {id, volume, fade_in, fade_out, src_in})` layer; `hit(at, cue, volume?)`; `tl.hit(pos, cue, volume?)`; `sfx: "pop"` on any tween (one hit per staggered layer, at its tween start); `tl.hits()` then `shot({ hits })`. Hits made on a timeline are shot-local seconds and are written as `"<shotId>.start+<t>"`, which the engine resolves after transitions have overlapped the shots (a shot with `start` keeps it). Film-level `timeline({ hits })` takes film time or such a literal |
| Stage | `stage(layers, poses)`: smart animate. `.start(pose)` places layers, `.go(tl, pose, { at, dur, ease, stagger, order })` glides every layer from where it is to the pose, tweening only the properties that differ. `ease` may be one name or one per property. A group's `opacity` fades everything parented to it. Returns the time the last layer lands |
| Utilities | `rand(seed)` (callable, `.range .int .pick .shuffle`), `range(n)`, `lerp`, `clamp`, `grid(cols, rows, cell)` (cells centred on 0,0), `darken` |

Tweens on one property of one layer merge into a single key list in time order (holding the layer's own value before
the first tween and between tweens); one clean tween stays in the `{from, to, at, dur, ease}` form. Times are in the
clock of the comp the layer lives in: a shot's layers use shot-local time. Overlapping tweens of one property are an
error unless the later tween explicitly sets `overwrite: "auto"`. Types live in the SDK interface modules, so an editor flags typos; at runtime the SDK checks sizes,
ids, parents, shot ids, tween names and layer values and says what to change.

`image(src, { sampling: "nearest" })` preserves source pixels; `"linear"` interpolates. Omission keeps automatic
cubic enlargement and mipmapped reduction. A containing `raw({ type: "precomp", comp: "sprite", sampling: "nearest",
scale: 4, ... })` also needs nearest sampling when enlarged; precomps default to linear. Use integer scale and aligned
positions for pixel art.

## Components

`import { promptBox, codePanel, ... } from "turbomotion"` (sources in `sdk/components/`). Each takes `{ tl, at, ... }`, registers its tweens and sounds on the timeline `tl`, and returns `{ layers, end, ...named parts }`: put it straight into `shot({ layers: [part] })` (arrays and components are both opened). Times are in the timeline's clock; `theme` restyles (defaults are the launch film's tokens). Fonts: `comp({ fonts: { "JetBrains Mono": "../kit/fonts/JetBrainsMono-Regular.woff" } })` takes `.woff` as well as `.ttf`.

| Component | Call |
| --- | --- |
| `promptBox` | `{ tl, text, at, cps, pos, w, h, size, prompt, enterAt?, beat, caretColor }`: glass card, typed mono text, caret that steps per character and blinks on beats, a `type` hit per visible character, Enter (`click`, card presses on a spring, `dot` appears) |
| `codePanel` | `{ tl, lines, at, perLine, mode: "type" \| "reveal", pos, w, size, title }`: syntax colours as text `runs` (`highlight(line, colors)`), a highlight bar that follows the current line, a `tick` per line |
| `browserWindow` | `{ tl, url, at, pos, w, h, slide }`: generic chrome (dots, URL pill, spinner, skeleton page); `root` is the group to move |
| `statCounter` | `{ tl, value, suffix, prefix, label, at, count, pos, size, sfx }`: counts up, slams with a scale overshoot and a `stamp` |
| `barRace` | `{ tl, bars: [{ label, value, color, dur? }], at, dur, pos, w, max }`: bars grow, counters ride the tips |
| `bboxOverlay` | `{ tl, boxes: [{ x, y, w, h, label, state, change? }], at, step }`: HUD boxes (`ok` cobalt, `clash` turbo, `fixed` volt), a `tick` per box |
| `cardWall` | `{ tl, cols, rows, cell, tilt, palette, seed, hero, heroLabel, words, pos, at }`: tilted 3D wall, origin on the hero card (`hero`, `wall`, `cards`) |
| `textStack` | `{ tl, items: [{ text, at, role, row }], roles: { key: { size, font, weight, color, upper, cps }, ... }, pos, width, align, anchor, leading }`: words arrive on cue and lay themselves out with the renderer's own text shaping. Rows wrap at `width`, a centred row re-centres as words join it, and the words already there glide (FLIP) instead of jumping. `cps` types a word in; otherwise it enters with `entry` (default `fade_up`) |
| `title3d` | `(word, { tl, at, pos, size, depth, color, side, tracking, stagger, ... })`: `letters3d` plus the tumble-in; `end`, `landed` |

Notes: the typewriter text preset staggers per visible character (a space takes no time).

## Determinism

The script runs once, with no clock, no network, no disk beyond imports. `Math.random` is a seeded generator, `Date`
and `performance.now()` are frozen at 0, there are no timers. Prefer `rand(seed)` (explicit, order-independent seeds).
A script that runs more than 20 s is stopped. Same script and options, same JSON, byte for byte.

## How it runs

`crates/script`: `strip.rs` (oxc parse + TypeScript transform + codegen with a source map), `loader.rs` (QuickJS module
resolver: `"turbomotion"` loads a selected SDK directory, with embedded `include_str!` sources as an installation fallback; `./x` and `./x.ts` are files beside the
importer, each type-stripped on load), `run.rs` (engine setup, deterministic globals, default export to JSON,
error reports), `cli.rs` (`sdk build`, `sdk render`).

Errors name the original file, line and column: stack frames are mapped back through the source map, SDK frames show
as `turbomotion/timeline.ts:LINE`, and tween mistakes found when a layer is written out quote the line that wrote the
tween. Syntax errors give `film.ts:LINE:COL` and the line.

## Not yet

No `precomp`/`comps` builder (use `raw`); no nested timelines.


## Editable SDK and shot previews

SDK lookup is explicit `--sdk-dir`, otherwise the sources embedded in the binary. A selected disk tree is authoritative: missing
modules produce an error instead of mixing SDK versions. A newly added module needs no binary rebuild.
Library callers select the same override with Rust `Options { sdk_dir: Some(path), ..Default::default() }`.

```sh
turbomotion sdk build examples/sdk/launch/film.ts --sdk-dir crates/script/sdk --out /tmp/launch.comp.json
turbomotion sdk render examples/sdk/launch/film.ts --shot hook --sheet /tmp/hook.png --times 0,0.5,1 --sheet-only
```

`--shot` is supported by build and render and always uses the shot's own zero-based clock. It writes
`film.shot.comp.json` and defaults video output to `film.shot.mp4`. Preview isolation removes incoming
transitions and film-level `film.add()` layers. It retains the selected shot's camera, speed, effects,
background, overlays and hits expressed as `<shot>.start+<seconds>`; absolute film hits are omitted.
The preview duration is the shot's declared duration. Use the ordinary film render to review transitions
and film-level overlays/audio together.

The asset root is the script's folder even when JSON is written elsewhere. `comp({ root: "assets", ... })`
changes that root relative to the script. Fonts, image/audio sources, blocks and `fx_path` use this root.
Changing an exported JSON's asset root is explicit: set its `root` field. CLI success exits 0; invalid
scripts, SDK overrides and unknown shots return nonzero with the error and available shot names.

## Measured text and vector tweens

```ts
import { comp, measure, text, timeline } from "turbomotion";
const film = comp({ width: 1280, height: 720, fps: 30, dur: 2 });
const bounds = measure("Craft matters", { size: 80, weight: 700, tracking: 1 });
const label = text("Craft matters", { pos: [640, 360], size: 80, weight: 700,
  scale: [1, 1], color: "#202020" });
timeline().to(label, { scale: [1.2, 0.9], color: "#ff6633", duration: 1 }, 0);
film.add(label);
export default film;
```

`measure(text, { font, size, weight, italic, tracking, leading, width, align })` returns `width`, `height`,
`baseline`, `lines`, and `chars` / `words` boxes `{x,w,line}`. Character boxes also carry `ch`; word boxes
carry `text`. It uses the renderer's shaper and the most recently constructed comp's font roles/root.
Construct the film before measuring role fonts. Installed family measurements depend on installed font
versions; explicit font files make the design portable. Numeric vector scale and colour tweens emit the
engine's native tween/key forms; overlapping property tweens require `overwrite: "auto"` on the later tween.


## Real type checking and interruption

```sh
turbomotion sdk check examples/sdk/launch/film.ts --sdk-dir sdk --tsc /path/to/installed/tsc.js --json
```

`sdk check` runs an installed `tsc` executable (default: `tsc` on PATH) or `tsc.js` through Node. It uses
strict TypeScript checking with the selected SDK tree, follows relative film imports, emits diagnostics
and returns nonzero on type errors. It never executes the film, installs packages or uses the network.
`--json` prints `{ok,file,diagnostics}`. An installation without on-disk SDK sources must pass `--sdk-dir`.

```ts
tl.to(label, { x: 300, color: "#ffffff", duration: 2, ease: "power2.in" }, 0);
tl.to(label, { x: 500, color: "#ff6633", duration: 1, ease: "none", overwrite: "auto" }, 1);
```

Overwrite acts only on properties named by the incoming tween. At its start it reads the previous
curve's live value and continues from there; explicit `fromTo` values can jump intentionally. Earlier
parts retain their exact easing, including overshoots. Scalar/vector/colour interruptions compile to
piecewise engine expressions, preserving the curve without sampled approximations. A later overlapping
tween still requires its own `overwrite: "auto"`; omitted/false keeps the existing overlap error.

`comp check film.comp.json --cost --json` reports findings plus per-composition effect-surface MPx and
glow-surface MPx estimates. The estimate samples five local times and counts each visible surface once
per effect stage; it is useful for comparing large effect layers, while blur padding, internal shader
passes and hardware timing remain outside the estimate. Ordinary `check --json` retains its findings-array format.
`comp catalogue` includes complete `runnable_property_examples` compositions, with tests that load and
evaluate every example (vectors, colour keys/channels, gradients and shape-item pivots included).

## Parenting and pixel groups

`group()` creates a `null` transform parent and flattens its children into sibling layers. Parenting
inherits the parent matrix (position, rotation, scale and anchor). Each child retains its own timing,
visibility, opacity, masks and effect chain. Setting `start`, `dur`, `hidden`, `opacity` or `fx` on the
null does not gate or process the children's pixels; a null has no rendered surface.

Use a named composition with a `precomp` layer, or a shot's nested composition, when a collection must
share a lifetime, opacity, mask or effect. Its child clocks are local to that composition and the
precomp's rendered surface receives the group effects. Example with the existing raw escape hatch:

```ts
import { comp, raw } from "turbomotion";
const film = comp({ width: 640, height: 360, fps: 30, dur: 4,
  extra: { comps: { card: { width: 640, height: 360, dur: 2, layers: [
    { type: "solid", color: "#ff6633", size: [160, 90], pos: [320, 180] }
  ] } } } });
film.add(raw({ type: "precomp", comp: "card", start: 1, dur: 2,
  fx: [{ effect: "glow", radius: 12 }] }));
export default film;
```
