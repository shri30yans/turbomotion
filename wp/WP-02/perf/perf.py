#!/usr/bin/env python3
"""Render time of a 1080p film with one blur, legacy (the baseline binary) against exact (this tree's binary).

    python3 wp/WP-02/perf/perf.py BASELINE_BIN NEW_BIN > wp/WP-02/perf/perf.tsv

24 frames of 1920 x 1080 with a software encode and one worker, three runs each; the median of the wall times is printed in ms per frame.
The film is a few shapes on a gradient, blurred by one `blur` effect on the layer, so the blur is most of what the frame costs."""
import json, statistics, subprocess, sys, tempfile, time
from pathlib import Path

FRAMES, RUNS = 24, 3
shapes = [{"ellipse": [200 + 300 * i, 300 + 120 * (i % 3), 160, 160], "fill": ["#ff5533", "#33aaff", "#ffffff"][i % 3]} for i in range(6)]


def film(fx):
    return {"width": 1920, "height": 1080, "fps": 24, "dur": 1, "bg": "#101820", "layers": [{"type": "shape", "items": shapes, "fx": [fx]}]}


def ms_per_frame(binary, path, out):
    times = []
    for _ in range(RUNS):
        t = time.perf_counter()
        subprocess.run([binary, "comp", "render", str(path), "--from", "0", "--to", str(FRAMES), "--workers", "1", "--codec", "software", "--out", str(out)], check=True, capture_output=True)
        times.append(time.perf_counter() - t)
    return 1000 * statistics.median(times) / FRAMES


base, new = sys.argv[1], sys.argv[2]
tmp = Path(tempfile.mkdtemp(prefix="wp02-perf-"))
print("sigma\tlegacy_ms_per_frame\texact_ms_per_frame\tnone_ms_per_frame")
for sigma in (4, 16, 64):
    cells = []
    for binary, fx in ((base, {"effect": "blur", "radius": sigma}), (new, {"effect": "blur", "radius": sigma, "quality": "exact"})):
        p = tmp / f"{sigma}-{len(cells)}.comp.json"
        p.write_text(json.dumps(film(fx)))
        cells.append(ms_per_frame(binary, p, tmp / f"{sigma}-{len(cells)}.mp4"))
    p = tmp / "none.comp.json"
    p.write_text(json.dumps(film({"effect": "blur", "radius": 0})))
    print(f"{sigma}\t{cells[0]:.1f}\t{cells[1]:.1f}\t{ms_per_frame(base, p, tmp / 'none.mp4'):.1f}")
