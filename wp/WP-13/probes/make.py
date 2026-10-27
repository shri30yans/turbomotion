#!/usr/bin/env python3
"""Writes the motion blur probe films, renders them with `turbomotion comp still`, measures the pixels and writes
`data.tsv` (probe, quantity, expected, measured, tolerance, ok) and `numbers.txt`.  Run from turbomotion/:
    python3 wp/WP-13/probes/make.py [path/to/turbomotion]
The expected numbers come from closed forms written here (boxes, uniform shutters, cubic Beziers), not from the engine."""
import hashlib, json, math, os, subprocess, sys
from PIL import Image
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
BIN = sys.argv[1] if len(sys.argv) > 1 else "target/release/turbomotion"
W, H, FPS, SIZE, FRAME = 800, 120, 30, 20.0, 5
rows, notes = [], []

def put(name, film):
    path = os.path.join(HERE, name + ".comp.json")
    json.dump(film, open(path, "w"), indent=2)
    return path

def still(name, film, frame):
    path = put(name, film)
    out = os.path.join(HERE, "tmp-" + name)
    subprocess.run([BIN, "comp", "still", path, "--out", out, "--times", repr(frame / FPS)], check=True, capture_output=True)
    png = sorted(os.listdir(out))[0]
    img = np.asarray(Image.open(os.path.join(out, png)).convert("RGBA")).astype(float)
    final = os.path.join(HERE, name + ".png")
    os.replace(os.path.join(out, png), final)
    os.rmdir(out)
    return img

def check(path):
    r = subprocess.run([BIN, "comp", "check", path], capture_output=True, text=True)
    return (r.stdout + r.stderr).strip().splitlines()[-1] if (r.stdout + r.stderr).strip() else "clean"

def record(probe, quantity, expected, measured, tol):
    ok = abs(expected - measured) <= tol
    rows.append((probe, quantity, f"{expected:.3f}", f"{measured:.3f}", f"{tol}", "ok" if ok else "FAIL"))
    return ok

def mover(speed, blur, y=30, size=SIZE, extra=None):
    layers = [{"type": "solid", "id": "square", "color": "#ffffff", "size": [size, SIZE], "mblur": True,
               "pos": {"from": [100, y], "to": [100 + speed * 30, y], "at": 0, "dur": 1, "ease": "none"}},
              {"type": "solid", "id": "sharp", "color": "#ffffff", "size": [size, SIZE],
               "pos": {"from": [100, 90], "to": [100 + speed * 30, 90], "at": 0, "dur": 1, "ease": "none"}}]
    film = {"width": W, "height": H, "fps": FPS, "dur": 2, "bg": "#000000", "layers": layers}
    if blur is not None: film["motion_blur"] = blur
    if extra: film.update(extra)
    return film

def profile(img, y):  # red channel along a row, 0..1
    return img[y, :, 0] / 255.0

def stats(v):
    mass = v.sum(); x = np.arange(len(v)) + 0.5
    c = (v * x).sum() / mass
    var = (v * (x - c) ** 2).sum() / mass + 1 / 12  # pixels are 1 px wide
    return mass, c, var

def extent(v, level=0.5 / 255):
    lit = np.nonzero(v >= level)[0]
    return lit[-1] + 1 - lit[0]

# 1 smear lengths (100 px per frame, 20 px square, frame 5: centre 600)
for angle in (180, 360, 0):
    name = f"smear-{angle}"
    img = still(name, mover(100, {"shutter_angle": angle}), FRAME)
    v, s = profile(img, 30), profile(img, 90)
    travel = 100 * angle / 360
    if angle:
        _, c, var = stats(v)
        got = math.sqrt(max(12 * var - SIZE ** 2, 0)) + SIZE
        record(name, "smear length by variance, px", SIZE + travel, got, 1.5)
        record(name, "outer extent, px (one 16-sample step short)", SIZE + travel - travel / 16, extent(v), 1.5)
        record(name, "centre of the smear, px (600 + (phase + angle / 2) / 360 * 100, phase -90)", 600 + (-90 + angle / 2) / 360 * 100, c, 0.3)
        lo, hi = int(c - (SIZE + travel) / 2 + SIZE + 1), int(c + (SIZE + travel) / 2 - SIZE - 1)
        top = v[lo:hi]  # the flat top of the box profile, where the square is always under the pixel
        record(name, "flat top: mean level, levels (255 * size / travel)", 255 * SIZE / travel, 255 * top.mean(), 2.0)
        record(name, "flat top: ripple max - min, levels (one of 16 samples is 255 / 16)", 255 / 16, 255 * (top.max() - top.min()), 1.0)
    else:
        record(name, "extent, px (sharp)", SIZE, extent(v), 0)
        record(name, "pixels identical to the unblurred row", 0, float(np.abs(v - s).max()), 0)
    record(name, "unswitched layer extent, px (sharp)", SIZE, extent(s), 0)

# 2 shutter phase: the window moves by phase/360 of a frame, the centroid by that fraction of 100 px
for phase in (-180, -90, 0, 90):
    name = f"phase{phase:+d}".replace("+", "p").replace("-", "m")
    img = still(name, mover(100, {"shutter_angle": 180, "shutter_phase": phase}), FRAME)
    _, c, _ = stats(profile(img, 30))
    record(name, "centroid, px (600 + (phase + 90) / 360 * 100)", 600 + (phase + 90) / 360 * 100, c, 0.3)

# 3 ease: x = 100 + 700 u^3 (power2.in), a 360 degree shutter centred on frame 27 (u = 0.9, seconds 0.9)
film = mover(0, {"shutter_angle": 360, "shutter_phase": -180})
film["layers"][0]["pos"] = {"from": [100, 30], "to": [800, 30], "at": 0, "dur": 1, "ease": "power2.in"}
film["layers"][1]["pos"] = [100, 90]; film["width"] = 1000
img = still("ease", film, 27)
u = np.linspace(0.9 - 1 / 60, 0.9 + 1 / 60, 20001)
expected = (100 + 700 * u ** 3).mean()
record("ease", "centroid, px (mean of the true positions)", expected, stats(img[30, :, 0] / 255.0)[1] - 0.0, 0.3)
record("ease", "gap to the unblurred position 610.3, px (a straight line gives 0)", expected - (100 + 700 * 0.9 ** 3), stats(img[30, :, 0] / 255.0)[1] - (100 + 700 * 0.9 ** 3), 0.3)

# 4 speed/influence handles: speed 0, influence 60 out and in is cubic-bezier(0.6, 0, 0.4, 1)
def bez(p1, p2, x):  # y of a cubic-bezier easing at x (CSS style), by bisection
    lo, hi = 0.0, 1.0
    for _ in range(60):
        s = (lo + hi) / 2
        bx = 3 * (1 - s) ** 2 * s * p1[0] + 3 * (1 - s) * s ** 2 * p2[0] + s ** 3
        lo, hi = (s, hi) if bx < x else (lo, s)
    s = (lo + hi) / 2
    return 3 * (1 - s) ** 2 * s * p1[1] + 3 * (1 - s) * s ** 2 * p2[1] + s ** 3
film = mover(0, {"shutter_angle": 360, "shutter_phase": -180})
film["layers"][0]["pos"] = {"keys": [{"t": 0, "v": [100, 30], "ease_out": {"speed": 0, "influence": 60}},
                                      {"t": 12 / 30, "v": [740, 30], "ease_in": {"speed": 0, "influence": 60}}]}
film["layers"][1]["pos"] = [100, 90]
img = still("handles", film, 4)
t = np.linspace((4 - 0.5) / 12, (4 + 0.5) / 12, 2001)
expected = np.mean([100 + 640 * bez((0.6, 0), (0.4, 1), x) for x in t])
unblurred = 100 + 640 * bez((0.6, 0), (0.4, 1), 4 / 12)
record("handles", "centroid, px (mean of the eased positions)", expected, stats(img[30, :, 0] / 255.0)[1], 0.3)
record("handles", "gap to the unblurred position, px", expected - unblurred, stats(img[30, :, 0] / 255.0)[1] - unblurred, 0.3)

# 5 rotation: a 200 x 6 bar turning 90 degrees per frame about (400, 150), 180 degree shutter; the tip sweeps an arc of radius 100
film = {"width": 800, "height": 300, "fps": FPS, "dur": 2, "bg": "#000000", "motion_blur": {"shutter_angle": 180},
        "layers": [{"type": "solid", "color": "#ffffff", "size": [200, 6], "mblur": True, "pos": [400, 150],
                    "rotation": {"from": 0, "to": 2700, "at": 0, "dur": 1, "ease": "none"}}]}
img = still("rotate", film, 1)[:, :, 0] / 255.0
ys, xs = np.nonzero(img >= 8 / 255)
rad = np.hypot(xs + 0.5 - 400, ys + 0.5 - 150); ang = np.degrees(np.arctan2(ys + 0.5 - 150, xs + 0.5 - 400)) % 360
tips = []
for lo in range(70, 110, 4):  # tip bins well inside the swept 67.5..112.5 degrees
    sel = (ang >= lo) & (ang < lo + 4)
    tips.append(rad[sel].max())
record("rotate", "outermost lit radius over 10 angle bins, worst deviation from 100 px", 0, max(abs(r - 100) for r in tips), 1.5)
record("rotate", "swept angle of the tip, degrees (45 = 180 degree shutter)", 45, ang[rad > 95].max() - ang[rad > 95][ang[rad > 95] < 180].min() if False else
       (ang[(rad > 95) & (ang < 180)].max() - ang[(rad > 95) & (ang < 180)].min()), 3.0)

# 6 linear light and display numbers: right edge of a 100 px bar goes through pixel 650
for name, color in (("display", {"linear_light": False}), ("linear", {"linear_light": True})):
    film = mover(100, {"shutter_angle": 180}, size=100, extra={"color": color})
    px = still(name, film, FRAME)[30, 650, 0]
    record(name, "pixel 650 of the edge, level (display 127.5, linear light 187.5)", 127.5 if name == "display" else 187.5, px, 1.0)

# 7 alpha: a red bar over blue, pixel 650 holds half of each, premultiplied
film = {"width": W, "height": H, "fps": FPS, "dur": 2, "bg": "#000000", "motion_blur": {"shutter_angle": 180}, "layers": [
    {"type": "solid", "color": "#0000ff", "size": [W, H], "pos": [W / 2, H / 2]},
    {"type": "solid", "color": "#ff0000", "size": [100, SIZE], "mblur": True, "pos": {"from": [100, 30], "to": [3100, 30], "at": 0, "dur": 1, "ease": "none"}}]}
px = still("alpha", film, FRAME)[30, 650]
record("alpha", "red at the edge pixel, level (half red)", 127.5, px[0], 1.0)
record("alpha", "blue at the edge pixel, level (half of what is behind)", 127.5, px[2], 1.0)
record("alpha", "alpha at the edge pixel", 255, px[3], 0)

# 8 a scene that does not move is drawn once and is bit-identical to the unblurred film
still_film = mover(0, {"shutter_angle": 360})
a = still("static", still_film, FRAME)
no_block = dict(still_film); del no_block["motion_blur"]
b = still("static-off", no_block, FRAME)
record("static", "largest pixel difference from the film without the block", 0, float(np.abs(a - b).max()), 0)

# 9 above 16 samples: the new block is refused with the range, the older key is warned about its cost
refused = {}
for name, film in (("samples-32", mover(100, {"shutter_angle": 180, "samples": 32})),
                   ("older-key-32", mover(100, None, extra={"mblur": {"samples": 32, "shutter": 180}}))):
    path = put(name, film)
    r = subprocess.run([BIN, "comp", "check", path], capture_output=True, text=True)
    refused[name] = (r.returncode, (r.stdout + r.stderr).strip())

# checks
checks = {n: check(os.path.join(HERE, n + ".comp.json")) for n in sorted(f[:-10] for f in os.listdir(HERE) if f.endswith(".comp.json")) if n not in refused}

with open(os.path.join(HERE, "data.tsv"), "w") as f:
    f.write("probe\tquantity\texpected\tmeasured\ttolerance\tresult\n")
    for r in rows: f.write("\t".join(r) + "\n")
with open(os.path.join(HERE, "numbers.txt"), "w") as f:
    for r in rows: f.write(" | ".join(r) + "\n")
    f.write("\ncomp check:\n")
    for n, c in checks.items(): f.write(f"  {n}: {c}\n")
    f.write("\nabove 16 samples (exit code, output):\n")
    for n, (code, text) in refused.items(): f.write(f"  {n}: exit {code}\n" + "".join(f"    {l}\n" for l in text.splitlines()))
print(open(os.path.join(HERE, "numbers.txt")).read())
sys.exit(1 if any(r[-1] != "ok" for r in rows) else 0)
