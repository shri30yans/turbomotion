// Chapter 8, second film: a 360 degree shutter (a whole frame of smear, centred with phase -180), against no blur.

import { timeline } from "turbomotion";
import { C, frame, newFilm } from "./theme";
import { mblurScene } from "./mblur-scene";

const DUR = 3.6;
const tl = timeline();
const film = newFilm(DUR, { motion_blur: { shutter_angle: 360, shutter_phase: -180, samples: 16 } });
mblurScene(film, tl, 360, 0.3);
frame(film, tl, {
  num: "08",
  title: "Motion blur",
  sub: "",
  caption: "Shutter 360, phase -180: the smear doubles to a full frame and stays centred on the frame's own moment.",
  hot: ["360", "doubles"],
  dur: DUR,
  accent: C.orange,
  card: false,
});
export default film;
