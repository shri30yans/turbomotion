#!/usr/bin/env python3
"""Writes the WP-05 probe films next to this file: `modes.comp.json` (every mode, each in its own cell over one standard
backdrop, default numbers), `display.comp.json` and `light.comp.json` (multiply, screen and overlay on display numbers and in linear light) and
`old-modes.comp.json` / `old-modes-light.comp.json` (the 17 original modes at partial alpha, for the identity run).
The MODES list is read from the engine, so a mode added to the table appears in the next run."""
import json, subprocess, sys
from pathlib import Path

HERE = Path(__file__).parent
ENGINE = HERE.parents[2]
BIN = sys.argv[1] if len(sys.argv) > 1 else str(ENGINE / "target/release/turbomotion")
CELL_W, CELL_H, LABEL_H = 270, 150, 24

modes = [m for m in json.loads(subprocess.check_output([BIN, "comp", "catalogue"], cwd=ENGINE))["blend_modes"]["modes"]]
names = [m["name"] for m in modes]


def backdrop():
    return [
        {"type": "shape", "items": [{"rect": [0, 0, CELL_W, CELL_H], "fill": {"gradient": {"type": "linear", "from": [0, 0], "to": [CELL_W, 0], "stops": [[0, "#1d2b64"], [1, "#f8cdda"]]}}}]},
        {"type": "shape", "items": [{"rect": [0, 100, CELL_W, 50], "fill": "#808080"}]},
    ]


def source(mode, opacity):
    return {"type": "shape", "pos": [CELL_W / 2, CELL_H / 2], "blend": mode, "opacity": opacity, "items": [
        {"ellipse": [-40, 0, 70, 70], "fill": "#e8a33d"}, {"rect": [-5, -45, 90, 90], "fill": "#3ddc97"}]}


def plain(mode):
    """One flat backdrop (64, 128, 192) with a disc of (224, 48, 128) in `mode`: the numbers of `blend_tests.rs`."""
    disc = {"type": "shape", "pos": [CELL_W / 2, CELL_H / 2], "blend": mode, "items": [{"ellipse": [0, 0, 60, 60], "fill": "#e03080"}]}
    return {"width": CELL_W, "height": CELL_H, "dur": 1, "layers": [{"type": "solid", "color": "#4080c0"}, disc]}


def cell(mode, opacity):
    return {"width": CELL_W, "height": CELL_H, "dur": 1, "layers": backdrop() + [source(mode, opacity)]}


def sheet(cells, cols, rows, color=None):
    """cells: [(label, comp name, comp)]"""
    pw, ph = CELL_W + 4, CELL_H + LABEL_H
    # a margin of 6% of the grid on every side keeps every label inside the 5% safe area `comp check` asks for
    mx, my = round(0.06 * cols * pw), round(0.06 * rows * ph)
    film = {"width": cols * pw + 2 * mx, "height": rows * ph + 2 * my, "fps": 30, "dur": 1, "bg": "#0b0d12", "comps": {}, "layers": []}
    if color:
        film["color"] = color
    for i, (label, key, comp) in enumerate(cells):
        x, y = mx + (i % cols) * pw + 2, my + (i // cols) * ph
        film["comps"][key] = comp
        film["layers"].append({"type": "precomp", "comp": key, "anchor": [0, 0], "pos": [x, y]})
        film["layers"].append({"type": "text", "text": label, "font": "Helvetica Neue", "ui": True, "size": 15, "color": "#ffffff", "anchor": [0, 0], "pos": [x + 4, y + CELL_H + 3]})
    return film


def write(name, film):
    (HERE / name).write_text(json.dumps(film, indent=1) + "\n")


write("modes.comp.json", sheet([(m, m, cell(m, 0.85)) for m in names], 7, 6))
trio = ["multiply", "screen", "overlay"]
# a film has one colour setting, so the two rows are two films
write("display.comp.json", sheet([(f"{m} (display numbers)", m, plain(m)) for m in trio], 3, 1))
write("light.comp.json", sheet([(f"{m} (linear light)", m, plain(m)) for m in trio], 3, 1, {"linear_light": True}))
for name in ("old-modes", "old-modes-light"):
    old = names[:17]
    film = sheet([(m, m, cell(m, 0.6)) for m in old], 6, 3, {"linear_light": True} if name.endswith("light") else None)
    write(f"{name}.comp.json", film)
