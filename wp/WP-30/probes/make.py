#!/usr/bin/env python3
"""WP-30 palette-cycle and stroke-taper probes: writes the probe films, renders them with the engine, reads the pixels
and holds them against numbers worked out here (not by the engine). Writes data.tsv and the PNGs.   usage: make.py BIN"""
import json, math, shutil, subprocess, sys, tempfile
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
BIN = sys.argv[1]
BG = "#10141f"
PAL = ["#e63946", "#f1fa8c", "#2a9d8f", "#264653", "#9b5de5"]
rows = []  # (probe, what, expected, measured, abs_diff)


def levels(hexc):
    return np.array([int(hexc[i:i + 2], 16) for i in (1, 3, 5)], dtype=float)


def to_light(v):
    v = v / 255.0
    return np.where(v <= 0.04045, v / 12.92, ((v + 0.055) / 1.055) ** 2.4)


def to_level(l):
    return 255.0 * np.where(l <= 0.0031308, l * 12.92, 1.055 * np.power(l, 1 / 2.4) - 0.055)


def save(name, comp):
    (HERE / f"{name}.comp.json").write_text(json.dumps(comp, indent=1) + "\n")
    return HERE / f"{name}.comp.json"


def still(path, t, keep=None):
    with tempfile.TemporaryDirectory() as d:
        subprocess.run([BIN, "comp", "still", str(path), "--times", str(t), "--out", d], capture_output=True, text=True, check=True)
        png = Path(d) / f"{t:07.2f}.png"
        if keep:
            shutil.copy(png, HERE / f"{keep}.png")
        return np.asarray(Image.open(png).convert("RGB"), dtype=float)


def check(path):
    out = subprocess.run([BIN, "comp", "check", str(path)], capture_output=True, text=True)
    assert out.returncode == 0 and "0 error" in (out.stdout + out.stderr) or out.returncode == 0, out.stdout + out.stderr


def film(layers, w=960, h=540, light=False, dur=2):
    f = {"width": w, "height": h, "fps": 30, "dur": dur, "bg": BG, "layers": layers}
    if light:
        f["color"] = {"linear_light": True}
    return f


# ---------------------------------------------------------------------------------------------------------------- palette
N = 8
STEP = 110


def row_item(y, cycle, copies=N, palette=PAL, **extra):
    return {"rect": [-30, -30, 60, 60], "offset": [60, y], "fill": "#ffffff", "repeat": {"copies": copies, "offset": [STEP, 0], "palette": palette, "cycle": cycle, **extra}}


def centre(i, y):
    return (60 + STEP * i, y)


def step_index(i, off, n, rev):
    k = int(math.floor(i + off)) % n
    return n - 1 - k if rev else k


def palette_step():
    cases = [(0, False), (1, False), (-1, False), (2.5, False), (-0.5, True), (7, True)]
    items = [row_item(50 + 80 * r, {"offset": off, "reverse": rev}) for r, (off, rev) in enumerate(cases)]
    path = save("palette-step", film([{"id": "rows", "type": "shape", "items": items}], h=540))
    check(path)
    img = still(path, 0.5, "palette-step")
    for r, (off, rev) in enumerate(cases):
        worst = 0.0
        for i in range(N):
            x, y = centre(i, 50 + 80 * r)
            got = img[y, x]
            want = levels(PAL[step_index(i, off, 5, rev)])
            worst = max(worst, float(np.abs(got - want).max()))
        rows.append(("palette-step", f"{N} copies, 5 colours, offset {off}, reverse {rev}: worst channel error over all copies, levels", "0", f"{worst:.0f}", f"{worst:.0f}"))


def gradient_expect(i, off, n, light):
    u = (i + off) % n
    k = int(math.floor(u))
    a, b, by = levels(PAL[k % n]), levels(PAL[(k + 1) % n]), u - k
    return to_level(to_light(a) + (to_light(b) - to_light(a)) * by) if light else a + (b - a) * by


def palette_gradient():
    for light in (False, True):
        name = "palette-gradient-light" if light else "palette-gradient"
        offs = [0, 0.25, -0.5]
        items = [row_item(60 + 100 * r, {"mode": "gradient", "offset": o}, palette=PAL[:3]) for r, o in enumerate(offs)]
        path = save(name, film([{"id": "rows", "type": "shape", "items": items}], h=360, light=light))
        check(path)
        img = still(path, 0.5, name)
        for r, o in enumerate(offs):
            worst = max(float(np.abs(img[centre(i, 60 + 100 * r)[1], centre(i, 0)[0]] - gradient_expect(i, o, 3, light)).max()) for i in range(N))
            rows.append((name, f"gradient, 3 colours, offset {o}, {'linear light (mix of light)' if light else 'display (mix of levels)'}: worst channel error, levels", "0", f"{worst:.2f}", f"{worst:.2f}"))


def palette_pingpong_and_keyed():
    items = [row_item(50, {"mode": "ping_pong"}, palette=PAL[:4])]
    layers = [{"id": "rows", "type": "shape", "items": items},
              {"id": "chip", "type": "solid", "color": "#ffffff", "size": [60, 60], "pos": [60, 200], "repeat": {"count": N, "offset": [STEP, 0], "palette": PAL[:3], "cycle": {"offset": [[0, 0], [1, 4, "none"]]}}}]
    path = save("palette-pingpong-keyed", film(layers, h=270))
    check(path)
    img = still(path, 0.5, "palette-pingpong-keyed")
    seq = [0, 1, 2, 3, 2, 1, 0, 1]
    got = [int(np.argmin([np.abs(img[50, 60 + STEP * i] - levels(c)).max() for c in PAL[:4]])) for i in range(N)]
    rows.append(("palette-pingpong", "ping_pong over 4 colours: palette index of each of 8 copies (nearest palette colour)", " ".join(map(str, seq)), " ".join(map(str, got)), "0" if got == seq else "mismatch"))
    for t in (0.0, 0.5, 1.0):
        img = still(path, t)
        want = [(int(math.floor(i + 4 * t)) % 3) for i in range(N)]
        got = [int(np.argmin([np.abs(img[200, 60 + STEP * i] - levels(c)).max() for c in PAL[:3]])) for i in range(N)]
        rows.append(("palette-keyed", f"offset keyed 0 to 4 over 1 s, layer repeater, t = {t}: palette index of each copy", " ".join(map(str, want)), " ".join(map(str, got)), "0" if got == want else "mismatch"))


def palette_opacity():
    layers = [{"id": "rows", "type": "shape", "items": [row_item(50, {"offset": 1}, palette=PAL[:4], opacity=0.8)]}]
    for light in (False, True):
        name = "palette-opacity-light" if light else "palette-opacity"
        path = save(name, film(layers, h=100, light=light))
        check(path)
        img = still(path, 0.5, name)
        bg = levels(BG)
        worst = 0.0
        for i in range(N):
            c, a = levels(PAL[step_index(i, 1, 4, False)]), 0.8 ** i
            want = to_level(to_light(c) * a + to_light(bg) * (1 - a)) if light else c * a + bg * (1 - a)
            worst = max(worst, float(np.abs(img[50, 60 + STEP * i] - want).max()))
        rows.append((name, f"offset 1, per-copy opacity 0.8 over {BG}, {'linear light' if light else 'display'}: worst channel error, levels", "0", f"{worst:.2f}", f"{worst:.2f}"))


# ------------------------------------------------------------------------------------------------------------------ taper
def cover(img):
    return np.clip((img[:, :, 0] - 16.0) / (255.0 - 16.0), 0, 1)  # white on #10141f: the red channel runs 16 to 255


def bilinear(c, x, y):
    fx, fy = x - 0.5, y - 0.5
    x0, y0 = int(math.floor(fx)), int(math.floor(fy))
    u, v = fx - x0, fy - y0
    g = lambda a, b: c[b, a] if 0 <= a < c.shape[1] and 0 <= b < c.shape[0] else 0.0
    return g(x0, y0) * (1 - u) * (1 - v) + g(x0 + 1, y0) * u * (1 - v) + g(x0, y0 + 1) * (1 - u) * v + g(x0 + 1, y0 + 1) * u * v


def across(c, p, n, reach):
    ks = np.arange(-int(reach / 0.05), int(reach / 0.05) + 1)
    return sum(bilinear(c, p[0] + n[0] * k * 0.05, p[1] + n[1] * k * 0.05) for k in ks) * 0.05


GSAP = {"none": lambda u: u, "power2.out": lambda u: 1 - (1 - u) ** 3, "sine.inOut": lambda u: -(math.cos(math.pi * u) - 1) / 2,
        "power2.inOut": lambda u: 4 * u ** 3 if u < 0.5 else 1 - (-2 * u + 2) ** 3 / 2}


def profile(s, length, full, start, end, ease):
    """The width at distance s: tips are (length, width) pairs in px; lengths held to half the path; eased from each tip."""
    w = full
    for tip, d in ((start, s), (end, length - s)):
        if tip:
            tl, tw = min(max(tip[0], 0), length / 2), tip[1]
            if d < tl:
                w = tw + (full - tw) * GSAP[ease](d / tl)
                break
    return w


def bezier(p0, p1, p2, p3, n=40000):
    t = np.linspace(0, 1, n)[:, None]
    pts = (1 - t) ** 3 * p0 + 3 * (1 - t) ** 2 * t * p1 + 3 * (1 - t) * t ** 2 * p2 + t ** 3 * p3
    seg = np.linalg.norm(np.diff(pts, axis=0), axis=1)
    return pts, np.concatenate([[0], np.cumsum(seg)])


def at_distance(pts, cum, s):
    i = int(np.searchsorted(cum, s))
    i = min(max(i, 1), len(pts) - 1)
    t = pts[i] - pts[i - 1]
    t = t / np.linalg.norm(t)
    return pts[i], np.array([-t[1], t[0]])


def taper_probes():
    eases = ["none", "power2.out", "sine.inOut"]
    items = [{"d": f"M 60 {80 + 90 * k} L 900 {80 + 90 * k}", "stroke": {"color": "#ffffff", "width": 30}, "taper": {"start": {"length": 200, "width": 0}, "end": {"length": 160, "width": 8}, "ease": e}} for k, e in enumerate(eases)]
    path = save("taper-straight", film([{"id": "lines", "type": "shape", "items": items}], h=360))
    check(path)
    c = cover(still(path, 0.5, "taper-straight"))
    for k, e in enumerate(eases):
        worst, detail = 0.0, []
        for s in (3, 50, 100, 199, 420, 700, 760, 837):
            m = across(c, (60 + s, 80 + 90 * k), (0, 1), 30)
            want = profile(s, 840, 30, (200, 0), (160, 8), e)
            worst = max(worst, abs(m - want))
            detail.append(f"{s}:{want:.2f}/{m:.2f}")
        rows.append((f"taper-straight", f"ease {e}, 840 px, full 30, start 200 px to 0, end 160 px to 8, widths at 8 distances (distance:expected/measured), worst error px", " ".join(detail), f"{worst:.3f}", f"{worst:.3f}"))
    # a curve with lengths as fractions of the path
    p0, p1, p2, p3 = (np.array(p, float) for p in ((80, 500), (80, 80), (880, 80), (880, 500)))
    pts, cum = bezier(p0, p1, p2, p3)
    length = cum[-1]
    spec = {"d": "M 80 500 C 80 80 880 80 880 500", "stroke": {"color": "#ffffff", "width": 28}, "taper": {"start": {"length": {"fraction": 0.3}, "width": 0}, "end": {"length": {"fraction": 0.2}, "width": {"fraction": 0.25}}, "ease": "power2.inOut"}}
    path = save("taper-curve", film([{"id": "curve", "type": "shape", "items": [spec]}]))
    check(path)
    c = cover(still(path, 0.5, "taper-curve"))
    worst, detail = 0.0, []
    for u in (0.01, 0.1, 0.2, 0.3, 0.5, 0.7, 0.8, 0.9, 0.99):
        s = u * length
        p, n = at_distance(pts, cum, s)
        m = across(c, p, n, 28)
        want = profile(s, length, 28, (0.3 * length, 0), (0.2 * length, 7), "power2.inOut")
        worst = max(worst, abs(m - want))
        detail.append(f"{u}:{want:.2f}/{m:.2f}")
    rows.append(("taper-curve", f"cubic, {length:.1f} px long, fractions 0.3 and 0.2, ease power2.inOut, widths at 9 fractions (fraction:expected/measured), worst error px", " ".join(detail), f"{worst:.3f}", f"{worst:.3f}"))
    # a closed path: an ellipse is a circle of radius 200; its start is at 3 o'clock and it runs clockwise on screen
    R, cx, cy = 200.0, 480.0, 270.0
    spec = {"ellipse": [cx, cy, R, R], "stroke": {"color": "#ffffff", "width": 24}, "taper": {"start": {"length": 250, "width": 0}, "end": {"length": 150, "width": 0}}}
    path = save("taper-closed", film([{"id": "ring", "type": "shape", "items": [spec]}]))
    check(path)
    c = cover(still(path, 0.5, "taper-closed"))
    length = 2 * math.pi * R
    worst, detail = 0.0, []
    for s in (5, 100, 250, 600, length - 150, length - 100, length - 5):
        a = s / R
        p, n = np.array([cx + R * math.cos(a), cy + R * math.sin(a)]), np.array([math.cos(a), math.sin(a)])  # the radius is the normal
        m = across(c, p, n, 24)
        want = profile(s, length, 24, (250, 0), (150, 0), "none")
        worst = max(worst, abs(m - want))
        detail.append(f"{s:.0f}:{want:.2f}/{m:.2f}")
    rows.append(("taper-closed", f"circle R 200 ({length:.1f} px), start 250 px and end 150 px both to 0, full 24, widths (distance:expected/measured), worst error px", " ".join(detail), f"{worst:.3f}", f"{worst:.3f}"))
    # lengths held to half the path
    spec = {"d": "M 60 100 L 460 100", "stroke": {"color": "#ffffff", "width": 20}, "taper": {"start": {"length": 5000, "width": 0}, "end": {"length": 5000, "width": 0}}}
    path = save("taper-clamp", film([{"id": "line", "type": "shape", "items": [spec]}], w=520, h=200))
    check(path)
    c = cover(still(path, 0.5, "taper-clamp"))
    worst, detail = 0.0, []
    for s in (20, 100, 200, 300, 380):
        m = across(c, (60 + s, 100), (0, 1), 20)
        want = 20 * min(s, 400 - s) / 200
        worst = max(worst, abs(m - want))
        detail.append(f"{s}:{want:.2f}/{m:.2f}")
    rows.append(("taper-clamp", "400 px line, both lengths 5000 px (held to 200 px each), widths (distance:expected/measured), worst error px", " ".join(detail), f"{worst:.3f}", f"{worst:.3f}"))


def determinism():
    for name in ("palette-step", "taper-straight", "taper-closed"):
        sums = []
        for k in range(2):
            out = HERE / f"_det{k}.mp4"
            subprocess.run([BIN, "comp", "render", str(HERE / f"{name}.comp.json"), "--from", "0", "--to", "12", "--workers", "1", "--codec", "software", "--out", str(out)], capture_output=True, text=True, check=True)
            sums.append(subprocess.run(["shasum", "-a", "256", str(out)], capture_output=True, text=True).stdout.split()[0][:16])
            out.unlink()
        rows.append(("determinism", f"{name}: software-codec mp4, frames 0 to 12, rendered twice, sha256 of each", sums[0], sums[1], "0" if sums[0] == sums[1] else "DIFFERENT"))


if __name__ == "__main__":
    palette_step()
    palette_gradient()
    palette_pingpong_and_keyed()
    palette_opacity()
    taper_probes()
    determinism()
    with open(HERE / "data.tsv", "w") as f:
        f.write("probe\twhat\texpected\tmeasured\tabs_diff\n")
        for r in rows:
            f.write("\t".join(r) + "\n")
    for r in rows:
        print("\t".join(r))
