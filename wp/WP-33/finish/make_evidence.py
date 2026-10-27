#!/usr/bin/env python3
"""Measures the stills `make_films.py`' films render to (in /tmp/fs-ev, see REPORT-finish-stack.md for the commands) and writes
the images and data tables of this folder. Every number in the tables is computed here from the pixels."""
import pathlib, sys
import numpy as np
from PIL import Image

OUT = pathlib.Path(__file__).resolve().parent
EV = pathlib.Path("/tmp/fs-ev")


def load(name):
    return np.asarray(Image.open(EV / name / "0000.50.png").convert("RGB")).astype(np.float64)


def save(name, a):
    Image.fromarray(np.clip(a, 0, 255).astype(np.uint8)).save(OUT / name)


# ---- Add Grain: the three presets over a clean ramp ---------------------------------------------------------------
clean = load("add_grain")
rows = []
spec = {"fine_35mm": (0.6, 1.2, 0.15, 0.8, 0.45), "coarse_16mm": (1.1, 2.2, 0.3, 0.9, 0.5), "vintage_8mm": (1.8, 3.5, 0.45, 1.0, 0.7)}
# panel 0 has no effect: the other panels' ramps are identical to it, so the difference is the grain and its rounding
def smooth(l, lo, hi):
    t = np.clip((l - lo) / (hi - lo), 0, 1)
    return t * t * (3 - 2 * t)

def tone(l, shadows, mids, highs):
    dark, light = 1 - smooth(l, 0, 0.5), smooth(l, 0.5, 1)
    return shadows * dark + mids * (1 - dark - light) + highs * light

lines = ["| preset | brightness | measured std (levels) | spec std (levels) | measured/spec | R-G correlation | spec 1 - saturation |", "|---|---|---|---|---|---|---|"]
for i, name in enumerate(["fine_35mm", "coarse_16mm", "vintage_8mm"], start=1):
    panel, base = load("add_grain")[:, i * 320:(i + 1) * 320], clean[:, 0:320]
    diff = panel - base
    intensity, size, soft, shadows, highs = spec[name]
    for level in (0.2, 0.5, 0.8):
        y = int(level * 720)
        band = diff[y - 6:y + 6]
        std = band[..., 1].std()
        l = base[y, 0, 1] / 255.0
        want = np.sqrt((intensity * 0.04 * 255 * tone(l, shadows, 1.0, highs)) ** 2 + 1 / 12)
        corr = np.corrcoef(band[..., 0].ravel(), band[..., 1].ravel())[0, 1]
        sat = {"fine_35mm": 0.15, "coarse_16mm": 0.3, "vintage_8mm": 0.45}[name]
        lines.append(f"| {name} | {l:.2f} | {std:.2f} | {want:.2f} | {std / want:.3f} | {corr:.3f} | {1 - sat:.2f} |")
(OUT / "add_grain-table.md").write_text("\n".join(lines) + "\n")
full = load("add_grain")
save("add_grain-presets.png", full)
crop = np.concatenate([np.kron(full[360:424, i * 320 + 128:i * 320 + 192], np.ones((4, 4, 1))) for i in range(4)], axis=1)
save("add_grain-zoom.png", crop)

# ---- Halation: lamps with and without the effect --------------------------------------------------------------------
plain, halated = load("lamps-plain"), load("film_halation")
save("halation-before-after.png", np.concatenate([plain[:, :], halated[:, :]], axis=0))
lines = ["| measurement | value |", "|---|---|"]
col = halated[:, 280]  # straight up from the lamp's centre (the lamp's edge is at y = 290)
for label, y in [("10 px above the lamp", 280), ("40 px", 250), ("80 px", 210), ("160 px", 130)]:
    r, g, b = col[y]
    lines.append(f"| {label} (y={y}): R, G, B | {r:.0f}, {g:.0f}, {b:.0f} (plain frame: {plain[y, 280, 0]:.0f}, {plain[y, 280, 1]:.0f}, {plain[y, 280, 2]:.0f}) |")
added = col[250] - plain[250, 280]
lines.append(f"| halo added at 40 px, R : G : B | {added[0]:.0f} : {added[1]:.0f} : {added[2]:.0f} = 1 : {added[1] / added[0]:.3f} : {added[2] / added[0]:.3f} (the tint 255 : 90 : 40 is 1 : 0.353 : 0.157) |")
lines.append(f"| 25 px above the grey #666666 disc (0.40 as a number, under the 0.6 threshold) | {halated[275, 1080].round(0).tolist()} against plain {plain[275, 1080].round(0).tolist()} |")
(OUT / "halation-table.md").write_text("\n".join(lines) + "\n")

# ---- Dither: the dark ramp, three roundings, amplified 16 times --------------------------------------------------------
lines = ["| rounding | distinct levels | distinct 16-column means | max step between 16-column means | max slab-mean error (levels) |", "|---|---|---|---|---|"]
shown = []
for kind in ["none", "ordered", "blue_noise"]:
    a = load(f"dark-ramp-{kind}")[..., 0]
    x = np.arange(1280)
    ideal = 255 * (0.0 + (16 / 255) * (x + 0.5) / 1280)
    slabs = a.reshape(720, 80, 16).mean(axis=(0, 2))
    ideal_slabs = ideal.reshape(80, 16).mean(axis=1)
    lines.append(f"| {kind} | {len(np.unique(a))} | {len(np.unique(slabs.round(6)))} | {np.abs(np.diff(slabs)).max():.3f} | {np.abs(slabs - ideal_slabs).max():.3f} |")
    shown.append(np.clip(a * 16, 0, 255))
(OUT / "dither-table.md").write_text("\n".join(lines) + "\n")
stack = np.concatenate([s[0:240] for s in shown], axis=0)
save("dither-ramp-x16.png", np.stack([stack] * 3, axis=-1))
zoom = np.concatenate([np.kron(s[300:332, 600:664], np.ones((8, 8))) for s in shown], axis=0)
save("dither-zoom-x16.png", np.stack([zoom] * 3, axis=-1))
print("done")
