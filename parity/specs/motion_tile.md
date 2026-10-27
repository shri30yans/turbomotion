# Motion Tile (`motion_tile`)

Written in my own words (BUILD_PLAN section 4, step 2). Sources: the After Effects help text for Motion Tile as I remember
it (parameter names, units, the idea of "output width and height as a percentage of the input"), and the behaviour of
`tmp/clones/effectcraft` (`crates/effects/src/stylize2.rs::motion_tile` and `lib.rs::motion_tile_frame`, read only, nothing
copied), which the plan tells me to treat as a hypothesis. **No After Effects dump is available here, so none of what follows
is a measurement.** Every rule below is a **definition** I implemented; the ones that rest on a guess are marked `UNSURE`.

## What it does

Motion Tile builds an output rectangle from copies of the layer. The layer is cut into nothing: every **tile** is the whole
layer drawn at the tile's size, tiles are laid on a grid, and the output shows as much of that grid as its size says. The
output is centred on the layer, so asking for more than 100 % grows the layer on every side (this is the effect that makes a
layer larger than its source), and less than 100 % crops it.

## Parameters

Units are the layer's own pixels and percentages of its width (x) and height (y). `box` is the layer's content box (`resolution`
in the shader): the layer's own size, not the size of a buffer that earlier effects grew.

| name | AE name | unit | default | range | notes |
| --- | --- | --- | --- | --- | --- |
| `tile_center` | Tile Center | share of box, `[x, y]` | `[0.5, 0.5]` | any | where tile (0, 0) is centred. **Differs from AE**: AE counts pixels and defaults to the layer's centre; a plugin default cannot depend on the layer's size, and a share stays at the middle when the layer is resized (as `barrel`, `twirl` and `vanish` do). `pixels = share * box` |
| `tile_width` | Tile Width | % of box width | 100 | 1 to 1000 | width of one tile; the whole layer is drawn into it |
| `tile_height` | Tile Height | % of box height | 100 | 1 to 1000 | |
| `output_width` | Output Width | % of box width | 100 | 1 to 1000 | width of the result, centred on the layer. AE's range is wider (32 000 %); a layer's picture is limited to 16384 px a side and 64 Mpx (`eval/buffer.rs`), which 1000 % of a layer wider than 1638 px exceeds; that is an error naming the layer (`an_output_that_would_make_the_picture_too_big_...`) |
| `output_height` | Output Height | % of box height | 100 | 1 to 1000 | |
| `mirror_edges` | Mirror Edges | choice `off`, `on` | `off` | | every other tile is flipped (see Formula). The engine has no checkbox parameter type; a choice named `off`/`on` is how `displacement_map` writes its switches |
| `phase` | Phase | degrees | 0 | -36000 to 36000 | `UNSURE`, see Formula. 360 is a whole tile |
| `horizontal_phase_shift` | Horizontal Phase Shift | choice `off`, `on` | `off` | | `on`: the phase shifts rows sideways; `off`: it shifts columns up and down |
| `gutter_size` | (none) | px | 0 | 0 to 1000 | **Not in After Effects.** `BUILD_PLAN` section 7 (WP-34) lists "gutter colour/size" for Motion Tile. Written as the plan says, default 0 so a use without it is After Effects' |
| `gutter_color` | (none) | colour | transparent | | fills the gaps; **not in AE** |

Out of range values are held in range (the engine's rule for every plugin parameter, `eval/fx.rs`); a misspelt name or option
is an error that lists the valid ones. All parameters but the two switches are properties and animate (keys, eases, expressions).

## Formula

Let `u` be a pixel centre in layer pixels (`x + 0.5`), `W, H` the box, `c = tile_center * (W, H)`, `t = (W, H) * (tile_width, tile_height) / 100`
the tile size, `g = gutter_size`, and `p = t + g` the pitch from one tile to the next.

1. **Output.** `d_o = u - (W, H) / 2`. If `|d_o.x| > W * output_width / 200` or `|d_o.y| > H * output_height / 200` the pixel is
   transparent (the crop; a pixel is in when its centre is).
2. **Tile number.** `d = u - c`. Tile `(0, 0)` is centred on `c`; tile `n` is `n * p` away. With `n = floor(d / p + 1/2)` per axis
   and `local = d - n * p`, a pixel is in the tile when `|local| <= t / 2` on both axes and in the **gutter** otherwise (it takes
   `gutter_color`; the test is skipped when `gutter_size` is 0, so no float error can ever open a gap).
3. **Phase** (`UNSURE`). With `s = phase / 360`: if `horizontal_phase_shift` is on, the row `n.y` is found first and the
   position is moved `d.x += s * n.y * p.x` before `n.x` is found; if off, the column `n.x` is found first and `d.y += s * n.x * p.y`
   moves the position before `n.y` is found. So each row (column) is shifted `s` pitches further than the one before: 180 degrees
   gives a brick pattern, 90 degrees a staircase of quarter-tile steps, 360 degrees a whole tile and so no change. A positive
   phase moves the content of row `j` to the left by `j * s` pitches (rows below the middle row left, above it right).
   *This is EffectCraft's rule. The other reading, "every other row is shifted by the phase" (so 90 degrees would shift odd rows
   a quarter and leave even rows alone), agrees with this one at 0, 180 and 360 degrees and disagrees elsewhere; I have no AE
   measurement to choose by.*
4. **Position in the tile.** `a = local / t + 1/2`, 0 to 1 across the tile. With `mirror_edges`, `a` becomes `1 - a` along an axis
   whose tile number `n` is odd (the middle tile is 0, so it is never flipped; negative odd numbers flip too).
5. **Colour.** The layer's picture read at `a * (W, H)`, **filtered** as below.

### Filtering (what "does not bleed" means here)

* A tile is the whole layer at `t`, so one tile pixel covers `r = 100 / (tile_width, tile_height)` layer pixels along each axis.
* The colour is the **average of a grid of bilinear taps** spread evenly over that footprint: `ceil(r)` taps along an axis,
  at least 1 (a tile as big as the layer or bigger is one bilinear tap) and at most **5** (`MAX_TAPS`, 25 taps in all). A
  reduction of up to 5:1 is a true box average; a smaller tile than that still averages 5 x 5 taps spread over its footprint,
  an unbiased but coarse estimate that aliases a little on very fine detail. A reduction of exactly 2:1 puts the taps on the
  centres of the two texels it covers, so the result is their exact mean (`a_reduced_tile_is_the_average_...`).
* Every tap is **held at least half a pixel inside the layer** (`clamp(p, 0.5, box - 0.5)`). A bilinear tap at the layer's edge
  would otherwise reach the empty space beside the layer and make the first and last pixel of every tile partly transparent
  (a visible seam line when the tile centre has a fractional part). Held taps read the edge texel instead, so a tile edge is as
  opaque as the layer's own, and a seam never blends in the next tile's pixels. The price: the edge texel row is repeated for
  at most half a pixel at each tile edge.
* Averaging and interpolation are done in the encoding the film is in: the plugin declares `@space linear`, so in a
  `linear_light` film they average **light** (two texels of 224 and 32 give 187, not 128), and in an ordinary film they average
  the encoded numbers. Colours are premultiplied while averaged.

## Edge cases

* Neutral parameters (tile 100 %, output 100 %, centre `[0.5, 0.5]`, phase 0, gutter 0) return the layer byte for byte, in
  both effect spaces (`motion_tile_with_neutral_parameters_...`).
* A tile centre off the layer's middle moves the **whole grid** (cells included) with it; since the tile is the whole layer
  this reads as the layer moving, and wrapping around seamlessly at tile 100 % (`moving_the_centre_by_whole_pixels_scrolls...`).
* A tile larger than the output leaves the middle of one tile (or parts of several) in the output; a tile of 1 % of a small layer
  is finite and averaged as above.
* Output below 100 % crops the layer's own picture; the picture does not shrink (buffers only grow, `eval/buffer.rs`), the cut
  away part is transparent.
* An output above 100 % in `effect_space: screen` is clipped to the frame like every screen-space effect: the layer's picture there
  is the frame, so the output is `output_width` % of the **frame**, centred on it, and what falls outside the frame is not drawn.
* In `effect_space: layer` the picture grows (below). One that would pass 16384 px a side or 64 Mpx is an error from the layer's
  evaluation naming the layer and the size (1000 % of a 1920 px layer is 19200 px); lower the output or the layer's size.
* A layer whose earlier effects grew its picture (a blur before the tile) is tiled by its content box only; the halo is not read
  (taps are held inside the box).
* Only the content box is tiled, so for a text layer the tile is the measured text box (`resolution` is the content box).

## Bounds

Grows bounds: **yes**, in layer space, by half the extra output on each side, from the layer's own box (not the running box of
earlier effects, which are summed separately):

`@bounds expand_x=box_w*(output_width-100)/200 expand_y=box_h*(output_height-100)/200`

`box_w` and `box_h` are new names in the `@bounds` grammar (the layer's content width and height, `fxlib/bounds.rs`), the first
growth that is a share of the layer and not a number of pixels. A negative result (an output below 100 %) grows nothing. Rounded
out to whole pixels like every growth, so a 32 x 16 layer at 300 % is a 96 x 48 picture, origin (-32, -16)
(`the_picture_grows_by_half_the_extra_output_on_each_side`). Animated output sizes re-plan the picture every frame.

## Colour

Linear light: **yes** (`@space linear`): the filtering above is done on light in a `linear_light` film (ledger `linear` = `yes`).
