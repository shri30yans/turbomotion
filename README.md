# turbomotion

A native motion-graphics / compositing engine aimed at After Effects–class vocabulary and behaviour: layers, masks, mattes, effects, blends, colour, keyframes, text, 3D, audio — described in data, rendered deterministically to H.264 on macOS (Rust + [Skia](https://skia.org) on Metal). Authors write **TypeScript or JSON**; there is no timeline UI, no browser, and no Node runtime in the render path. The binary strips types, runs scripts on embedded QuickJS, and draws every frame on the GPU straight into the encoder.

This repository is the engine only. Films, reference studies and production kits live next door in [`../motions`](../motions) and talk to the engine through its CLI and file formats.

It is **not** pixel-matching After Effects. The goal (see [`docs/BUILD_PLAN.md`](docs/BUILD_PLAN.md)) is reachable AE names, order of operations and behaviour, asserted by tests and rebuilds — with an explicit “not built / decided against” list in [`docs/reference/parity.md`](docs/reference/parity.md) (`turbomotion parity gaps`).

## How it works

```
authoring          TypeScript (SDK) or hand-written comp.json
       ↓
compose / check    schema → plan → eval → lint / assertions / inspect
       ↓
render graph       paint layers → masks / mattes → effects → blend → colour → encode
       ↓
output             stills, contact sheets, or MP4
```

A frame is a pure function of time: `frame(t) = draw(evaluate(plan, t))`. That is what makes parallel segments, single-frame stills, scrubbing and golden tests possible.

| Crate | Role |
| --- | --- |
| [`crates/core`](crates/core) | Platform: GPU context, encoder, scheduler, SkSL runner, effects library, colour |
| [`crates/comp`](crates/comp) | Composition model: schema → plan → eval → render (layers, masks, text, particles, 3D, audio, check/sheet) |
| [`crates/script`](crates/script) | TypeScript authoring: strip types, QuickJS, emit composition; embeds the [SDK](crates/script/sdk) |
| [`crates/cli`](crates/cli) | The `turbomotion` binary: `comp`, `sdk`, `parity`, `docs` |

Inside `comp`, effects run in **screen** space by default (transform first, then effects on the framed picture) or **layer** space (`effect_space: "layer"` — AE order: grow bounds, masks/effects, then transform). Built-in and plugin effects share one composite path (mask + blend + mix). Shader plugins are SkSL files on named paths (`fx_path` / `TURBOMOTION_FX_PATH`); blocks are the same idea for parametrised sub-comps. Some blurs and related kernels keep a `quality: "legacy"` default for existing films and expose `quality: "exact"` for kernel-accurate light-preserving behaviour — opt in per effect when you need it.

Everything the engine loads is named, never discovered by walking parent folders: plugins, blocks, SDK (`--sdk-dir` or the copy compiled in), media relative to the composition root.

## Quickstart

```sh
cargo build --release

# What a composition can contain (JSON catalogue + examples)
target/release/turbomotion comp catalogue

# Validate a small example, then look at moments
target/release/turbomotion comp check  examples/comp/01-shapes/comp.json
target/release/turbomotion comp sheet  examples/comp/01-shapes/comp.json --times 0,1,2
target/release/turbomotion comp render examples/comp/showcase/comp.json --out showcase.mp4

# TypeScript film → sheet or MP4 (examples/sdk/launch/film.ts)
target/release/turbomotion sdk check  examples/sdk/launch/film.ts --schema
target/release/turbomotion sdk render examples/sdk/launch/film.ts --sheet sheet.png --times 0.5,1.5 --sheet-only
target/release/turbomotion sdk render examples/sdk/launch/film.ts --out launch.mp4

# Probe a single effect across its knobs
target/release/turbomotion comp fx test blur --on <photo.jpg>

cargo test --release --workspace
```

Useful companions: `comp inspect` / `comp graph` for one moment; `sdk new probe NAME` to scaffold an effect film; `docs --write` to refresh generated reference pages (CI fails if they drift).

## Layout

```
crates/          core, comp, script, cli (dependency order: core ← comp ← script ← cli)
docs/            ARCHITECTURE.md (contract), BUILD_PLAN.md (work packages), reference/ (generated)
examples/        small runnable compositions and SDK films, one capability at a time
wp/              work-package probes, identity tables, reports
```

## Where to read next

1. [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — ownership, stages, plugins, agent loop (`check` → `inspect` → `sheet`)
2. [`docs/BUILD_PLAN.md`](docs/BUILD_PLAN.md) — what “AE parity” means here, ranked work packages, everyday commands
3. [`crates/script/sdk/README.md`](crates/script/sdk/README.md) — TypeScript API, builders, tweens, components
4. [`docs/reference/`](docs/reference) — generated layer / timeline / effects / blend / SDK pages (`parity.md` for gaps)

Per-crate detail: [`crates/comp/README.md`](crates/comp/README.md), [`crates/script/README.md`](crates/script/README.md).
