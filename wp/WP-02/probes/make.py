#!/usr/bin/env python3
"""Writes the WP-02 probe films to parity/probes/, renders them and measures them.

    python3 wp/WP-02/probes/make.py [path/to/turbomotion]

Probes (parity/probes/): gaussian_blur, box_blur, lens_blur, unsharp_mask, radial_blur. Each is a film a person can open; the numbers below
come from stills of the same films with other sigmas, so the sweeps are measured on rendered 8-bit frames, not on the unit tests' floats.
Output: `data.tsv` (every measurement, `probe quantity setting legacy exact expected`), `numbers.txt` (the figures README.md quotes) and
one PNG per probe (`comp still`, only there to confirm what the numbers say)."""
import json, math, subprocess, sys, tempfile
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).parent
ENGINE = HERE.parents[2]
BIN = sys.argv[1] if len(sys.argv) > 1 else str(ENGINE / "target/release/turbomotion")
W, H, EDGE, WINDOW = 1280, 720, 640, 80
TMP = Path(tempfile.mkdtemp(prefix="wp02-"))
rows: list[tuple] = []


def film(layers, linear=False):
    f = {"width": W, "height": H, "fps": 24, "dur": 1, "bg": "#000000", "layers": layers}
    return {**f, "color": {"linear_light": True}} if linear else f


def solid(color, size, pos, fx=None, **kw):
    return {"type": "solid", "color": color, "size": size, "pos": pos, **({"fx": fx} if fx else {}), **kw}


def render(f, name):
    path = TMP / f"{name}.comp.json"
    path.write_text(json.dumps(f))
    out = TMP / name
    subprocess.run([BIN, "comp", "still", str(path), "--out", str(out), "--times", "0.5"], check=True, capture_output=True)
    png = next(out.glob("*.png"))
    return png, np.asarray(Image.open(png).convert("RGB"), dtype=np.float64)


def phi(x):
    return 0.5 * (1 + math.erf(x / math.sqrt(2)))


def encode(light):
    return 255 * (12.92 * light if light <= 0.0031308 else 1.055 * light ** (1 / 2.4) - 0.055)


def decode(level):
    v = np.asarray(level) / 255
    return np.where(v <= 0.04045, v / 12.92, ((v + 0.055) / 1.055) ** 2.4)


def add(probe, quantity, setting, legacy, exact, expected):
    rows.append((probe, quantity, setting, legacy, exact, expected))


def edge_error(px, y, want):
    """The most levels any pixel within WINDOW of the edge is from `want(d)`, d being the pixel middle's distance from the edge."""
    xs = range(EDGE - WINDOW, EDGE + WINDOW)
    return max(abs(px[y, x, 0] - want(x + 0.5 - EDGE)) for x in xs)


def edge_film(fx, linear):
    return film([solid("#ffffff", [W // 2, H], [EDGE, 0], [fx], anchor=[0, 0])], linear)


def energy(px, centre, reach, count):
    cx, cy = centre
    window = px[cy - reach:cy + reach, cx - reach:cx + reach, 0]
    return float(decode(window).sum() / count)


def gaussian():
    for linear in (False, True):
        for sigma in (4, 8, 16, 24):
            # a display film mixes its own numbers, a linear-light film mixes light and shows its sRGB encoding
            want = (lambda d, s=sigma: encode(phi(d / s))) if linear else (lambda d, s=sigma: 255 * phi(d / s))
            errs = [edge_error(render(edge_film({"effect": "blur", "radius": sigma, "quality": q}, linear), f"edge-{q}-{sigma}-{int(linear)}")[1], H // 2, want) for q in ("legacy", "exact")]
            add("gaussian_blur", "hard edge, worst level off the erf profile", f"sigma {sigma}, {'linear light' if linear else 'display'}", *[round(e, 2) for e in errs], "0 (1 allowed)")
    for sigma in (4, 5, 6, 8, 12, 16, 32):
        # a 4 x 4 square, and one that grows with the blur: the tail of a 4 x 4 square at sigma 32 is under one 8-bit level
        for side in sorted({4, max(4, sigma // 2)}):
            vals = []
            for q in ("legacy", "exact"):
                px = render(film([solid("#ffffff", [side, side], [W // 2, H // 2], [{"effect": "blur", "radius": sigma, "quality": q}])], True), f"lone-{q}-{sigma}-{side}")[1]
                vals.append(round(energy(px, (W // 2, H // 2), min(int(5 * sigma), 300), side * side), 4))
            add("gaussian_blur", "light of a white square after the blur / before", f"sigma {sigma}, {side} x {side} square", *vals, "1.0000")


def box():
    r = 6
    for iterations in (1, 3):
        sigma = math.sqrt(iterations * ((2 * r + 1) ** 2 - 1) / 12)
        ramp = (lambda d: encode(min(1, max(0, (d + r + 0.5) / (2 * r + 1))))) if iterations == 1 else (lambda d: encode(phi(d / sigma)))
        px = render(edge_film({"effect": "box_blur", "radius": r, "iterations": iterations}, True), f"box-{iterations}")[1]
        add("box_blur", "hard edge, worst level off the " + ("linear ramp" if iterations == 1 else "Gaussian of equal variance"), f"radius {r}, iterations {iterations}", "-", round(edge_error(px, H // 2, ramp), 2), "0 (1 allowed) for 1; for 3 the Gaussian is only near (10 levels)")
    px = render(film([solid("#ffffff", [8, 8], [W // 2, H // 2], [{"effect": "box_blur", "radius": 6, "iterations": 1}])], True), "box-energy")[1]
    add("box_blur", "light of an 8 x 8 white square after / before", "radius 6", "-", round(energy(px, (W // 2, H // 2), 40, 64), 4), "1.0000")
    lit = np.argwhere(px[:, :, 0] > 0)
    add("box_blur", "lit width of that square, px", "radius 6", "-", int(lit[:, 1].max() - lit[:, 1].min() + 1), "8 + 2 * 6 = 20")


def lens():
    for radius in (4, 6, 8, 12):
        px = render(film([solid("#ffffff", [2, 2], [W // 2, H // 2], [{"effect": "lens_blur", "radius": radius}])], True), f"lens-{radius}")[1]
        cx, cy = W // 2, H // 2
        flat = [px[cy, x, 0] for x in range(cx - int(0.7 * radius), cx + int(0.7 * radius))]
        far = px[cy - radius - 6:cy + radius + 6, cx - radius - 6:cx + radius + 6, 0]
        yy, xx = np.mgrid[-radius - 6:radius + 6, -radius - 6:radius + 6]
        outside = far[np.hypot(xx + 0.5, yy + 0.5) > radius + 2.0]
        add("lens_blur", "disc: spread of levels over the middle 70% of the radius, light past radius + 2 px", f"radius {radius}", "-", f"{max(flat) - min(flat):.0f} levels, max {outside.max():.0f}", "flat top (at most 2 levels), 0")
        plateau = encode(4 / (math.pi * radius * radius))
        add("lens_blur", "disc: level of the plateau, 2 x 2 square, area over pi r squared", f"radius {radius}", "-", f"{flat[len(flat) // 2]:.0f}", f"{plateau:.1f} (1 allowed)")


def unsharp():
    for sigma, amount in ((2, 100), (4, 100), (4, 200)):
        layers = [solid("#646464", [W, H], [0, 0], anchor=[0, 0]), solid("#a0a0a0", [W // 2, H], [EDGE, 0], anchor=[0, 0]),
                  {"type": "adjustment", "fx": [{"effect": "unsharp_mask", "amount": amount, "radius": sigma, "threshold": 0}]}]
        px = render(film(layers), f"unsharp-{sigma}-{amount}")[1]
        want = lambda d: min(255, max(0, (160 if d > 0 else 100) + amount / 100 * 60 * ((d > 0) - phi(d / sigma))))
        add("unsharp_mask", "edge 100 to 160, worst level off v + a (v - blur)", f"sigma {sigma}, amount {amount}%", "-", round(edge_error(px, H // 2, want), 2), "0 (1 allowed)")
    px = render(film([solid("#646464", [W, H], [0, 0], anchor=[0, 0]), {"type": "adjustment", "fx": [{"effect": "unsharp_mask", "amount": 300, "radius": 3, "threshold": 0}]}]), "unsharp-flat")[1]
    add("unsharp_mask", "flat grey 100: levels that change", "amount 300", "-", int((px[:, :, 0] != 100).sum()), "0")


def radial():
    cx, cy, off = W // 2, H // 2, 200
    for degrees in (20, 60):
        px = render(film([solid("#ffffff", [4, 4], [cx + off, cy], [{"effect": "radial_blur", "mode": "spin", "amount": degrees, "quality": "exact"}])], True), f"spin-{degrees}")[1]
        ys, xs = np.nonzero(px[:, :, 0] > 3)
        r = np.hypot(xs + 0.5 - cx, ys + 0.5 - cy)
        a = np.degrees(np.arctan2(ys + 0.5 - cy, xs + 0.5 - cx))
        add("radial_blur", "spin of a 4 x 4 square at 200 px: radius spread (px), angle extent (deg), light after / before", f"{degrees} degrees", "-", f"{r.max() - r.min():.1f} px, {a.max() - a.min():.1f} deg, {energy(px, (cx + off, cy), 120, 16):.3f}", f"about 4 px, {degrees} deg less the ends, 1.000")


def parity():
    """The films kept in parity/probes: what a person opens to see each effect."""
    out = ENGINE / "parity/probes"
    edge = lambda y, fx: solid("#ffffff", [W // 2, 180], [EDGE, y], [fx], anchor=[0, 0])
    sources = lambda fx, x, y: solid("#ffffff", [4, 4], [x, y], [fx])
    films = {
        "gaussian_blur": film([edge(0, {"effect": "blur", "radius": 16, "quality": "legacy"}), edge(180, {"effect": "blur", "radius": 16, "quality": "exact"}),
                               edge(360, {"effect": "blur", "radius": 16, "quality": "exact", "dimensions": "horizontal"}),
                               sources({"effect": "blur", "radius": 8, "quality": "legacy"}, 320, 600), sources({"effect": "blur", "radius": 8, "quality": "exact"}, 960, 600)], True),
        "box_blur": film([edge(0, {"effect": "box_blur", "radius": 6, "iterations": 1}), edge(180, {"effect": "box_blur", "radius": 6, "iterations": 3}),
                          solid("#ffffff", [8, 8], [320, 600], [{"effect": "box_blur", "radius": 6}])], True),
        "lens_blur": film([solid("#ffffff", [6, 6], [x, 360], [{"effect": "lens_blur", "radius": 28, "blades": b, "rotation": 0}]) for x, b in ((220, 0), (640, 6), (1060, 5))], True),
        "unsharp_mask": film([solid("#646464", [W, H], [0, 0], anchor=[0, 0]), solid("#a0a0a0", [W // 2, H], [EDGE, 0], anchor=[0, 0]),
                              {"type": "adjustment", "fx": [{"effect": "unsharp_mask", "amount": 100, "radius": 4, "threshold": 0}]}]),
        "radial_blur": film([solid("#ffffff", [6, 6], [640 + off, 360], [{"effect": "radial_blur", "mode": "spin", "amount": 30, "quality": "exact"}]) for off in (100, 200, 300)]
                            + [solid("#ffffff", [6, 6], [640, 360 - off], [{"effect": "radial_blur", "mode": "zoom", "amount": 0.3, "quality": "exact"}]) for off in (100, 200)], True),
    }
    for name, f in films.items():
        (out / f"{name}.json").write_text(json.dumps(f, indent=1) + "\n")
        png, _ = render(f, name)
        (HERE / f"{name}.png").write_bytes(png.read_bytes())
        check = subprocess.run([BIN, "comp", "check", str(out / f"{name}.json")], capture_output=True, text=True)
        print(name, "comp check exit", check.returncode, check.stdout.strip().splitlines()[-1:] or check.stderr.strip()[-200:])


if __name__ == "__main__":
    for step in (gaussian, box, lens, unsharp, radial, parity):
        step()
    with open(HERE / "data.tsv", "w") as f:
        f.write("probe\tquantity\tsetting\tlegacy\texact\texpected\n")
        for r in rows:
            f.write("\t".join(str(c) for c in r) + "\n")
    print((HERE / "data.tsv").read_text())
