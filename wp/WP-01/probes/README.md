# WP-01 probe: layer-space effects

`comp.json` is one film of ten precomps, five scenarios in columns, each drawn twice: the top row (orange rule) is
`"effect_space": "screen"`, the bottom row (cyan rule) `"layer"`. Render with
`turbomotion comp still wp/WP-01/probes/comp.json --out <dir> --times 1` (`comp check`: 0 errors, 0 warnings; two renders have
the same sha256, `3fe990e13edf2a6a...`). The image is `layer_space_probe.png`.

1. **scale** (250 px cell): a 100 px square (40 px at scale 2.5) with `blur` radius 8 spreads to 136 px across in screen space
   (18 px a side) and 192 px in layer space (46 px a side, 2.55 times as much): the blur scales with the layer.
2. **rotate**: a 100x24 bar rotated -35 degrees with a 70 px `directional_blur` at angle 0 smears along the frame's x in
   screen space (166 x 76 box) and along the bar in layer space (152 x 118 box): the smear turns with the layer.
3. **offscreen**: a 120 px square centred 75 px left of the frame with `glow` radius 18: screen space draws nothing (0 lit
   pixels), layer space lets the glow bleed 29 columns into the frame.
4. **glitch** (triage 002): "BURN" with `glitch_slices` amount 36: in screen space the bands are frame-height bands and the
   word stays whole in a 179 px wide drawing; in layer space the bands cut the word itself and the slices fly out to 249 px
   (the cell's whole width) without being clipped at the text box.
5. **echo** (triage 005): "TIME" rotated -20 with `echo` travel 60 at angle 0: in screen space the trail goes along the
   frame's x (198 x 91 box); in layer space along the text's own baseline (196 x 104 box, angle is in layer degrees).
