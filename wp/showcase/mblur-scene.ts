// Chapter 8's scene, shared by its two films (a film has one shutter, so 180 and 360 degrees are two films): the same
// fast slider and propeller twice, left without motion blur and right with the layer switch on.

import { motionPath, rect, shape, timeline } from "turbomotion";
import type { Film } from "turbomotion";
import { C, border, tag } from "./theme";

const PW = 840;
const PH = 650;
const Y0 = 200;

export function mblurScene(film: Film, tl: ReturnType<typeof timeline>, angle: number, at: number): void {
  const panels: Array<[string, number, boolean, string]> = [
    ["sharp", 96, false, "mblur: false"],
    ["blur", 984, true, `mblur: true, shutter ${angle}`],
  ];
  for (const [id, x0, on, label] of panels) {
    film.add(rect(PW, PH, { id: `bg-${id}`, fill: C.panel, pos: [x0 + PW / 2, Y0 + PH / 2] }), border(`bd-${id}`, x0, Y0, PW, PH));
    film.add(tag(`lab-${id}`, label, x0, Y0 - 28, on ? C.cyan : C.orange));
    // 540 px in 0.28 s, back and forth: about 1930 px/s, 64 px a frame
    const slider = shape([{ rect: [-45, -45, 90, 90], radius: 12, fill: C.orange }], {
      id: `slider-${id}`, mblur: on,
      pos: motionPath([{ t: at, v: [x0 + 140, Y0 + 130], ease: "sine.inOut" }, { t: at + 0.28, v: [x0 + 700, Y0 + 130], ease: "sine.inOut" }], { loop: "pingpong" }),
    });
    // a propeller at 900 degrees a second: 30 degrees a frame
    const prop = shape(
      [
        { rect: [-190, -16, 380, 32], radius: 16, fill: C.cyan },
        { rect: [-16, -190, 32, 380], radius: 16, fill: C.cyan },
        { ellipse: [0, 0, 30, 30], fill: "#ffffff" },
      ],
      { id: `prop-${id}`, pos: [x0 + PW / 2, Y0 + 440], mblur: on },
    );
    tl.from(prop, { rotation: -5400, duration: 6, ease: "none" }, 0);
    film.add(slider, prop);
  }
}
