# TypeScript authoring

Owns TypeScript stripping, deterministic QuickJS execution, SDK module selection and the `sdk` CLI.
It must not implement rendering or duplicate text shaping; those belong to `comp`.

```sh
CARGO_TARGET_DIR=target-engine cargo test --release -p turbomotion-script
target/release/turbomotion sdk build examples/sdk/launch/film.ts --sdk-dir sdk --shot hook
```

SDK options and preview semantics are documented in `sdk/README.md`. `measure.rs` bridges JSON to
`comp::text::measure`; `shot.rs` isolates timeline content before the normal composition pipeline.
