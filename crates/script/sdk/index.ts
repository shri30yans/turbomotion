// turbomotion SDK: write a film in TypeScript, export it, and the binary renders it.
//
//   import { comp, text, timeline } from "turbomotion";
//   const film = comp({ width: 1920, height: 1080, fps: 30, bg: "#0b0d12" });
//   export default film;

export { comp, shot, Film, Shot } from "./comp";
export { Layer, text, solid, image, shape, rect, group, camera, light, raw } from "./layers";
export { textOnPath, orbitText, wrapText, marquee, charFlip, decode, outlineText } from "./text-fx";
export { mesh, box, sphere, torus, cylinder, extrude, text3d, letters3d, spin } from "./three";
export { timeline, Timeline } from "./timeline";
export { svg, svgFile, pixelSprite } from "./assets";
export type { SvgOpts, PixelOpts } from "./assets";
export { audio, hit, melody, drumPattern, scale, transpose } from "./sound";
export type { MelodyOpts, DrumOpts, Step } from "./sound";
export { stage, Stage } from "./stage";
export type { Pose, GoOptions } from "./stage";
export * from "./components/index";
export { rand, range, lerp, clamp, deg, grid, darken } from "./motion";
export type { Rand, Cell } from "./motion";
export * from "./types";

export { measure } from "./measure";
export type { MeasureOptions, Measured, CharBox, WordBox } from "./measure";
