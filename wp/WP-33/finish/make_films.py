#!/usr/bin/env python3
"""Writes the finish-stack probe films: parity/probes/{add_grain,film_halation}.json (the ledger's probes) and the three
dither films under wp/WP-33/finish/films/. Run from turbomotion/; `make_evidence.py` renders and measures them."""
import json, pathlib

ROOT = pathlib.Path(__file__).resolve().parents[3]


def dump(path, film):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(film, indent=1) + "\n")


def ramp_layer(x, w, h, fx):
    return {"type": "shape", "items": [{"rect": [x, 0, w, h], "fill": {"gradient": {"type": "linear", "from": [x, 0], "to": [x, h], "stops": [[0, "#000000"], [1, "#ffffff"]]}}}], "fx": fx}


# Add Grain: the same black-to-white ramp in four panels: clean, then the three presets
presets = [None, "fine_35mm", "coarse_16mm", "vintage_8mm"]
dump(ROOT / "parity/probes/add_grain.json", {
    "width": 1280, "height": 720, "fps": 24, "dur": 1, "bg": "#000000",
    "layers": [ramp_layer(i * 320, 320, 720, [{"effect": "add_grain", "preset": p, "seed": 3}] if p else []) for i, p in enumerate(presets)],
})

# Film halation: lamps and a bar on a dark frame, with the effect as the film's look (over the finished frame)
def disc(x, y, r, color):
    return {"type": "shape", "items": [{"ellipse": [x, y, r, r], "fill": color}]}

def lamps(look):
    film = {
        "width": 1280, "height": 720, "fps": 24, "dur": 1, "bg": "#05060a",
        "layers": [
            disc(280, 360, 70, "#ffffff"),
            {"type": "solid", "color": "#ffffff", "size": [420, 8], "pos": [700, 360]},
            disc(1080, 360, 60, "#666666"),
        ],
    }
    if look:
        film["look"] = look
    return film

dump(ROOT / "parity/probes/film_halation.json", lamps([{"effect": "film_halation", "threshold": 0.6, "radius": 40, "strength": 0.6, "color": [255, 90, 40]}]))
dump(ROOT / "wp/WP-33/finish/films/lamps-plain.json", lamps(None))

# Dither: a dark ramp, level 0 to 16 across the frame, in each of the three roundings
for kind in ["none", "ordered", "blue_noise"]:
    dump(ROOT / f"wp/WP-33/finish/films/dark-ramp-{kind}.json", {
        "width": 1280, "height": 720, "fps": 24, "dur": 1, "bg": "#000000", "color": {"dither": kind},
        "layers": [{"type": "shape", "items": [{"rect": [0, 0, 1280, 720], "fill": {"gradient": {"type": "linear", "from": [0, 0], "to": [1280, 0], "stops": [[0, "#000000"], [1, "#101010"]]}}}]}],
    })
