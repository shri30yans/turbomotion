The same four panels rendered twice, once as always (`comp_default.json`, `default.png`) and once with `"color": {"linear_light": true}`
(`comp_linear.json`, `linear.png`); `compare.png` has the default on the left and linear light on the right. Numbers are the
red channel read from the stills at t = 0.5 s (960x540).

- `compare.png` panel 1, "50% white": a white layer at 50% opacity over black is level 128 by default and 188 in linear light (physical value 187.5).
- `compare.png` panel 2, "blur edge": a radius 24 blur across a black/white edge reads 125 and 130 on the two pixels beside the centre by default and 186 and 189 in linear light, the same edge 60 levels brighter because it averages light.
- `compare.png` panel 3, "ramp": a black to white gradient is level 64 a quarter of the way along and 128 halfway by default, 138 and 188 in linear light, 255 at the end in both.
- `compare.png` panel 4, "add 2 x 128": two `#808080` layers added is 255 by default (clipped) and 176 in linear light (physical value 175.6); the lone grey is 128 in both.
- `default.png` / `linear.png`: the two stills `compare.png` joins; `comp check` says `ok` with 0 errors and 0 warnings on both films.
