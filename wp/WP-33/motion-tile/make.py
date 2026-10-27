#!/usr/bin/env python3
"""WP-33 Motion Tile probes. Writes the probe film (parity/probes/motion_tile.json), renders it and three more films with the
engine, and holds every panel against a prediction made HERE from the definitions in parity/specs/motion_tile.md (an
independent numpy model of the same formulas, read from the engine's own render of the plain card, not from the effect).
Writes the PNGs and data.tsv.   usage: make.py BIN"""
import json, math, shutil, subprocess, sys, tempfile
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
PROBE = HERE.parents[2] / "parity" / "probes" / "motion_tile.json"
BIN = sys.argv[1]
BG = "#10141f"
BG_RGB = np.array([0x10, 0x14, 0x1f], dtype=float)
QUAD = ["#e02020", "#20c020", "#2040e0", "#e0e020"]
MAX_TAPS = 5
rows = []  # (probe, what, worst level difference)


def card(w, h):
    """A w x h card: four different quadrants and an asymmetric white marker, so a flip or a shift shows."""
    q = lambda i: {"type": "solid", "color": QUAD[i], "size": [w // 2, h // 2], "pos": [w // 4 + w // 2 * (i % 2), h // 4 + h // 2 * (i // 2)]}
    marker = {"type": "solid", "color": "#ffffff", "size": [w * 3 // 20, h // 8], "pos": [w * 3 // 20, h // 7]}
    return {"width": w, "height": h, "dur": 1, "layers": [q(0), q(1), q(2), q(3), marker]}


def still(comp, name=None):
    with tempfile.TemporaryDirectory() as d:
        path = Path(d) / "comp.json"
        path.write_text(json.dumps(comp))
        subprocess.run([BIN, "comp", "still", str(path), "--times", "0.5", "--out", d], capture_output=True, text=True, check=True)
        png = Path(d) / "0000.50.png"
        if name:
            shutil.copy(png, HERE / f"{name}.png")
        return np.asarray(Image.open(png).convert("RGB"), dtype=float)


def check(path):
    out = subprocess.run([BIN, "comp", "check", str(path)], capture_output=True, text=True)
    text = out.stdout + out.stderr
    assert out.returncode == 0 and "0 errors, 0 warnings" in text, text
    return text.strip().splitlines()[-1]


def bilinear(img, x, y):
    h, w, _ = img.shape
    x0, y0 = np.floor(x - 0.5), np.floor(y - 0.5)
    fx, fy = (x - 0.5 - x0)[..., None], (y - 0.5 - y0)[..., None]
    at = lambda i, j: img[np.clip(j, 0, h - 1).astype(int), np.clip(i, 0, w - 1).astype(int)]
    return (at(x0, y0) * (1 - fx) + at(x0 + 1, y0) * fx) * (1 - fy) + (at(x0, y0 + 1) * (1 - fx) + at(x0 + 1, y0 + 1) * fx) * fy


def predict(img, lx, ly, tile=(100, 100), out=(100, 100), center=(0.5, 0.5), mirror=False, phase=0.0, horizontal=False, gutter=0.0, gutter_rgb=(255, 255, 255)):
    """The spec's formula, on arrays of layer pixel indices lx, ly: (colour, drawn) with `drawn` false outside the output."""
    h, w, _ = img.shape
    ux, uy = lx + 0.5, ly + 0.5
    drawn = (abs(ux - w / 2) <= w * out[0] / 200) & (abs(uy - h / 2) <= h * out[1] / 200)
    tw, th = w * tile[0] / 100, h * tile[1] / 100
    px, py = tw + gutter, th + gutter
    dx, dy = ux - center[0] * w, uy - center[1] * h
    s = phase / 360
    if horizontal:
        ny = np.floor(dy / py + 0.5)
        dx = dx + s * ny * px
        nx = np.floor(dx / px + 0.5)
    else:
        nx = np.floor(dx / px + 0.5)
        dy = dy + s * nx * py
        ny = np.floor(dy / py + 0.5)
    lxl, lyl = dx - nx * px, dy - ny * py
    in_gutter = (gutter > 0) & ((abs(lxl) > tw / 2) | (abs(lyl) > th / 2))
    ax, ay = lxl / tw + 0.5, lyl / th + 0.5
    if mirror:
        ax, ay = np.where(nx % 2 == 1, 1 - ax, ax), np.where(ny % 2 == 1, 1 - ay, ay)
    rx, ry = 100 / tile[0], 100 / tile[1]
    tx, ty = min(max(math.ceil(rx), 1), MAX_TAPS), min(max(math.ceil(ry), 1), MAX_TAPS)
    acc = 0
    for j in range(ty):
        for i in range(tx):
            sx = np.clip(ax * w + ((i + 0.5) / tx - 0.5) * rx, 0.5, w - 0.5)
            sy = np.clip(ay * h + ((j + 0.5) / ty - 0.5) * ry, 0.5, h - 0.5)
            acc = acc + bilinear(img, sx, sy)
    colour = np.where(in_gutter[..., None], np.array(gutter_rgb, dtype=float), acc / (tx * ty))
    return colour, drawn


def compare(shot, origin, size, model, region):
    """Worst level difference between the render and the model over `region` = (lx0, lx1, ly0, ly1), layer pixels."""
    lx, ly = np.meshgrid(np.arange(region[0], region[1]), np.arange(region[2], region[3]))
    colour, drawn = model(lx, ly)
    want = np.where(drawn[..., None], colour, BG_RGB)
    got = shot[origin[1] + region[2]:origin[1] + region[3], origin[0] + region[0]:origin[0] + region[1]]
    return float(np.abs(got - want).max()), int(drawn.sum())


# ------------------------------------------------------------------------------------------------------------- the probe
# 160 x 96: a 50 % tile is 80 x 48, so a tile edge never falls on a pixel centre (there floor(x + 0.5) is a tie that float noise decides)
CW, CH = 160, 96
CELL = (426, 320)


def centre(k):
    return CELL[0] // 2 + CELL[0] * (k % 3), 150 + CELL[1] * (k // 3)

PANELS = [
    ("tiles", "250 % output, tile 100 %", {"output_width": 250, "output_height": 250}, dict(out=(250, 250))),
    ("mirror edges", "250 %, mirror on", {"output_width": 250, "output_height": 250, "mirror_edges": "on"}, dict(out=(250, 250), mirror=True)),
    ("phase 180 bricks", "tile 50 %, phase 180, horizontal", {"tile_width": 50, "tile_height": 50, "output_width": 250, "output_height": 250, "phase": 180, "horizontal_phase_shift": "on"},
     dict(tile=(50, 50), out=(250, 250), phase=180.0, horizontal=True)),
    ("gutter 8 px", "tile 50 %, gutter 8 white", {"tile_width": 50, "tile_height": 50, "output_width": 250, "output_height": 250, "gutter_size": 8, "gutter_color": "#ffffff"},
     dict(tile=(50, 50), out=(250, 250), gutter=8.0)),
    ("centre +20, -10", "tile 50 %, centre moved", {"tile_width": 50, "tile_height": 50, "output_width": 250, "output_height": 250, "tile_center": [0.5 + 20 / CW, 0.5 - 10 / CH]},
     dict(tile=(50, 50), out=(250, 250), center=(0.5 + 20 / CW, 0.5 - 10 / CH))),
    ("output 60 % crop", "output 60 %", {"output_width": 60, "output_height": 60}, dict(out=(60, 60))),
]


def probe_film():
    layers = []
    for k, (label, _, fx, _) in enumerate(PANELS):
        cx, cy = centre(k)
        layers.append({"id": f"panel{k}", "type": "precomp", "comp": "card", "pos": [cx, cy], "fx": [dict(effect="motion_tile", **fx)]})
        layers.append({"id": f"label{k}", "type": "text", "text": label, "font": "Helvetica Neue", "weight": 600, "size": 22, "color": "#c9d1e3", "pos": [cx, cy + 135], "align": "center", "ui": True})
    return {"width": 1280, "height": 720, "fps": 30, "dur": 1, "bg": BG, "effect_space": "layer", "comps": {"card": card(CW, CH)}, "layers": layers}


def panels(ref):
    film = probe_film()
    PROBE.write_text(json.dumps(film, indent=1) + "\n")
    summary = check(PROBE)
    shot = still(film, "panels")
    for k, (label, what, _, kw) in enumerate(PANELS):
        cx, cy = centre(k)
        worst, pixels = compare(shot, (cx - CW // 2, cy - CH // 2), (CW, CH), lambda lx, ly: predict(ref, lx, ly, **kw), (-130, 290, -80, 162))
        rows.append((f"panels/{label}", f"{what}; {pixels} px drawn, the rest background", f"{worst:.2f}"))
    return summary


# -------------------------------------------------------------------------------- screen space against layer space
def frame_card_film(space, fx, w=480, h=270):
    return {"width": w, "height": h, "fps": 30, "dur": 1, "bg": BG, "effect_space": space, "comps": {"card": card(w, h)},
            "layers": [{"type": "precomp", "comp": "card", "pos": [w // 2, h // 2], "fx": [fx]}]}


def spaces():
    fx = {"effect": "motion_tile", "tile_width": 40, "tile_height": 30, "output_width": 300, "output_height": 300, "mirror_edges": "on", "phase": 90, "tile_center": [0.46, 0.55]}
    layer, screen = still(frame_card_film("layer", fx), None), still(frame_card_film("screen", fx), None)
    worst = float(np.abs(layer - screen).max())
    Image.fromarray(np.hstack([layer, np.full((layer.shape[0], 6, 3), 255.0), screen]).astype(np.uint8)).save(HERE / "layer-vs-screen.png")
    rows.append(("layer-vs-screen", "frame-sized card, tile 40 x 30 %, output 300 %, mirror, phase 90, centre (0.46, 0.55), left layer space, right screen space", f"{worst:.2f}"))


# ------------------------------------------------------------------------------------------- scale 2 and a quarter turn
def scaled(rotation):
    fx = {"effect": "motion_tile", "tile_width": 50, "tile_height": 50, "output_width": 150, "output_height": 150, "mirror_edges": "on"}
    return {"width": 640, "height": 640, "fps": 30, "dur": 1, "bg": BG, "effect_space": "layer", "comps": {"card": card(CW, CH)},
            "layers": [{"type": "precomp", "comp": "card", "pos": [320, 320], "scale": 2, "rotation": rotation, "fx": [fx]}]}


def turned():
    flat, quarter, slant = still(scaled(0)), still(scaled(90), "scale2-quarter-turn"), still(scaled(30), "scale2-rotation30")
    # a quarter turn clockwise about the frame's middle (640 x 640, even): the flat picture rotated by np.rot90 with k = -1
    worst = float(np.abs(np.rot90(flat, k=-1) - quarter).max())
    rows.append(("scale2-quarter-turn", "layer scale 2 and rotation 90 against the unrotated scale 2 picture turned by 90 degrees", f"{worst:.2f}"))
    return slant


# ------------------------------------------------------------------------------------------------- sub-pixel seams
def seam():
    film = {"width": 96, "height": 48, "fps": 30, "dur": 1, "bg": BG, "effect_space": "layer", "comps": {"card": card(32, 16)},
            "layers": [{"type": "precomp", "comp": "card", "pos": [48, 24], "fx": [{"effect": "motion_tile", "output_width": 300, "output_height": 300, "tile_center": [0.5 + 0.3 / 32, 0.5]}]}]}
    ref = still({"width": 32, "height": 16, "fps": 30, "dur": 1, "bg": BG, "comps": {"card": card(32, 16)}, "layers": [{"type": "precomp", "comp": "card", "pos": [16, 8]}]})
    shot = still(film)
    zoom = np.kron(shot, np.ones((8, 8, 1))).astype(np.uint8)
    Image.fromarray(zoom).save(HERE / "seam-subpixel.png")
    # the first pixel of every tile (layer x = 0, 32, -32) is the card's first column and the one before it the last, exactly
    worst = 0.0
    for ly in range(16):
        for lx, col in [(-32, 0), (0, 0), (32, 0), (-1, 31), (31, 31)]:
            worst = max(worst, float(np.abs(shot[16 + ly, 32 + lx] - ref[ly, col]).max()))
    rows.append(("seam-subpixel", "tile centre 0.3 px right: the 5 seam columns x 16 rows against the card's first and last texel column", f"{worst:.2f}"))


summary = panels(still({**card(CW, CH), "fps": 30, "bg": BG}))
spaces()
turned()
seam()
(HERE / "data.tsv").write_text("probe\twhat\tworst_level_difference_from_prediction\n" + "\n".join("\t".join(r) for r in rows) + "\n")
print(summary)
for r in rows:
    print("\t".join(r))
