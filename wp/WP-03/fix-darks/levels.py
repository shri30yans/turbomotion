#!/usr/bin/env python3
"""Reads the stills of the probe films and prints what each source level came out as.
usage: levels.py swatches STILL.png      one row per level: level<TAB>output
       levels.py ramp STILL.png          one row per pixel of the gradient: x<TAB>output (red channel)
       levels.py summary swatches STILL  the numbers the report quotes"""
import sys
from PIL import Image

GRID, CELL = 16, 40


def swatch_levels(path):
    im = Image.open(path).convert("RGB")
    out = []
    for level in range(256):
        x, y = (level % GRID) * CELL + CELL // 2, (level // GRID) * CELL + CELL // 2
        r, g, b = im.getpixel((x, y))
        assert r == g == b, (level, r, g, b)
        out.append(r)
    return out


def srgb_to_light(v):
    v /= 255
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4


def light_to_level(l):
    return 255 * (l * 12.92 if l <= 0.0031308 else 1.055 * l ** (1 / 2.4) - 0.055)


if __name__ == "__main__":
    kind, path = sys.argv[1], sys.argv[-1]
    if kind == "swatches":
        for level, out in enumerate(swatch_levels(path)):
            print(f"{level}\t{out}")
    elif kind == "ramp":
        im = Image.open(path).convert("RGB")
        for x in range(256):
            print(f"{x}\t{im.getpixel((32 + x, 8))[0]}")
    elif kind == "summary":
        out = swatch_levels(path)
        off = [l for l in range(256) if abs(out[l] - l) > 1]
        flat = [l for l in range(1, 256) if out[l] <= out[l - 1]]
        print(f"levels more than 1 off: {len(off)}; worst error {max(abs(out[l] - l) for l in range(256))}; "
              f"distinct outputs {len(set(out))} of 256; levels that do not rise above the one below: {len(flat)}")
        print("levels 1..32: " + " ".join(f"{l}->{out[l]}" for l in range(1, 33)))
