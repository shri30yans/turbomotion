#!/usr/bin/env python3
"""Writes the probe films of the darks fix: every grey level 0..255 as a small swatch (a solid, a shape), in a linear-light
film and in a default film, and a 256-step gradient across a small layer. Run from anywhere: python3 make.py"""
import json, pathlib

HERE = pathlib.Path(__file__).parent
GRID, CELL, INK = 16, 40, 32
SIDE = GRID * CELL


def hexgrey(level):
    return "#%02x%02x%02x" % (level, level, level)


def centre(level):
    return [(level % GRID) * CELL + CELL // 2, (level // GRID) * CELL + CELL // 2]


def solid(level):
    return {"type": "solid", "color": hexgrey(level), "size": [INK, INK], "pos": centre(level)}


def shape(level):
    x, y = centre(level)
    return {"type": "shape", "items": [{"rect": [x - INK // 2, y - INK // 2, INK, INK], "fill": {"color": hexgrey(level)}}]}


def film(layers, color):
    f = {"width": SIDE, "height": SIDE, "fps": 10, "dur": 1, "bg": "#000000", "layers": layers}
    if color:
        f["color"] = {"linear_light": True}
    return f


def ramp(color):
    # 256 steps across a layer that is 256 px wide in a frame that is 320 px wide: it starts at x = 32
    f = {"width": 320, "height": 16, "fps": 10, "dur": 1, "bg": "#000000", "layers": [
        {"type": "shape", "items": [{"rect": [32, 0, 256, 16], "fill": {"gradient": {"type": "linear", "from": [32, 0], "to": [288, 0], "stops": [[0, "#000000"], [1, "#ffffff"]]}}}]}]}
    if color:
        f["color"] = {"linear_light": True}
    return f


films = {
    "swatches-solid-light": film([solid(l) for l in range(256)], True),
    "swatches-shape-light": film([shape(l) for l in range(256)], True),
    "swatches-solid-default": film([solid(l) for l in range(256)], False),
    "ramp-light": ramp(True),
    "ramp-default": ramp(False),
}
for name, f in films.items():
    (HERE / f"{name}.comp.json").write_text(json.dumps(f, indent=1) + "\n")
