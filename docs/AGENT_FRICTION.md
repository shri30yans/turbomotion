# What slowed agents down, and the fixes

Five Sonnet agents built two films with the TypeScript SDK on 2026-10-25:
- the turbomotion launch film (46 s, 16 shots, four agents);
- a shot-for-shot rebuild of a 20 s vertical reference video (one agent).

Sources:
- `launch/debrief/*.md` (components; shots 3-4, 5-6, 7-9);
- `examples/rebuild/snapgram/GAPS.md`;
- the agents' reports.

## Numbers

| Agent | Iterations | Error-only builds | Binary rebuilds | Biggest cost |
| --- | --- | --- | --- | --- |
| components + shots 1-2 | 7 | 0 | 6 | rebuilding the binary for every SDK edit; caret sync |
| shots 3-4 | 9 | 3 | 0 (worked around) | embedded SDK; anchor and scale semantics; 3D needs a camera |
| shots 5-6 | 8 | 2 | 0 | timing conflict between shots; 3D placement under a tilted parent |
| shots 7-9 | 4 | 1 | 0 | font paths; no text measurement |
| reference rebuild | 17 | 6 | 0 | hand-drawing assets (35 %); misreading the reference; finding schema rules by hitting errors |

Building a script takes 0.01-0.7 s, a sheet under 1.5 s, and full renders run at 90-150 fps (glow-heavy shots drop to 27 fps). **Rendering is never the bottleneck. Iterations, guessing and missing assets are.**

## Fixes, ranked by agent time they would save

### Tier 1: remove iteration loops (every agent hit these)

| # | Fix | Where | Hit by | Status |
| --- | --- | --- | --- | --- |
| 1 | **Load the SDK from disk when present** (`sdk/` beside the binary, or `--sdk-dir`); embed only as a fallback. New components then need no edit to `loader.rs` and no rebuild. | `crates/script/src/loader.rs` | 3 of 5 | done |
| 2 | **`measure(text, {font, size, weight, tracking})` at build time**, plus `line([...words], {gap})` and the caret position for typed text. The engine already shapes text. | `crates/script` + `sdk/` | 5 of 5 | partial (`measure` done; `line()` and a caret helper open) |
| 3 | **Per-shot dev loop:** `sdk render --shot s4 [--local]` renders one shot from its own time zero, with sheet times local to the shot. No stub shots and no 30 s blank lead-in. | `crates/script/src/cli.rs` | 4 of 5 | done |
| 4 | **Paths resolve against the film root** (the script's folder), not where the JSON lands. Font failures should name the font, not a frame number. | SDK `comp()` + comp errors | 3 of 5 | done |
| 5 | **Type check.** Ship `sdk/*.d.ts` and run a fast checker in `sdk check`; today types are stripped and never checked. | `crates/script` | 3 of 5 | done (needs an installed `tsc`) |
| 6 | **`inspect` sees inside shots:** `comp inspect --layer s9/disc`, with the timeline expanded in `sdk build`. | comp CLI | 3 of 5 | done |

### Tier 2: assets and art (the gap behind "graphics need improving")

| # | Fix | Where | Status |
| --- | --- | --- | --- |
| 7 | **An asset path in the SDK:** `svg("<path d>")` and `svgFile("icon.svg")` (the `comp svg` importer, callable from TS), `pixelSprite(rows, palette)`, `image()` with fit and focus | SDK + `comp svg` as a library call | done (`svg`, `svgFile`, `pixelSprite`) |
| 8 | **An icon and illustration library in `kit/`:** UI icons, hands, cursor, devices, cubes, sprites; drawn once, reusable | `kit/` | partial (`hand`, cube faces and icons are components; no shared UI-icon set) |
| 9 | **Promote the rebuild's components:** `cube({size, faces})`, `puzzleCube`, `letterCube`, `swarm`, `flipTile`, hands | `sdk/components/` | done |
| 10 | **Surfaces:** glass panels that blur what is behind them, radial and mesh-gradient backdrops, a studio backdrop, reflective floors (in the docs) | engine + SDK | done (`backdrop` effect + `glassPanel`; `aura` mesh gradient; `floor`) |

### Tier 3: API semantics that caused wrong guesses

| # | Fix | Status |
| --- | --- | --- |
| 11 | `tl.to` accepts vectors (`scale: [sx, sy]`) and colours (emit the colour tween form the engine accepts), and shape-item properties (trim, rect width) | done (shape-item geometry stays static: animate item `scale` about `anchor`) |
| 12 | GSAP-style `overwrite: "auto"`: a later tween on the same property truncates the earlier one instead of erroring (hit 5 times in one film) | done |
| 13 | A pivot for shape-item rotation (an `anchor` per item), documented | done |
| 14 | `typewriter` counts spaces optionally, takes `dur`, and exposes caret position and per-character hits | partial (`dur` and counted spaces done; caret via `measure` and `promptBox`) |
| 15 | Precomps inherit `bpm`, so `beat()` works inside shots | done (nested comps inherit the film's `bpm`) |
| 16 | Shots with `three_d` layers get a default camera; document the z sign (negative is toward the lens) and aiming (`poi`, `z = -zoom` for 1:1) | done |
| 17 | `letterbox` / safe area on `comp()`, applied after transitions | done |

### Tier 4: docs and checks

| # | Fix | Status |
| --- | --- | --- |
| 18 | Every property form with a runnable example in `comp catalogue` (vector keys, colour keys, radial gradients, item pivots), and one page of gotchas | done |
| 19 | Rules for shot time versus film time in `sdk/README.md`: `start` is the leading edge of the transition; hits are local to the shot; how `film.add` and `timeline` coexist | done |
| 20 | A check that warns when a later transition covers a shot's last beat | done (`check` rule `transition-covers-last-beat`) |
| 21 | `comp check --cost`: per-shot effect cost (glow on large shapes is 5x slower), so agents know what is expensive | done |
| 22 | `sdk render --ref video.mp4 --crop ...`: reference and ours side by side at matched times (the rebuild spent about 20 % of its time on ffmpeg plumbing for this) | done (`comp sheet --ref --crop`; not wired into `sdk render`) |
| 23 | Pitched kit sounds (`pluck:A3`, `bass:C2`) so a music bed needs no Python synth | done (`kit:pluck/C4`) |
| 24 | Higher-level helpers: `montage([{word, field, leaves}])`, `dollyZoom(cam)`, `appear` entrances on every component | done (`montage`, `dollyZoom`, `appear`) |

## Order of work

1. **Tier 1 first.** About half of all iterations were tooling loops rather than design work.
2. **Then Tier 2.** It is what makes the films look premium, and the reference rebuild shows the engine already matches structure, timing, colour and motion; the gap is art.
3. **Tiers 3 and 4 as one sweep.** Each item is small; together they remove most first-attempt errors.

## Implementation status — 2026-10-25

Verified code paths in this pass (historical reports above remain the original findings):

| Items | Current behavior |
| --- | --- |
| 1 | SDK source tree loads from `--sdk-dir` / `Options.sdk_dir`, SDK beside the executable, or Cargo project SDK; embedded modules remain an installation fallback. A chosen disk tree is authoritative and new imports need no registry entry. |
| 2 | `measure(text, options)` uses the renderer's shaper and comp font roles/root; returns character/word boxes. `line()` and a dedicated caret helper remain open. |
| 3 | `sdk build/render --shot id` isolates a shot at zero, including matching overlays/local hits, camera and effects. Sheet times are local automatically. Incoming transitions and film-level layers/hits are omitted explicitly; preview the full film for those. |
| 4 | SDK records script-root assets in exported JSON, and Plan resolves `root` before block expansion, media/fonts or shader discovery. |
| 5 | `sdk check --sdk-dir ... --tsc <installed compiler> [--json]` performs real strict TypeScript checking without executing the film or installing packages. Build-time stripping remains separate. |
| 6 | `comp inspect --layer shot/layer` traverses evaluated nested frames; output coordinates/time belong to the containing comp. Inactive parent paths fail with an actionable time error. SDK build still emits the authored timeline. |
| 11 | Numeric vector scale and native colour tween/key forms work; shape-item trim, rotation, scale, opacity, fill and stroke tween (rect size is animated through item `scale` about `anchor`). |
| 12–13 | `overwrite: "auto"` preserves exact interrupted easing for scalar/vector/colour properties through native expressions. Default overlapping tweens remain errors. Shape-item pivots already exist in core and now have a runnable catalogue example.  |
| 14–16 | Existing core supports typewriter `dur` and counted spaces, tempo inheritance and default cameras for 3D shots. These are not new higher-level SDK caret APIs.  |
| 17 | Typed SDK `letterbox: [pictureWidth, pictureHeight]`; flat compositions and timeline compositions apply the bars. |
| 18–19 | SDK README documents roots, local shot previews, measured text, vector/colour and overwrite examples. Thirteen complete property-example compositions now cover scalar/vector/colour forms, radial gradients and item pivots; all load and evaluate in tests.  |
| 20–22 | `check --cost` reports transparent per-comp effect/glow surface estimates, with no GPU-time claim. Transition-cover warnings are now `check` rule `transition-covers-last-beat`; the reference/crop sheet is `comp sheet --ref --crop` (not wired into `sdk render`). The separate refstudy tool owns reference comparison.  |
| 23–24 | Pitched kit-cue integration and remaining high-level helpers are outside this engine pass.  |

Additional fix: composition render intervals now mix precisely their film-time audio interval into zero-based WAVs;
source offsets, fades and volume automation agree with the corresponding slice of the complete soundtrack.

Parenting clarification from generator visual review: a `null` parent passes its transform matrix only.
Its timing, hidden flag, opacity, masks and effects do not cascade to sibling child layers. Use a real
precomposition to share pixel timing/effects. `sdk/README.md` includes the explicit raw-precomp pattern.

## Status audit — 2026-10-26

Every item above carries a Status, checked against the code: **21 done, 3 partial, 0 open**
(partial: 2, 8, 14). Engine items proved by a test or command in this pass: 3 (property forms and error text,
`anim::prop` tests, catalogue test), 10 (glass-panel and `aura` examples in `runnable_property_examples`),
15 (`plan` test), 20 (`check::cover` tests), 21 (`comp check --cost`), 22 (`comp sheet --ref`, `media::refframe` tests),
6 (`comp inspect --layer a/t`).

Glow and blur cost: a hand-made reduced-resolution blur for radius above 16 px was built, matched the full blur to
4/255 and then **removed**: Skia's blur already filters wide radii at reduced size, and on a glow-heavy 1080x1920 film
(three large glows plus a growing blur, 90 frames) five alternating single-worker runs gave 4.1-4.8 s for both, no
measurable speed-up. The cost of a big glow is the full-size half-float layers each effect allocates, not the blur
kernel; use `check --cost` to find them, and limit a glow to a smaller layer or a precomp.

