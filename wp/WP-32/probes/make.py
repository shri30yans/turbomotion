#!/usr/bin/env python3
"""WP-32 step 1 probes: writes the four probe films, renders them with the engine, scans the drawn box out of the pixels and
holds it against what `comp inspect` measures. Prints and writes data.tsv.   usage: make.py BIN"""
import json, subprocess, sys, tempfile
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
BIN = sys.argv[1]
ROOT = "../../../../motions/lib/fonts"  # from this folder to motions/lib/fonts
FONTS = ["Manrope-Variable.ttf", "InstrumentSerif-Regular.woff", "JetBrainsMono-Regular.woff"]
BG, RED, GOLD, INK = "#10141f", "#e63946", "#e2c48a", "#10141f"
rows = []  # (probe, what, measured, drawn)


def rgb(hex_):
    return np.array([int(hex_[i:i + 2], 16) for i in (1, 3, 5)], dtype=float)


def film(w, h, layers, **extra):
    return {"width": w, "height": h, "fps": 30, "dur": 3, "bg": BG, "root": ROOT, **extra, "layers": layers}


def text(id_, s, font, size, pos, box, **kw):
    return {"id": id_, "type": "text", "text": s, "font": font, "weight": 400, "size": size, "color": kw.pop("color", "#ffffff"), "pos": pos, "box": box, **kw}


def box(fill=RED, padding=(28, 14), radius=18, **kw):
    return {"fit": "text", "padding": list(padding), "radius": radius, "fill": fill, **kw}


def save(name, comp):
    path = HERE / f"{name}.comp.json"
    path.write_text(json.dumps(comp, indent=1) + "\n")
    return path


def inspect(path, t, layer):
    out = subprocess.run([BIN, "comp", "inspect", str(path), "--t", str(t), "--layer", layer], capture_output=True, text=True, check=True).stdout
    return json.loads(out)["layers"][0]


def still(path, t, out_dir):
    subprocess.run([BIN, "comp", "still", str(path), "--times", str(t), "--out", str(out_dir)], capture_output=True, text=True, check=True)
    return np.asarray(Image.open(out_dir / f"{t:07.2f}.png").convert("RGB"), dtype=float)


def cover(img, fill, bg=BG):
    """How much of each pixel is box: 0 on the ground, 1 in the fill, along the channel that differs most."""
    f, b = rgb(fill), rgb(bg)
    c = int(np.argmax(np.abs(f - b)))
    return np.clip((img[:, :, c] - b[c]) / (f[c] - b[c]), 0, 1)


def run(line):
    """The two edges of the run of cover along a line: whole pixels plus the part of the edge pixels."""
    on = np.nonzero(line > 0.001)[0]
    first, last = on[0], on[-1]
    return first + 1 - line[first], last + line[last]


def screen_box(layer, rect, pos):
    a = layer["content"]["anchor"]
    return [pos[0] - a[0] + rect[0], pos[1] - a[1] + rect[1], pos[0] - a[0] + rect[2], pos[1] - a[1] + rect[3]]


def drawn_width(cov, rect_screen, pad=None, middle=True):
    """Width of the box along the row through its middle, from its first covered pixel to its last: ink inside it is not a gap."""
    row = int((rect_screen[1] + rect_screen[3]) / 2)
    left, right = run(cov[row])
    return right - left, left


def tmp(name):
    d = Path(tempfile.mkdtemp(prefix=f"wp32-{name}-"))
    return d


# 1. widths in three fonts, kerning pairs and two lines
def fonts():
    layers = [text(f"f{i}", "AVATAR Wave", f, 96, [120, 150 + 190 * i], box()) for i, f in enumerate(FONTS)]
    layers.append(text("lines", "Wave\nTo AVATAR", FONTS[0], 72, [120, 730], box(GOLD, (28, 14), 10, per_line=True), color=INK))
    path = save("fonts", film(1600, 900, layers))
    out = tmp("fonts")
    img = still(path, 0, out)
    Image.fromarray(img.astype(np.uint8)).save(HERE / "fonts.png")
    for layer in layers:
        info = inspect(path, 0, layer["id"])
        c = info["content"]
        pad = layer["box"]["padding"]
        fill = layer["box"]["fill"]
        cov = cover(img, fill)
        for k, r in enumerate(c["text_box"]["rects"]):
            sb = screen_box(info, r, layer["pos"])
            dw, _ = drawn_width(cov, sb, pad)
            block = c["bounds"]["lines"][k]["rect"] if layer["box"].get("per_line") else c["bounds"]["block"]
            what = f"{layer['font'].split('-')[0]} {layer['text']!r}" + (f" line {k}" if layer["box"].get("per_line") else "")
            rows.append(("fonts", what, block["width"] + 2 * pad[0], dw))


# 2. a typed bar grows; also the bar of the revealed prefix typed out as a static string (what a swap would draw)
def typed():
    s, font, at, each = "To Yo-yo Wave", FONTS[1], 0.2, 0.12
    comp = film(1600, 300, [text("t", s, font, 110, [120, 150], box(RED, (30, 16), 20), preset={"name": "typewriter", "at": at, "each": each})])
    path = save("typed", comp)
    out, frames = tmp("typed"), []
    times = [0.1, 0.26, 0.5, 0.86, 1.22, 1.7, 2.5]
    for t in times:
        img = still(path, t, out)
        frames.append(img)
        info = inspect(path, t, "t")
        rects = info["content"]["text_box"]["rects"]
        n = 0 if t < at else min(int((t - at) / each) + 1, len(s.replace(" ", "")))
        if not rects:
            rows.append(("typed", f"t={t}: {n} characters, no box (nothing drawn)", 0.0, float(cover(img, RED).sum())))
            continue
        prefix, seen = "", 0
        for ch in s:
            prefix += ch
            seen += ch != " "
            if seen == n:
                break
        static = save_tmp(f"prefix{n}", film(1600, 300, [text("t", prefix, font, 110, [120, 150], box(RED, (30, 16), 20))]))
        static_w = inspect(static, 0, "t")["content"]["text_box"]["rects"][0]
        sb = screen_box(info, rects[0], [120, 150])
        dw, _ = drawn_width(cover(img, RED), sb, (30, 16), middle=True)
        rows.append(("typed", f"t={t}: {n} chars {prefix!r}: typed bar (inspect {rects[0][2] - rects[0][0]:.2f}) against the prefix laid out alone", static_w[2] - static_w[0], dw))
    Image.fromarray(np.vstack(frames).astype(np.uint8)).save(HERE / "typed.png")


def save_tmp(name, comp):
    p = Path(tempfile.mkdtemp(prefix="wp32-")) / f"{name}.comp.json"
    comp = dict(comp, root=str((HERE / ROOT).resolve()))
    p.write_text(json.dumps(comp))
    return p


# 3. layer space: the box takes the layer's scale and turn
def layer_space():
    kw = dict(effect_space="layer")
    fx = [{"effect": "blur", "radius": 2}]
    layers = [
        text("s1", "Wave AV", FONTS[1], 80, [140, 150], box(RED, (24, 12), 20), fx=fx),
        text("s2", "Wave AV", FONTS[1], 80, [160, 450], box(RED, (24, 12), 20), fx=fx, scale=2),
        text("s3", "Wave AV", FONTS[1], 80, [1000, 380], box(GOLD, (24, 12), 20), fx=fx, scale=2, rotation=30, color=INK),
    ]
    path = save("layer-space", film(1700, 800, layers, **kw))
    img = still(path, 0, tmp("ls"))
    Image.fromarray(img.astype(np.uint8)).save(HERE / "layer-space.png")
    widths = {}
    for layer in layers[:2]:
        info = inspect(path, 0, layer["id"])
        r = info["content"]["text_box"]["rects"][0]
        s = layer.get("scale", 1)
        sb = screen_box(info, r, layer["pos"])
        # scale about the anchor: the anchor sits on pos
        a = info["content"]["anchor"]
        sb = [layer["pos"][0] + s * (r[0] - a[0]), layer["pos"][1] + s * (r[1] - a[1]), layer["pos"][0] + s * (r[2] - a[0]), layer["pos"][1] + s * (r[3] - a[1])]
        cov = cover(img, RED)
        row = int((sb[1] + sb[3]) / 2)
        line = cov[row]
        up = [i for i in range(len(line) - 1) if line[i] < 0.5 <= line[i + 1] and i > sb[0] - 20 and i < sb[0] + 20][0]
        dn = [i for i in range(len(line) - 1) if line[i] >= 0.5 > line[i + 1] and abs(i - sb[2]) < 20][0]
        edge = lambda i: i + 0.5 + (0.5 - line[i]) / (line[i + 1] - line[i])
        widths[layer["id"]] = edge(dn) - edge(up)
        rows.append(("layer-space", f"{layer['id']} scale {s}: {s} x the {r[2] - r[0]:.2f} px box (half-cover crossings)", s * (r[2] - r[0]), widths[layer["id"]]))
    rows.append(("layer-space", "scale 2 box over scale 1 box (ratio)", 2.0, widths["s2"] / widths["s1"]))


# 4. min_width, align and radius, and a keyed padding
def fit():
    layers = []
    for i, align in enumerate(["left", "center", "right"]):
        layers.append(text(f"a-{align}", "Hi", FONTS[0], 80, [500, 140 + 130 * i], box(RED, (24, 12), 0, min_width=360, align=align)))
    for i, radius in enumerate([0, 24, 1000]):
        layers.append(text(f"r-{radius}", "Pill", FONTS[0], 80, [130 + 330 * i, 560], box(GOLD, (36, 14), radius), color=INK))
    keyed = text("pad", "Wave", FONTS[0], 80, [140, 760], box(RED, (0, 0), 14))
    keyed["box"]["padding"] = [{"from": 4, "to": 52, "at": 0, "dur": 2, "ease": "none"}, {"from": 0, "to": 24, "at": 0, "dur": 2, "ease": "none"}]
    layers.append(keyed)
    path = save("fit", film(1400, 900, layers))
    out, frames = tmp("fit"), []
    for t in (0, 1, 2):
        img = still(path, t, out)
        frames.append(img)
        if t == 0:
            Image.fromarray(img.astype(np.uint8)).save(HERE / "fit.png")
        for layer in layers[:3]:
            info = inspect(path, t, layer["id"])
            r = info["content"]["text_box"]["rects"][0]
            sb = screen_box(info, r, layer["pos"])
            dw, left = drawn_width(cover(img, RED), sb, (24, 12))
            if t == 0:
                rows.append(("fit", f"min_width 360 align {layer['box']['align']}: width / left edge {sb[0]:.2f}", r[2] - r[0], dw))
        info = inspect(path, t, "pad")
        r = info["content"]["text_box"]["rects"][0]
        sb = screen_box(info, r, [140, 760])
        pad = (4 + (52 - 4) * t / 2, 24 * t / 2)
        dw, _ = drawn_width(cover(img, RED), sb, pad, middle=True)
        text_w = info["content"]["bounds"]["block"]["width"]
        rows.append(("fit", f"keyed padding t={t}: padding x {pad[0]:.0f}: text {text_w:.2f} + 2 x {pad[0]:.0f}", text_w + 2 * pad[0], dw))
    Image.fromarray(np.hstack([f[680:860, 80:500] for f in frames]).astype(np.uint8)).save(HERE / "keyed-padding.png")


fonts()
typed()
layer_space()
fit()
with open(HERE / "data.tsv", "w") as f:
    f.write("probe\twhat\tmeasured_px\tdrawn_px\tabs_diff_px\n")
    for probe, what, m, d in rows:
        f.write(f"{probe}\t{what}\t{m:.3f}\t{d:.3f}\t{abs(m - d):.3f}\n")
worst = max(abs(m - d) for _, _, m, d in rows)
print(open(HERE / "data.tsv").read())
print(f"worst |measured - drawn| = {worst:.3f} px over {len(rows)} rows")
