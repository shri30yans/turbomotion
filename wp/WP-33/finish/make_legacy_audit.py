#!/usr/bin/env python3
"""The audit's measurements of the grains that existed before `add_grain`: the platform `grain` effect and the project's
`film_grain` plugin, on flat grey cards (so the spread is the grain's). Writes legacy-grain-table.md. Run from turbomotion/."""
import json, pathlib, subprocess, numpy as np
from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[3]
BIN = ROOT / "target/release/turbomotion"
TMP = pathlib.Path("/tmp/fs-legacy"); TMP.mkdir(exist_ok=True)
FX_PATH = str(ROOT.parent / "motions/lib/effects")


def card(level, fx):
    return {"width": 256, "height": 256, "fps": 24, "dur": 1, "bg": "#000000", "fx_path": [FX_PATH],
            "layers": [{"type": "solid", "color": f"#{level:02x}{level:02x}{level:02x}", "fx": [fx]}]}


def measure(level, fx, name):
    path = TMP / f"{name}.json"
    path.write_text(json.dumps(card(level, fx)))
    subprocess.run([BIN, "comp", "still", path, "--out", TMP / name, "--times", "0.5"], check=True, capture_output=True)
    a = np.asarray(Image.open(TMP / name / "0000.50.png").convert("RGB")).astype(float)
    g = a[..., 1] - a[..., 1].mean()
    lag1 = (g[:, 1:] * g[:, :-1]).mean() / g.var()
    rg = np.corrcoef(a[..., 0].ravel(), a[..., 1].ravel())[0, 1]
    return a[..., 1].mean(), a[..., 1].std(), lag1, rg


lines = ["| effect | grey level | mean | std (levels) | lag-1 autocorrelation | R-G correlation |", "|---|---|---|---|---|---|"]
for label, fx in [("`grain` amount 0.1", {"effect": "grain", "amount": 0.1}), ("`film_grain` amount 0.12", {"effect": "film_grain", "amount": 0.12})]:
    for level in (32, 128, 224):
        m, s, l1, rg = measure(level, fx, f"{label.split()[0].strip('`')}-{level}")
        lines.append(f"| {label} | {level} | {m:.1f} | {s:.2f} | {l1:.3f} | {rg:.3f} |")
(pathlib.Path(__file__).resolve().parent / "legacy-grain-table.md").write_text("\n".join(lines) + "\n")
print("\n".join(lines))
