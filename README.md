# turbomotion

A native motion-graphics engine: describe a video as JSON (or a TypeScript script that builds it), get an H.264 MP4.
No browser, no Node. Rust draws every frame with [Skia](https://skia.org) on the Mac's GPU (Metal) and pipes the pixels
straight into the encoder.

This folder is the engine and nothing else. It has no films, no reference material and no production pipeline; those
live in [`../motions`](../motions), which drives the engine through its command line and file formats only.

```
crates/core     the platform: GPU context, encoder, parallel scheduler, shader runner, effects library, colour
crates/comp     compositions: schema -> plan -> eval -> render (layers, masks, mattes, text, particles, 3D, audio)
crates/script   TypeScript authoring: strips types, runs the script on embedded QuickJS, emits a composition
crates/cli      the `turbomotion` binary: `comp` and `sdk` subcommands, nothing else
docs/           ARCHITECTURE.md (the contract), reference/ (generated), AGENT_FRICTION.md, agent-briefs/
examples/       small runnable compositions, one per capability
```

```sh
cargo build --release
target/release/turbomotion comp catalogue            # everything a composition can contain, with an example of each
target/release/turbomotion comp check  examples/comp/01-shapes/comp.json
target/release/turbomotion comp sheet  examples/comp/01-shapes/comp.json --times 0,1,2
target/release/turbomotion comp render examples/comp/showcase/comp.json --out showcase.mp4
cargo test --release --workspace
```

Everything the engine loads is named, never discovered: shader plugins from `fx_path` or `TURBOMOTION_FX_PATH`, blocks
from `block_path` or `TURBOMOTION_BLOCK_PATH`, the SDK from `--sdk-dir` (otherwise the copy compiled in). Nothing walks
parent folders.

The reference pages in [docs/reference](docs/reference) (layer kinds, timeline, effects, SDK) are generated from the code.
`target/release/turbomotion docs --write` refreshes them; a test and CI fail while they are stale, so they cannot drift.

Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) before changing anything; per-crate detail is in
[crates/comp/README.md](crates/comp/README.md) and [crates/script/README.md](crates/script/README.md).
