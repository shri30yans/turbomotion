// Chapter 9c's test picture: a black to #101010 ramp (levels 0 to 16) across the frame, in a film that rounds the finished
// frame with `dither`. Two films use it (none and blue_noise); `build.sh` takes a still of each and the chapter compares them.

import { shape } from "turbomotion";
import type { Film } from "turbomotion";
import { newFilm } from "./theme";

export function rampFilm(dither: "none" | "ordered" | "blue_noise"): Film {
  const film = newFilm(1, { bg: "#000000", color: { dither } });
  film.add(
    shape(
      [{ rect: [0, 0, 1920, 1080], fill: { gradient: { type: "linear", from: [96, 0], to: [1824, 0], stops: [[0, "#000000"], [1, "#101010"]] } } }],
      { id: "ramp" },
    ),
  );
  return film;
}
