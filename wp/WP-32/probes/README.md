# WP-32 step 1 probes: text bounds and the text `box`

Four films, one script. `python3 make.py $PWD/../../../target/release/turbomotion` writes the films, renders them, scans the
drawn box out of the pixels and writes `data.tsv` (measured px, drawn px, difference; **worst 0.014 px over 21 rows**).
Fonts are the bundled Manrope (variable), Instrument Serif and JetBrains Mono. Each film passes `comp check` with 0 errors
and 0 warnings, and renders to the same bytes twice (`shasum` of the software-codec mp4, frames 0-45, one worker).

| image | what it shows |
| --- | --- |
| `fonts.png` | Three boxes of 632.38, 483.78 and 689.60 px (= the shaped "AVATAR Wave" + 2 x 28), each within 0.003 px of the pixels, in three fonts, and a two-line label whose per-line boxes are 230.02 and 392.31 px wide (0.002 px from the drawn ones). |
| `typed.png` | A typewriter bar at 0.1, 0.26, 0.5, 0.86, 1.22, 1.7 and 2.5 s: no box, then 110.71, 219.72, 345.89, 517.27 and 596.25 px, each equal to the same text laid out alone (0.003 px at most) and never narrower than the step before. |
| `layer-space.png` | `effect_space: "layer"`, blur 2: the scale 1 box is 266.40 px, the scale 2 box 532.79 px (ratio 2.000), and the third (scale 2, turned 30 degrees) turns about its anchor as one piece with its text. |
| `fit.png` | `min_width: 360` gives three 360.00 px bars that keep the left edge (476.0), the middle and the right edge; `radius` 0, 24 and 1000 give a square, a rounded card and a pill. |
| `keyed-padding.png` | A padding keyed from [4, 0] to [52, 24] over 2 s at 0, 1 and 2 s: the box is 201.36, 249.36 and 297.36 px wide (193.36 of text + 2 x 4, 28 and 52). |

Open the images to see them: every box hugs its text with the padding named, the typed bar has equal margins left and
right at every keystroke (the kerning to the next, hidden character is not in it), and the rotated box and its text turn together.

`data.tsv`: `probe`, `what`, `measured_px` (what `comp inspect` or the same text laid out alone says), `drawn_px` (scanned
from the PNG along the row through the middle of the box: first to last covered pixel, edge pixels by their coverage),
`abs_diff_px`.
