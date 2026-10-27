# Feature showcase (work in progress)

TypeScript chapter films for a single demo reel. Each `chNN-*.ts` file is one **segment** of the video (not an engine concept). Shared chrome lives in `theme.ts`.

**Status:** sources were written by an automated run; the final concatenated MP4, contact sheet, and render script were **not** finished (worker hit a model limit). Build each chapter with the SDK, render with `turbomotion render`, then concat with ffmpeg.

**Chapters:** 1 layer-space, 2 colour, 3 blend modes, 4 motion paths, 5 text box, 6 palette/taper, 7 motion tile, 8 motion blur (two films), 9 grain/halation/dither ramps.
