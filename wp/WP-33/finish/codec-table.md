| encode | rounding | distinct luma values | distinct 16-column means | max step between 16-column means (luma levels) | luma of the ramp's ends |
|---|---|---|---|---|---|
| libx264 crf 0 (lossless) | none | 15 | 29 | 1.000 | 0.0 to 16.0 |
| libx264 crf 0 (lossless) | ordered | 15 | 72 | 0.414 | 0.1 to 15.9 |
| libx264 crf 0 (lossless) | blue_noise | 15 | 72 | 0.415 | 0.1 to 15.9 |
| libx264 crf 18 | none | 15 | 29 | 1.000 | 0.0 to 16.0 |
| libx264 crf 18 | ordered | 15 | 29 | 1.000 | 0.0 to 16.0 |
| libx264 crf 18 | blue_noise | 15 | 29 | 1.011 | 0.0 to 16.0 |
| h264_videotoolbox 20 Mbit/s | none | 15 | 30 | 1.000 | 0.0 to 16.0 |
| h264_videotoolbox 20 Mbit/s | ordered | 15 | 30 | 1.000 | 0.0 to 16.0 |
| h264_videotoolbox 20 Mbit/s | blue_noise | 15 | 16 | 2.000 | 0.0 to 16.0 |
