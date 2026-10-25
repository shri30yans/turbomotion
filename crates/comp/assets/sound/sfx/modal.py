"""Modal synthesis of struck objects: a bank of decaying sinusoids (the object's modes) driven by a short contact burst."""
from __future__ import annotations

import numpy as np

from .core import SR, n_of, rng
from .filt import bandpass, highpass, lowpass
from .noise import white

# material -> (mode ratios, per-mode decay in seconds at the fundamental, brightness exponent, decay slope with ratio)
MATERIALS = {
    "metal": ([1, 2.756, 5.404, 8.933, 13.344, 18.64], 1.6, 0.9, 0.55),     # free bar / plate: bright, long
    "glass": ([1, 2.32, 4.25, 6.63, 9.38, 12.5], 1.1, 0.7, 0.5),          # wine-glass rim modes
    "wood": ([1, 2.57, 4.64, 7.1, 9.9], 0.14, 1.1, 0.65),                  # wood block: dull, quick
    "plastic": ([1, 1.83, 2.9, 4.1, 5.6], 0.06, 1.0, 0.7),                 # hollow plastic: tick
    "bell": ([0.5, 1.0, 1.19, 1.56, 2.0, 2.51, 3.01, 4.17], 3.5, 0.6, 0.45),  # hum, prime, minor tierce ...
    "bar": ([1, 4.0, 10.0, 17.6], 0.9, 0.5, 0.6),                          # tuned (marimba) bar
    "membrane": ([1, 1.59, 2.14, 2.3, 2.65, 2.92], 0.35, 0.9, 0.7),       # drum skin Bessel modes
    "can": ([1, 1.47, 2.09, 2.56, 3.38, 4.8], 0.22, 0.8, 0.6),            # small tin / pot
}


def strike(f0: float, material: str, dur: float, hardness: float = 0.5, seed=0, decay: float = 1.0, detune: float = 0.004, spread: float = 0.0):
    """Hit an object whose lowest mode is `f0`. `hardness` 0..1 brightens upper modes; `decay` scales ring time.

    Returns mono. `spread` > 0 returns stereo by panning modes across the field.
    """
    ratios, t60, bright, slope = MATERIALS[material]
    r = rng("strike", material, seed, round(f0, 1))
    t = np.arange(n_of(dur)) / SR
    out = np.zeros((len(t), 2)) if spread else np.zeros(len(t))
    for k, ratio in enumerate(ratios):
        f = f0 * ratio * (1 + (detune * r.standard_normal() if k else 0.0))
        if f > SR * 0.45:
            continue
        amp = ratio ** (-bright * (1.6 - hardness * 1.2)) * (0.6 + 0.4 * r.random())
        tau = max(0.004, t60 * decay / (1 + slope * (ratio - 1)) / 6.9)
        m = amp * np.exp(-t / tau) * np.sin(2 * np.pi * f * t + r.random() * 6.28)
        if spread:
            p = (r.random() * 2 - 1) * spread
            out += np.stack([m * np.cos((p + 1) * np.pi / 4), m * np.sin((p + 1) * np.pi / 4)], 1)
        else:
            out += m
    return out / (np.abs(out).max() + 1e-9)


def contact(dur: float = 0.012, centre: float = 3000.0, seed=0, width: float = 1.5) -> np.ndarray:
    """The brief noise of two things meeting: band-limited noise with a very fast decay."""
    n = white(dur, ("contact", seed))
    n = bandpass(n, centre / width, min(centre * width, 20000)) * np.exp(-np.arange(len(n)) / SR / (dur / 5))
    return n / (np.abs(n).max() + 1e-9)
