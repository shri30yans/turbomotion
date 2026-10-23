"""Generate the synthetic images the benchmark corpus film refers to (deterministic, CC0).

    python3 bench/make_corpus.py          # writes bench/corpus/public/{photos,depth}

The shots in bench/corpus/film.json cover every shot type and parameter variant the renderer
supports; only the photographs are synthetic: layered noise with edges and fine grain, so that
decoding, scaling and video encoding behave as they do on real footage.
"""
import json
from pathlib import Path

import numpy as np
from PIL import Image

CORPUS = Path(__file__).resolve().parent / 'corpus'


def octave(rng, w, h, cells):
    """Smooth random field at `cells` grid cells across the longer side, in [0, 1]."""
    gw, gh = max(2, cells), max(2, round(cells * h / w))
    grid = rng.random((gh, gw)).astype(np.float32)
    return np.asarray(Image.fromarray(grid).resize((w, h), Image.BICUBIC)).clip(0, 1)


def photo(rng, w, h):
    field = sum(octave(rng, w, h, 2 ** (i + 1)) / 2 ** (i * 0.8) for i in range(7))
    field = (field - field.min()) / (field.max() - field.min())
    hue = octave(rng, w, h, 3)[..., None]
    base = np.concatenate([field[..., None] * (0.7 + 0.3 * hue), field[..., None] * 0.85, field[..., None] * (1.1 - 0.4 * hue)], axis=2)
    ys, xs = np.mgrid[0:h, 0:w]
    for _ in range(14):  # hard-edged blocks and discs give the encoder real edges to work on
        cx, cy, r = rng.random() * w, rng.random() * h, (0.03 + 0.12 * rng.random()) * w
        mask = (xs - cx) ** 2 + (ys - cy) ** 2 < r * r if rng.random() < 0.5 else (abs(xs - cx) < r) & (abs(ys - cy) < r * 0.6)
        base[mask] = base[mask] * 0.55 + rng.random(3) * 0.45
    base += rng.normal(0, 0.012, base.shape)  # sensor grain
    return Image.fromarray((base.clip(0, 1) * 255).astype(np.uint8))


def depth(rng, w, h):
    ys, xs = np.mgrid[0:h, 0:w]
    centre = 1 - np.hypot((xs / w - 0.5) * 1.3, ys / h - 0.5)
    field = 0.6 * centre + 0.4 * octave(rng, w, h, 4)
    field = (field - field.min()) / (field.max() - field.min())
    return Image.fromarray((field * 255).astype(np.uint8))


def main():
    film = json.loads((CORPUS / 'film.json').read_text())
    public = CORPUS / 'public'
    for name, asset in sorted(film['assets'].items()):
        rng = np.random.default_rng(sum(map(ord, name)))
        for path, make in ((asset['file'], photo), (asset['depth'], depth)):
            target = public / path
            target.parent.mkdir(parents=True, exist_ok=True)
            image = make(rng, asset['width'], asset['height'])
            image.save(target, quality=90) if path.endswith('.jpg') else image.save(target)
            print(target.relative_to(CORPUS))


main()
