# Feature showcase

TypeScript chapter films for a single demo reel (`showcase.mp4`, 1920×1080, 30 fps, ~64.7 s). Shared chrome lives in `theme.ts`. Contact sheet of mid-chapter stills: `contact-sheet.png`.

**Build / check / render** (engine binary from `git archive HEAD` into `/tmp/showcase-bin`, then):

```bash
BIN=/tmp/showcase-bin/target/release/turbomotion
$BIN sdk build chNN-….ts --out /tmp/out.comp.json
$BIN comp check /tmp/out.comp.json   # expect 0 errors, 0 warnings
$BIN comp render /tmp/out.comp.json --out ch.mp4 --codec software --workers 1
# dither ramps: add --crf 0 so the pattern survives the encode
```

## Timestamps (in `showcase.mp4`)

| t | Chapter | Source | Look for |
|---|---------|--------|----------|
| **0:00** | 01 Layer-space effects | `ch01-layer-space.ts` | Right column: blur grows with scale; lamp glow bleeds in from off-panel. Left (screen space): neither. |
| **0:06** | 02 Colour pipeline | `ch02-colour.ts` | Linear-light column: 50% white ≈ 188, blurred edge ≈ 190; display column stays near 128 / 131. |
| **0:12** | 03 Blend modes | `ch03-blend-modes.ts` | Page flip ~3.9 s into the chapter. Violet labels = 20 new modes; dissolve dither; stencil / silhouette cut-outs on page 2. |
| **0:18.5** | 04 Property model v2 | `ch04-motion-paths.ts` | Arrow banks on the bezier; cyan roving vs orange timed on one path; handle ease vs `sine.inOut`; pink hold steps. |
| **0:25** | 05 Text bounds | `ch05-text-box.ts` | Orange box grows with the typewriter + padding ease; cyan per-line boxes; lime `min_width` pill. |
| **0:30.8** | 06 Palette + taper | `ch06-palette-taper.ts` | Three palette cycles with animated offset; ribbon / ring / line stroke tapers. |
| **0:36.8** | 07 Motion Tile | `ch07-motion-tile.ts` | Mirrored “F” tiles meet without seams; right panel animates phase, tile size, centre. |
| **0:42.3** | 08 Motion blur 180° | `ch08a-motion-blur-180.ts` | Right panel: slider smear + propeller fan at shutter 180 / phase −90. |
| **0:47.1** | 08 Motion blur 360° | `ch08b-motion-blur-360.ts` | Same scene, shutter 360 / phase −180 — longer, centred smear (no title card). |
| **0:50.7** | 9a Add Grain | `ch09a-grain.ts` | clean → fine_35mm → coarse_16mm → vintage_8mm; read strength on the #808080 band. |
| **0:55.7** | 9b Film halation | `ch09b-halation.ts` | Right: lamps / BAR / headlight ring orange; grey lamp under threshold stays dark. |
| **1:00.7** | 9c Dither | `ch09c-ramp-none.ts` + `ch09c-ramp-blue-noise.ts` | Reel segment is a **gain ×16** still comparison (`ch09c-compare.png`): banding vs blue-noise on the same 0…16 ramp. (`color.dither` is root-only, so the two films cannot share one composition.) |

Ends **1:04.7**.

## Notes

- All chapter `sdk build` + `comp check` runs: **0 errors, 0 warnings**.
- Chapter encodes used `--codec software --workers 1` (dither ramps `--crf 0`).
- The raw 9c ramp films are nearly black on a normal display (levels 0–16 by design); the reel’s 9c segment amplifies for visibility and is labelled as such.
