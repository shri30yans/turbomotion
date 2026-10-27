// Chapter 8, first film: motion blur with a 180 degree shutter (After Effects' default), against no blur.

import { timeline } from "turbomotion";
import { C, T0, frame, newFilm } from "./theme";
import { mblurScene } from "./mblur-scene";

const DUR = 4.8;
const tl = timeline();
const film = newFilm(DUR, { motion_blur: { shutter_angle: 180, shutter_phase: -90, samples: 16 } });
mblurScene(film, tl, 180, T0);
frame(film, tl, {
  num: "08",
  title: "Motion blur",
  sub: "After Effects' shutter: angle, phase, samples.",
  caption: "Shutter 180: each frame is averaged over half a frame of motion, so the slider and the propeller smear along their path.",
  hot: ["180", "smear"],
  dur: DUR,
  accent: C.orange,
});
export default film;
