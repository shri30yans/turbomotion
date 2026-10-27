# GAP: 3D portrait carousel (reference rebuild)

Date: 2026-10-27. Film: `motions/carousel-3d/`. Engine binary: `/tmp/showcase-bin/target/release/turbomotion` (git-archive HEAD, no crate edits). No engine hacks.

Reference: `motions/carousel-3d/study/reference.mp4` (960×540 / 30 fps / 15.02 s). Study: `motions/carousel-3d/study/study.md`.

## Can / cannot today

| Need | Status | How / why |
|------|--------|-----------|
| 3D card planes + camera | **Can** | `three_d` shape/text + `camera` + parent `group` cylinder |
| Cover-flow / vertical ring motion | **Can** | Per-card `pos`/`rotation` expressions on a ring |
| Depth order (non-intersecting) | **Can** | Consecutive `three_d` depth sort (`render/mod.rs`) |
| Approximate DOF | **Partial** | `camera.dof {focus, blur}` → whole-layer Gaussian by \|depth−focus\|/focus |
| Phone bezel clip | **Can** | `clip_to` screen plate |
| Photographic plates | **Stand-in** | Coloured silhouettes labelled STAND-IN (no photo budget) |
| True optical DOF / iris / focus distance | **Cannot** | Needs depth buffer → **WP-24** (depends WP-22) |
| Intersecting / hinged depth compositing | **Cannot** (not required by this ref’s non-crossing cards) | **WP-22** |
| Per-pixel mesh / textured materials | **Cannot** | Painter’s algorithm + flat materials → **WP-20**, **WP-21** |
| Env / chrome reflections on cards | **N/A for this ref** (matte plates); floor `reflect` only | **WP-21** if needed later |
| Soft cast shadows | **N/A for this ref** (none in source) | **WP-23** |

## Residual diffs (measured)

`refstudy diff` ref vs `out/film.mp4` (`out/diff/diff.json`), times 1 / 3 / 7 / 11:

| t | region_ssim_mean | layout_score | color_distance | edge_ref→ours | motion_ref→ours |
|--:|-----------------:|-------------:|---------------:|---------------:|----------------:|
| 1.0 | 0.403 | 0.053 | 0.145 | 0.089→0.027 | 0.127→0.036 |
| 3.0 | 0.296 | 0.000 | 0.318 | 0.085→0.014 | 0.186→0.034 |
| 7.0 | 0.668 | 0.500 | 0.126 | 0.052→0.052 | 0.054→0.035 |
| 11.0 | 0.425 | 0.188 | 0.083 | 0.258→0.115 | 0.015→0.005 |

Best structural match: **t=7** (ssim 0.668, edges match). Worst: free-float **t=3** (scatter + photo edges).

Budget: `pass: false`, `warn_fraction: 1.0`, floor failed at all sampled times. Dominant cause: **photo content / edge detail**, not missing camera or ring math. Side-by-side: `motions/carousel-3d/out/side_by_side.mp4`.

Structural gaps still visible after stand-in discount:

1. **DOF quality** — ref has strong soft falloff on ±1/±2 neighbors; ours is uniform Gaussian on the whole layer (no iris shape, no highlight bokeh). Repro: `sdk inspect film.ts --t 7` → `extra_blur` scales with \|depth−focus\|; hero focus must be set to ≈ `zoom − radius` or every card softens.
2. **Free-float scatter** — ref cards sit on a loose volume (uneven Y, multi-orbit); ours is one clean horizontal cylinder + one drift card. Layout_score 0.053 at t=1.
3. **Hero cadence** — ref eases card-to-card ~0.6–0.8 s; ours continuous `time*spin` (no hold). Motion energy lower than ref (e.g. t=1 motion 0.13→0.04).
4. **Phone inner scale / cover-flow angle** — ref neighbors ~25–40° Y; ours depends on ring radius/count; clip works but edge density still low (stand-ins).

## Engine work proposal (WP-20…24 sizing)

Do **not** start Phase E until Phase B + WP-30…32 are done (`BUILD_PLAN.md`). For *this* film’s blockers, priority if 3D work opens:

| WP | Size | Why this film cares | Repro without hacks |
|----|------|---------------------|---------------------|
| **WP-24** Camera / DoF v2 | **M** (depends WP-02, WP-22) | Biggest look gap vs ref’s shallow focus | Compare neighbor softness at t=7; need per-pixel CoC from depth, `f_stop`, `focus_distance` |
| **WP-22** Depth compositing | **XL** (depends WP-20) | Not blocking this non-crossing carousel; required before honest WP-24 | Two crossing planes (templates 003/018) |
| **WP-20** Mesh rasteriser / G-buffer | **XL** | Unlocks WP-22/24; textured card faces later | Cube golden + intersecting quads |
| **WP-21** Materials | **L** | Optional polish (glossy phone bezel); cards are matte | Chrome/glass probes |
| **WP-23** Lights / shadows | **L** | Ref has **no** cast shadows — skip for carousel | — |

**Recommendation:** keep authoring on current `three_d` + layer DOF. **Do not start WP-20 now** solely for this clip — the film already maps to existing layers/camera; residual error is mostly plates + DOF quality gated on WP-22→24. When Phase E opens, **WP-20 → WP-22 → WP-24** is the path that upgrades this film; WP-21/23 are lower for this reference.

## Check / render gate

```
sdk check motions/carousel-3d/film.ts  → passed, 0 warnings
out/film.mp4  960×540 30fps ~14.9s (447 frames)
```
