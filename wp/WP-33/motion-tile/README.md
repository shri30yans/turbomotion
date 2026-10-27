# Motion Tile probes

Regenerate: `python3 wp/WP-33/motion-tile/make.py $PWD/target/release/turbomotion` (writes `parity/probes/motion_tile.json`, renders it, writes the PNGs and `data.tsv`).
Every number is the largest 8-bit level difference between the engine's render and a prediction made in numpy from the spec's formulas (`make.py::predict`), read from the engine's own render of the plain card; a rounding difference is 0.5.

| image | what it shows |
| --- | --- |
| `panels.png` | Six panels of one 160 x 96 four-quadrant card: 250 % tiles, mirror edges, 180-degree bricks, an 8 px white gutter, centre moved (+20, -10) px, 60 % crop; each matches the prediction to 0.00 to 0.50 levels over 93 600 px (5 568 px for the crop). |
| `layer-vs-screen.png` | A frame-sized card at tile 40 x 30 %, output 300 %, mirror, phase 90 and an off-centre tile centre, layer space left and screen space right: the two differ by 0.00 levels. |
| `scale2-quarter-turn.png` | The same 150 % mirrored 50 % tiling on a layer of scale 2 turned 90 degrees: it equals the unturned picture turned by `np.rot90` to 0.00 levels, so the pattern lives in the layer's own space. |
| `scale2-rotation30.png` | The same layer turned 30 degrees (scale 2): the tiled picture is one 480 x 288 px rectangle turned, edge to edge with no gaps. |
| `seam-subpixel.png` | 8x zoom of a 32 x 16 card at 300 % with the tile centre 0.3 px right: the five seam columns equal the card's first and last texel columns exactly (0.00 levels), the only blended column is the card's own red/green edge (0.3 red + 0.7 green), none at a tile seam. |

`data.tsv` has the table of numbers; `identity/` the before/after hash tables of the byte-identity proof.
