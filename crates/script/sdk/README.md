# turbomotion SDK

Write a film in TypeScript. The binary strips the types (oxc), runs the script once on an embedded QuickJS, and the
script's default export becomes the composition JSON that `turbomotion comp` renders. No Node or browser. Read
[GOTCHAS.md](GOTCHAS.md) once: it is the list of things that cost earlier agents iterations.

```
turbomotion sdk build   film.ts [--out film.comp.json] [--sdk-dir sdk] [--shot hook]
turbomotion sdk render  film.ts [--out film.mp4] [--sheet sheet.png --times 0.5,2,4] [--sheet-only] [--shot hook]
turbomotion sdk watch   film.ts --sheet out.png --times 0.5,2 [--shot hook] [--interval-ms 400] [--once]
turbomotion sdk inspect film.ts --t 2.5 [--layer s4/word] [--shot hook]
turbomotion sdk check   film.ts [--tsc path/to/tsc.js] [--schema]
turbomotion sdk new scene NAME [--dir DIR] [--probe]
```

- `build` and `render` write `film.comp.json` beside the script and record the script folder as the asset root.
- `--shot id` builds one shot from its own time zero (see GOTCHAS), with sheet times local to the shot.
- `watch` re-renders the sheet when the script, anything it imports, an `svgFile`, or the `--sdk-dir` tree changes. It polls modification times; a build error is printed and watching continues.
- `inspect` prints the evaluated frame at `--t`; `--layer shot/layer` reaches inside shots and precomps (time is the containing comp's).
- `check` runs strict TypeScript through an installed `tsc` (never installs one). With no `tsc` (or `--schema`) it runs the film, validates every emitted layer with the engine (parse, plan, evaluate, lint) and reports each complaint at the `film.ts:LINE:COL` that created the layer. It prints which mode ran.
- `new scene NAME` writes `DIR/NAME/index.ts` (a function that takes typed options and returns a `shot(...)`) and `DIR/NAME/types.ts`; with `--probe` it also writes `DIR/probe_NAME.ts`, a film that renders only that scene. NAME is a folder-safe identifier (lowercase letters, digits, `-`, `_`); `chevron-tunnel` gives `chevronTunnel` and `ChevronTunnelOptions`. It never overwrites: if any target exists it lists the conflicts and writes nothing. It prints what it created and the `sdk render` command for the probe.
- `import ... from "./scenes/vs"` resolves like standard modules: `vs.ts`, then `vs/index.ts`. A miss lists every path tried. `--layer` accepts `id` or `shot/id`; an id two shots hold is an error that lists both.
- `--sdk-dir crates/script/sdk` loads the SDK from disk (new modules need no rebuild); without it the copy embedded in the binary is used. A chosen directory is authoritative.

## A film

```ts
import { comp, shot, timeline, light, letters3d } from "turbomotion";

const film = comp({ width: 1920, height: 1080, fps: 30, bg: "#0b0d12" });
const letters = letters3d("MOTION", { pos: [960, 540, 0], size: 240, depth: 70, color: "#f4f1ea" });
const tl = timeline();
tl.from(letters, { rotation: [-100, 60, null], opacity: 0, z: -400, duration: 1, ease: "back.out(1.4)", stagger: 0.07 }, 0.2);
film.timeline({ shots: [shot({ id: "hook", dur: 2.6, camera: { move: "orbit", deg: 24 }, layers: [light({ kind: "ambient" }), letters] })] });
export default film;
```

Working examples: `examples/sdk/launch/film.ts`, and one per component in `examples/sdk/components/`.

## API

| Area | Names |
| --- | --- |
| Film | `comp({width, height, fps, bg?, dur?, root?, fonts?, bpm?, look?})`; `film.add(...layers)`; `film.timeline({shots, overlays?, hits?, transition?, mblur?})` |
| Layers | `text`, `rect(w, h, {fill, radius, stroke})`, `shape(items, o)`, `image(src, o)`, `solid`, `group(o, children)`, `camera`, `light`, `raw({type, ...})` for any other kind |
| Artwork | `svg(markup \| pathData, {width, fill, stroke, viewBox})`, `svgFile("icons/x.svg", opts)` (engine SVG importer), `pixelSprite(rows, palette, {px})`, `image()` checks `fit`, `sampling`, `size`, `focus`, `camera`, `parallax` |
| 3D | `mesh`, `text3d`, `letters3d(word, {pos, tracking})`, `box`, `sphere`, `torus`, `cylinder`, `extrude`, `spin(deg_per_s)` |
| Timeline | `timeline({ease?, duration?})`: `.to .from .fromTo .set .addLabel .hit .hits .duration`; see Tweens |
| Shots | `shot({id, dur, layers, in, move, camera, bg, speed, fx, hits, start})` |
| Sound | `audio(src, opts)`, `hit(at, cue, volume?)`, `tl.hit`, `sfx: "pop"` on a tween |
| Stage | `stage(layers, poses).start(pose)`, `.go(tl, pose, {at, dur, ease, stagger})`: smart animate |
| Text | `measure(text, {font, size, weight, tracking, width})` returns widths and character/word boxes |
| Utilities | `rand(seed)`, `range`, `lerp`, `clamp`, `grid(cols, rows, cell)`, `darken` |

Layer options: `id, pos | x,y,z, anchor, scale, skew, rotation, orientation, aim, cull, opacity, three_d, lit, material, parent, start, dur, blend, fx, masks, matte, clip_to, enter, exit, wrap, stagger, flip, along, reflect, layout, hidden, mblur, extra`. Shape items (`rect`, `ellipse`, `d`, `points`, fills, strokes, `trim`, `repeat`) and text options are typed too; a test (`tests/sdk_option_types.rs`) fails when the engine gains a field the types lack.

`rotation` is After Effects' X, Y and Z Rotation: one number is the Z Rotation (the turn in the layer's own plane), three numbers turn it about each axis. `orientation` is the layer's own pose and `aim` the point its face turns toward.

## Tweens

`tl.to(layer, vars, position?)`. `vars`: `x y z opacity rotation scale size intensity zoom color`, plus `pos`, `items`, `orientation`, `aim`, and `duration ease delay stagger sfx overwrite`.

```ts
tl.to(card, { scale: [1.2, 0.9], pos: [300, "+=40", null], color: "#ff6633", duration: 1 }, 0);   // vectors, relative values, colours
tl.to(ring, { items: { 0: { trim_end: 1, stroke_width: "+=4" } }, duration: 1 }, 0.5);            // a shape layer's items
tl.to(card, { x: 500, duration: 1, overwrite: "auto" }, 0.5);                                      // interrupt the earlier x tween
```

Positions: seconds, `"+=0.5"`, `"-=0.2"`, `"<"`, `">"`, or a label, on the clock of the shot (or film) the layer lives in, whatever the layer's `start`. `tl.set(layer, vars, at)` is an exact step. Tweens of one property merge into one key list. Overlapping tweens are an error unless the later one sets `overwrite: "auto"`. Item properties: `trim_start trim_end trim_offset offset rotation scale opacity morph stroke_width stroke_opacity dash_offset stroke_color fill_opacity fill_color` (a rect's size and radius are fixed in the schema).

## Components

`import { cube, promptBox, ... } from "turbomotion"` (sources in `sdk/components/`). Most take `{ tl, at, ... }`, register tweens and sounds on `tl`, and return `{ layers, end, ...parts }`; put the result straight into `shot({ layers: [part] })`. `theme` restyles. Components marked **appear** take `appear: "slide_up"` or `{ preset, dur, stagger, delay }`, an engine entrance on every layer that never fights the component's own motion.

| Component | One line |
| --- | --- |
| `promptBox` **appear** | `promptBox({ tl, text: "turbomotion render x.ts", at: 0.9, cps: 14, enterAt: 3.2 })`: glass card, typed text, blinking caret, Enter |
| `codePanel` **appear** | `codePanel({ tl, lines: [highlight(...)], at: 12, mode: "type" })`: syntax-coloured lines, a bar following the line |
| `browserWindow` **appear** | `browserWindow({ tl, url: "localhost:3000", at: 0.4 })`: window chrome with spinner and skeleton page |
| `statCounter` **appear** | `statCounter({ tl, value: 373, suffix: " fps", label: "frames per second", at: 36 })`: counts up, slams |
| `barRace` **appear** | `barRace({ tl, at: 37, dur: 2.4, bars: [{ label: "ours", value: 373, color: "#ff5a1f" }] })` |
| `bboxOverlay` **appear** | `bboxOverlay({ tl, at: 1, boxes: [{ x: 120, y: 80, w: 1240, h: 180, label: "title" }] })`: HUD boxes |
| `cardWall` **appear** | `cardWall({ tl, cols: 8, rows: 5, seed: 11, hero: 20, at: 0.1 })`: tilted 3D wall, origin on the hero card |
| `waveform` **appear** | `waveform({ tl, at: 4.5, pos: [960, 800], w: 700, bars: 32 })`: beat-driven bars |
| `textStack` | `textStack({ tl, roles, items: [{ text: "two", at: 0.6, role: "key", row: 0 }], pos, width })`: shaped, wrapped, FLIP-gliding words |
| `title3d` | `title3d("MOTION", { tl, at: 0.3, size: 240 })`: letters tumble in |
| `scribble` | `scribble({ tl, at: 1.2, pos: [960, 560], w: 1300, h: 760 })`: hand-drawn strike-through |
| `cutout` | `cutout({ id: "p", src: "a.png", size: [200, 300], idle: { float: { amp: 10, period: 2 } } })`: an image with idle motion |
| `cube` | `cube({ id: "c", size: 200, faces: ["#3D6BFF", { fill: "#FF5A1F", items }], pos: [960, 540, 0] })`: six shape faces; spin `root` |
| `puzzleCube` | `puzzleCube({ id: "p", size: 240, tile: (i, s) => icon(i, s) })`: 3x3x3 with `slabs` to twist |
| `letterCube` | `letterCube({ id: "l", size: 200, fill: "#8F6CFF", items, letter: "A" })`: art front, letter back; turn `root` 180 |
| `swarm` | `swarm({ tl, id: "s", n: 18, seed: 5, at: 0.1, dur: 1.2, art: (i, s) => items })`: cubes burst toward the lens |
| `flipTile` | `flipTile({ tl, id: "t", size: [200, 260], pos: [300, 270], front: { fill, label: "?" }, back: { fill, label: "A" }, at: 1 })` |
| `hand` | `hand({ id: "h", pos: [400, 600], rotation: -20, fill: "#f3f0e8", rim: "#3D6BFF" })`: glowing vector hand (`flip` for the other) |
| `capsule` | `capsule({ from: [100, 100], to: [500, 300], w: 24, fill: "#C8F031" })` or `{ pos, len, deg, w }`; `capsulePath(...)` gives the path |
| `lens` | `lens({ id: "lens", pos: [960, 540], size: 360, open: { tl, at: 0.5 } })`: barrel, glass, iris that opens |
| `blob` | `blob({ id: "b", size: 320, seed: 4, fill, morph: { tl, seed: 9, at: 1, dur: 1.2, back: true } })`: seeded organic shape |
| `montage` | `film.timeline({ shots: montage({ items: [{ word: "fast", field: "#FF5A1F" }], hold: 0.5 }).shots })`: word shots with start/overlap arithmetic done; `starts`, `inView` give film times |
| `dollyZoom` | `dollyZoom(cam, { tl, at: 0.5, dur: 2, amount: 0.45 })`: vertigo; the plane at z = 0 keeps its size |
| `glassPanel` | `glassPanel({ id: "card", pos: [960, 540], size: [640, 360], radius: 28, tint: "#cfe1ff" })`: translucent panel with sheen and rim (see note) |
| `appear` | `appear(layers, "slide_up", at)`: the entrance used by the **appear** option, for any list of layers |

Note: `glassPanel` is translucent only. The engine has no `backdrop` blur effect yet; `blur` is passed through as
`{ effect: "backdrop", blur }` for when it has one, and the plan will not load with it until then.

Fonts: `comp({ fonts: { "JetBrains Mono": "../kit/fonts/JetBrainsMono-Regular.woff" } })` takes `.woff` and `.ttf`.

## Determinism

The script runs once with no clock, network or disk beyond imports and `svgFile`. `Math.random` is seeded, `Date` and
`performance.now()` are frozen, and there are no timers. Prefer `rand(seed)`. A script running longer than 20 s is stopped.
Same script and options give the same JSON byte for byte.

## How it runs

`crates/script`: `strip.rs` (oxc type stripping with a source map), `loader.rs` (module resolver: `"turbomotion"` is the SDK
from `--sdk-dir` or embedded), `run.rs` (QuickJS, deterministic globals, error mapping, the files read and layer creation lines),
`assets.rs` (SVG import through `turbomotion_comp::api`), `fallback.rs` (schema check), `watch.rs`, `cli.rs`.
Errors name the original file, line and column; SDK frames show as `turbomotion/timeline.ts:LINE`, and tween mistakes quote
the line that wrote the tween.

Not yet: a `precomp` builder (use `raw`), nested timelines, a backdrop-blur effect.
