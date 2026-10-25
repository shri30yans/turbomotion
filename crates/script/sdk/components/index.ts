// Reusable film components: each returns `{ layers, end, ...named parts }` and registers its motion and sounds on the
// timeline you pass in. Put one straight into `shot({ layers: [component] })`.

export type { Part, Theme } from "./util";
export { THEME, monoWidth, MONO_ADVANCE } from "./util";
export { promptBox } from "./promptBox";
export type { PromptBoxOpts } from "./promptBox";
export { codePanel, highlight } from "./codePanel";
export type { CodePanelOpts } from "./codePanel";
export { browserWindow } from "./browserWindow";
export type { BrowserWindowOpts } from "./browserWindow";
export { statCounter } from "./statCounter";
export type { StatCounterOpts } from "./statCounter";
export { barRace } from "./barRace";
export type { Bar, BarRaceOpts } from "./barRace";
export { bboxOverlay, stateColor } from "./bboxOverlay";
export type { Box, BoxState, BboxOverlayOpts } from "./bboxOverlay";
export { cardWall } from "./cardWall";
export type { CardWallOpts } from "./cardWall";
export { title3d } from "./title3d";
export type { Title3DOpts } from "./title3d";
export { scribble } from "./scribble";
export type { ScribbleOpts } from "./scribble";
export { waveform } from "./waveform";
export type { WaveformOpts } from "./waveform";
export { textStack } from "./textStack";
export type { StackItem, StackRole, TextStackOpts } from "./textStack";
export { cutout } from "./cutout";
export type { CutoutOpts, Idle, Wave } from "./cutout";
