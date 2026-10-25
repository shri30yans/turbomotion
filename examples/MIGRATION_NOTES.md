# Migration notes: the legacy X and Y rotation keys in the examples

Old order Rz . Ry . Rx; new `rotation: [x, y, z]` is Rx . Ry . Rz. There was no old render of the `comp/` examples and no old binary, so
the old look is rebuilt from nested single-axis nulls (Z outermost, then Y, then X) and compared with the new render.

| Example | Axes | Method | Check |
|---|---|---|---|
| `comp/06-3d`, `comp/showcase` | Y only, static | rename | exact |
| `comp/19-solids` box, torus, cylinder | X (static or `time` expression) and Y (`time` expression) | `orientation` carries Y, `rotation` carries X (Ry . Rx exactly), the sphere is a rename | pixel-identical to the nested reference at 4 times (max 0) |
| `comp/17-text3d` letters | X 80 to 0 and Y -140 to 0, back.out, animated | rename, by intent | below |
| `sdk/launch/film.ts` wall | X 25, Y -20, static | exact Euler `[26.3922, -18.0578, 8.7447]` (an earlier rename had left `[25, -20, 0]`) | mean difference to the old `film.mp4` after 2.4 s: 44 before, 1.7 after |
| `sdk/components/dolly-zoom.ts` boxes | X -15, Y 25, static | exact Euler `[-16.4703, 24.0929, 6.8817]` | exact Euler method; 9 poses (including near gimbal lock) rendered in the engine against nested single-axis nulls differ by at most 1/255 |
| `sdk/components/*.comp.json`, `sdk/launch/film.comp.json` | n/a | regenerated from the `.ts` | `comp check` 0 errors |

## 17-text3d, by intent

The letters tumble in and land on the identity pose, so the end pose is the same; the path in between turns X about Y first instead of
the reverse (left = old order, middle = new, right = difference times 3; identical from t = 3.0).

![text3d](migration/17-text3d.jpg)

## Not changed here

`sdk/components/cubes.ts` (the tipped puzzle and letter with a Y spin, `[-22, 35, 0]`, `[-12, -20, 0]`, and the tilted icon cube) and the
`swarm` component are animated multi-axis poses that were already renamed in the `.ts` by the upgrade; they turn in the new order and so do
not reproduce the old comp.json exactly (the old Ry . Rx wobble became a clean turntable). They are left as authored. The engine SDK
components under `crates/script/sdk` were not touched.

Cameras without a zoom (the default camera, `width / 0.72` now): `sdk/launch` (orbit move), `sdk/components` `flip-tile`, `swarm`, `cubes` (default camera).
