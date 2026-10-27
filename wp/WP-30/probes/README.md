# WP-30 palette cycle and stroke taper probes

Ten films, one script. `python3 make.py $PWD/../../../target/release/turbomotion` writes the films, renders them, reads the
pixels and writes `data.tsv` (expected, measured and the difference for **each of its 27 rows**; the expected values are
worked out in the script, not read from the engine). Worst colour error **0.89 level** (limit 1); worst width error **0.158 px**
(limit 0.25). Each film passes `comp check` with 0 errors and 0 warnings; the three films rendered twice (software codec, frames 0 to 12, one worker) give the same sha256.

| image | what it shows |
| --- | --- |
| `palette-step.png` | Six rows of 8 copies over 5 colours, offsets 0, 1, -1, 2.5, -0.5 (reversed) and 7 (reversed): every one of the 48 copies is the expected palette colour to 0 levels. |
| `palette-gradient.png` | A gradient over 3 colours at offsets 0, 0.25 and -0.5 in an ordinary film: the mix of encoded levels, worst 0.50 level from the closed form. |
| `palette-gradient-light.png` | The same in a `linear_light` film: the mix of light, worst 0.49 level from the closed form. |
| `palette-pingpong-keyed.png` | Top row ping_pong over 4 colours: indices 0 1 2 3 2 1 0 1 exactly. Second row a layer repeater with the offset keyed from 0 to 4 over 1 s: at t = 0, 0.5 and 1 the copies take 0 1 2 0 1 2 0 1, then 2 0 1 2 0 1 2 0, then 1 2 0 1 2 0 1 2, exactly the expected indices. |
| `palette-opacity.png` | Palette with the repeater's opacity ramp 0.8 per copy over the background: worst 0.89 level (display) and 0.48 level (linear light). |
| `taper-straight.png` | A 840 px line, 30 wide, start 200 px to a point, end 160 px to 8 px, eases none, power2.out and sine.inOut: width at 8 distances within 0.09, 0.09 and 0.16 px of the profile (3 px from the start 0.45 against 0.50 measured; 837 px, 8.41 against 8.50). |
| `taper-curve.png` | A 1137.9 px cubic, 28 wide, start 30% and end 20% of its length as fractions, power2.inOut: nine widths from 0.01 to 0.99 of the path within 0.13 px. |
| `taper-closed.png` | A circle (1256.6 px round), 24 wide, start 250 px and end 150 px both to a point: the stroke pinches at the start point from both sides; seven widths within 0.10 px (5 px along 0.48 against 0.48). |
| `taper-clamp.png` | A 400 px line with both lengths 5000 px: held to 200 px each, 2, 10, 20, 10 and 2 px at 20, 100, 200, 300 and 380 px, error 0.000. |

`data.tsv`: `probe`, `what`, `expected`, `measured`, `abs_diff`. A palette row is the worst channel error over its copies (levels);
a taper row lists `distance:expected/measured` px and the worst error.
