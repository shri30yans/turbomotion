#!/usr/bin/env python3
"""Writes the WP-10 probe films next to this file and samples them with `comp inspect`.

    python3 wp/WP-10/probes/make.py [path/to/turbomotion]

Films: `circle.comp.json` (a 4-key circle with auto_orient: path, and the same circle from Auto Bezier keys),
`motion.comp.json` (an S path with roving keys, speed/influence handles against a CSS cubic, a hold key).
Output: `data.tsv` (`film layer t x y`), `speeds.tsv` (per layer: speed in px/s between samples) and `numbers.txt`
(the figures quoted in README.md). Images come from `comp sheet`, run by `render.sh`."""
import json, math, subprocess, sys
from pathlib import Path

HERE = Path(__file__).parent
ENGINE = HERE.parents[2]
BIN = sys.argv[1] if len(sys.argv) > 1 else str(ENGINE / "target/release/turbomotion")
KAPPA = 0.5522847498307936  # the Bezier handle length of a quarter circle, in radii
CX, CY, R = 640, 360, 250
ARROW = {"closed": True, "fill": "#ffd166", "points": [[-40, -20], [40, 0], [-40, 20], [-20, 0]]}
DOT = lambda c, r=14: {"ellipse": [-r, -r, 2 * r, 2 * r], "fill": c}


def comp(layers, dur):
    return {"width": 1280, "height": 720, "dur": dur, "fps": 30, "bg": "#10131c", "layers": layers}


def ring(n=72):
    """The ideal circle as small grey dots, the reference the eye compares against."""
    return {"type": "shape", "items": [{"ellipse": [CX + R * math.cos(a) - 3, CY + R * math.sin(a) - 3, 6, 6], "fill": "#3a4054"} for a in (2 * math.pi * i / n for i in range(n))]}


def circle():
    # y runs down: right, bottom, left, top, right again; each handle is KAPPA * R along the direction of travel
    pts = [(CX + R, CY), (CX, CY + R), (CX - R, CY), (CX, CY - R), (CX + R, CY)]
    tang = [(0, 1), (-1, 0), (0, -1), (1, 0), (0, 1)]
    keys = []
    for i, ((x, y), (dx, dy)) in enumerate(zip(pts, tang)):
        k = {"t": i, "v": [x, y], "spatial": "bezier", "ease": "none"}
        if i > 0:
            k["in"] = [-dx * KAPPA * R, -dy * KAPPA * R]
        if i < 4:
            k["out"] = [dx * KAPPA * R, dy * KAPPA * R]
        keys.append(k)
    auto = [{"t": i, "v": [x, y], "ease": "none"} for i, (x, y) in enumerate(pts)]
    return comp([ring(),
                 {"type": "shape", "id": "arrow", "items": [ARROW], "pos": {"keys": keys, "auto_orient": "path"}},
                 {"type": "shape", "id": "auto", "items": [DOT("#ef476f", 10)], "pos": {"keys": auto}}], 4)


def motion():
    s_keys = [{"t": 0, "v": [140, 560], "ease": "none"}, {"roving": True, "v": [420, 150]}, {"roving": True, "v": [760, 600]},
              {"t": 4, "v": [1140, 200], "ease": "none"}]
    # speed 0 with 33 % and 67 % influence is cubic-bezier(0.33, 0, 0.67, 1) in the unit square
    handles = [{"t": 0, "v": [140, 660], "ease_out": {"speed": 0, "influence": 33}}, {"t": 4, "v": [1140, 660], "ease_in": {"speed": 0, "influence": 33}}]
    cubic = [{"t": 0, "v": [140, 700], "ease": "none"}, {"t": 4, "v": [1140, 700], "ease": "cubic(0.33,0,0.67,1)"}]
    hold = [{"t": 0, "v": [140, 40], "ease_out": "hold"}, {"t": 1, "v": [440, 40], "ease_out": "hold"}, {"t": 2, "v": [740, 40], "ease_out": "hold"}, {"t": 3, "v": [1140, 40]}]
    return comp([{"type": "shape", "id": "s_path", "items": [DOT("#06d6a0")], "pos": {"keys": s_keys}},
                 {"type": "shape", "id": "handles", "items": [DOT("#ffd166")], "pos": {"keys": handles}},
                 {"type": "shape", "id": "cubic", "items": [DOT("#118ab2", 8)], "pos": {"keys": cubic}},
                 {"type": "shape", "id": "hold", "items": [{"rect": [-14, -14, 28, 28], "fill": "#ef476f"}], "pos": {"keys": hold}}], 4)


def sample(path, layer, times):
    out = []
    for t in times:
        d = json.loads(subprocess.check_output([BIN, "comp", "inspect", str(path), "--t", f"{t:.6f}"]))
        l = next(x for x in d["layers"] if x["id"] == layer)
        out.append(tuple(l["origin"]))
    return out


if __name__ == "__main__":
    films = {"circle": (circle(), ["arrow", "auto"]), "motion": (motion(), ["s_path", "handles", "cubic", "hold"])}
    rows, speeds = ["film\tlayer\tt\tx\ty"], ["film\tlayer\tt\tspeed_px_s"]
    pos = {}
    for name, (c, layers) in films.items():
        path = HERE / f"{name}.comp.json"
        path.write_text(json.dumps(c, indent=1) + "\n")
        dur = c["dur"]
        times = [i / 10 for i in range(0, dur * 10 + 1)]
        for layer in layers:
            xy = sample(path, layer, times)
            pos[(name, layer)] = (times, xy)
            for t, (x, y) in zip(times, xy):
                rows.append(f"{name}\t{layer}\t{t:.1f}\t{x:.2f}\t{y:.2f}")
            for i in range(1, len(times)):
                v = math.dist(xy[i], xy[i - 1]) / (times[i] - times[i - 1])
                speeds.append(f"{name}\t{layer}\t{times[i]:.1f}\t{v:.1f}")
    (HERE / "data.tsv").write_text("\n".join(rows) + "\n")
    (HERE / "speeds.tsv").write_text("\n".join(speeds) + "\n")
    n = []
    for layer in ("arrow", "auto"):
        t, xy = pos[("circle", layer)]
        # radius error at the 0.1 s samples (the curve is fixed at the key times; between them it is the curve itself)
        err = [abs(math.dist(p, (CX, CY)) - R) for p in xy]
        n.append(f"circle {layer}: radius error over 41 samples max {max(err):.3f} px, mean {sum(err)/len(err):.3f} px")
    t, xy = pos[("circle", "arrow")]
    sp = [math.dist(xy[i], xy[i - 1]) / 0.1 for i in range(1, len(xy))]
    n.append(f"circle arrow: speed min {min(sp):.1f} max {max(sp):.1f} px/s (a circle of 2*pi*{R} = {2*math.pi*R:.1f} px in 4 s is {2*math.pi*R/4:.1f})")
    t, xy = pos[("circle", "arrow")]
    n.append(f"circle arrow at t=0.5 (the arc midpoint of the first quarter): {xy[5][0]:.2f},{xy[5][1]:.2f}, "
             f"angle from the centre {math.degrees(math.atan2(xy[5][1]-CY, xy[5][0]-CX)):.3f} deg (45 expected)")
    t, xy = pos[("motion", "s_path")]
    sp = [math.dist(xy[i], xy[i - 1]) / 0.1 for i in range(1, len(xy))]
    n.append(f"motion s_path (roving): speed min {min(sp):.1f} max {max(sp):.1f} px/s")
    h = pos[("motion", "handles")][1]; cu = pos[("motion", "cubic")][1]
    n.append(f"motion handles vs cubic(0.33,0,0.67,1): max |x difference| {max(abs(a[0]-b[0]) for a, b in zip(h, cu)):.4f} px")
    ho = pos[("motion", "hold")][1]
    n.append("motion hold x at t=0.9,1.0,1.9,2.0,2.9,3.0: " + ", ".join(f"{ho[i][0]:.1f}" for i in (9, 10, 19, 20, 29, 30)))
    (HERE / "numbers.txt").write_text("\n".join(n) + "\n")
    print("\n".join(n))
