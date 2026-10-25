"""Listening aids: `kit/audition.wav`  and a spectrogram sheet per family.

Run `python3 -m sfx.audition` from assets/sound/ after `python3 -m sfx.build`.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy.io import wavfile

from .core import SR, fade, stereo, write_wav

KIT = Path(__file__).resolve().parent.parent / "kit"
GAP, FAMILY_GAP, MAX_CLIP = 0.2, 0.6, 1.2


def load(path: Path) -> np.ndarray:
    sr, x = wavfile.read(path)
    assert sr == SR
    return x.astype(np.float64) / 32768


def pick_variants(names: list[str], n: int = 3) -> list[str]:
    """Spread the choice across the family's list: first, middle, last."""
    if len(names) <= n:
        return names
    return [names[round(i * (len(names) - 1) / (n - 1))] for i in range(n)]


def audition(index: dict) -> dict[str, float]:
    """Write audition.wav (mono, 3 variants per family, each clipped to 1.2 s); returns where each family starts."""
    parts, marks, t = [], {}, 0.0
    for fam, f in index["families"].items():
        marks[fam] = round(t, 2)
        for name in pick_variants(list(f["variants"]), 2 if "pitched" in f else 3):
            x = load(KIT / fam / f"{name}.wav")
            x = x.mean(1) if x.ndim == 2 else x
            x = fade(x[: int(MAX_CLIP * SR)], 2, 40 if len(x) > MAX_CLIP * SR else 4)
            parts += [x, np.zeros(int(GAP * SR))]
            t += len(x) / SR + GAP
        parts.append(np.zeros(int((FAMILY_GAP - GAP) * SR)))
        t += FAMILY_GAP - GAP
    write_wav(KIT / "audition.wav", np.concatenate(parts))
    (KIT / "audition.json").write_text(json.dumps(marks, indent=1) + "\n")
    return marks


def spectrogram(x: np.ndarray, w: int, h: int, fmax: float = 16000.0) -> np.ndarray:
    """dB spectrogram image (h x w, 0..255): log-frequency vertical axis, time horizontal."""
    x = x.mean(1) if x.ndim == 2 else x
    n_fft, hop = 1024, max(32, len(x) // w)
    win = np.hanning(n_fft)
    frames = np.stack([np.abs(np.fft.rfft(np.pad(x, (0, n_fft))[i : i + n_fft] * win)) for i in range(0, max(1, len(x) - 1), hop)])
    freqs = np.fft.rfftfreq(n_fft, 1 / SR)
    rows = np.geomspace(50, fmax, h)
    idx = np.clip(np.searchsorted(freqs, rows), 0, len(freqs) - 1)
    db = 20 * np.log10(frames[:, idx].T + 1e-7)
    db = np.clip((db - (db.max() - 56)) / 56, 0, 1)[::-1]
    img = np.array(Image.fromarray((db * 255).astype(np.uint8)).resize((w, h), Image.BILINEAR))
    return img


def heat(v: np.ndarray) -> np.ndarray:
    """A black -> purple -> orange -> white ramp."""
    stops = np.array([[0, 0, 0], [60, 10, 100], [200, 50, 80], [250, 160, 30], [255, 255, 220]], dtype=float)
    pos = v / 255 * (len(stops) - 1)
    lo = np.clip(pos.astype(int), 0, len(stops) - 2)
    f = (pos - lo)[..., None]
    return (stops[lo] * (1 - f) + stops[lo + 1] * f).astype(np.uint8)


def sheet(fam: str, f: dict, cols: int = 4, tile=(300, 120)) -> None:
    names = list(f["variants"])
    rows = (len(names) + cols - 1) // cols
    W, H, pad, lab = tile[0], tile[1], 6, 16
    img = Image.new("RGB", (cols * (W + pad) + pad, rows * (H + lab + pad) + pad + 20), (14, 14, 18))
    d = ImageDraw.Draw(img)
    d.text((pad, 4), f"{fam}: {f['desc']}", fill=(230, 230, 235))
    for i, name in enumerate(names):
        x, y = pad + (i % cols) * (W + pad), 20 + pad + (i // cols) * (H + lab + pad)
        v = f["variants"][name]
        tile_img = Image.fromarray(heat(spectrogram(load(KIT / fam / f"{name}.wav"), W, H)))
        img.paste(tile_img, (x, y + lab))
        d.text((x, y), f"{name}  {v['dur']:.2f}s  peak {v['peak']:.2f}s  {v['centroid']} Hz", fill=(200, 200, 210))
    (KIT / "sheets").mkdir(exist_ok=True)
    img.quantize(48, dither=Image.Dither.NONE).save(KIT / "sheets" / f"{fam}.png", optimize=True)


def main() -> None:
    index = json.loads((KIT / "index.json").read_text())
    marks = audition(index)
    for fam, f in index["families"].items():
        sheet(fam, f)
    print(f"audition.wav: {len(marks)} families; sheets in {KIT / 'sheets'}")


if __name__ == "__main__":
    sys.exit(main())
