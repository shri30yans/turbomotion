# Camera Lens Blur (`lens_blur`)

Written in my own words (BUILD_PLAN section 4, step 2). **No After Effects dump is available**; every rule is a definition implemented in
`core/src/kernel` (`shaders/kernel/iris.sksl`) and measured on the engine's own pixels.

## What it does

Every pixel becomes the mean of the pixels inside the lens's opening, an **iris**: a disc, or a polygon of `blades` sides with its corners at
`radius`, turned by `rotation`, its corners rounded toward a disc by `roundness`. A pixel's weight is how much of it the iris covers (one pixel
of linear falloff across the rim), so the radius is continuous and a disc is **flat-topped**: a lone point becomes a uniform disc of height
`1 / (pi r^2)`, not a bell. The mean is over the weights actually read, so no light is gained or lost.

* A polygon's corners reach `radius` and its sides `radius * cos(pi / blades)` (a hexagon's flat side is 10.39 px for 12;
  `a_polygon_iris_has_its_corners_at_the_radius_and_its_sides_nearer`). `roundness: 1` is the disc exactly.
* Cost is one pass over `(2 r + 1)^2` pixels, so `radius` is below 32 (`taps::MAX_IRIS_TAPS`); more is an error, never a narrower iris.
* **Reach** is `ceil(radius + 0.5)`; the planner's `disc_reach` is the same, conservative for a polygon.
* Not done (deferred, see the report): AE's Highlight Gain/Threshold/Saturation, Blur Map, Aspect and Fringe.

## Parameters

| name | unit | default | range |
| --- | --- | --- | --- |
| `radius` | px | 10 | 0 to 31.5 |
| `blades` | sides | 0 | 0 (disc), 3 to 16 |
| `rotation` | degrees | 0 | |
| `roundness` | | 0 | 0 to 1 |
| `edge` | choice | `transparent` | |

## Measured (`parity/probes/lens_blur.json`; `wp/WP-02/probes/data.tsv`)

| quantity | measured | closed form |
| --- | --- | --- |
| disc, 2 x 2 white square, plateau level (linear light), radius 4, 6, 8, 12 | 80, 53, 39, 23 | `encode(4 / (pi r^2))` = 79.7, 52.8, 38.6, 23.5 |
| the plateau, levels spread over 70% of the radius | 0 | flat top |
| light beyond radius + 2 px | none | |
| float energy (`a_disc_is_flat_topped_inside_its_radius_and_nothing_beyond_half_a_pixel_past_it`) | within 0.5% | 1 |
