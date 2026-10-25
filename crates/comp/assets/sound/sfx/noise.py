"""Noise colours and crackle."""
from __future__ import annotations

import numpy as np
from scipy.signal import lfilter

from .core import n_of, rng

_PINK_B = [0.049922035, -0.095993537, 0.050612699, -0.004408786]
_PINK_A = [1, -2.494956002, 2.017265875, -0.522189400]


def white(dur: float, seed=0) -> np.ndarray:
    return rng("white", seed).standard_normal(n_of(dur))


def pink(dur: float, seed=0) -> np.ndarray:
    x = lfilter(_PINK_B, _PINK_A, rng("pink", seed).standard_normal(n_of(dur) + 2000))[2000:]
    return x / np.abs(x).max() * 2.5


def brown(dur: float, seed=0) -> np.ndarray:
    x = np.cumsum(rng("brown", seed).standard_normal(n_of(dur)))
    x = lfilter([1, -1], [1, -0.995], x)
    return x / (np.abs(x).max() + 1e-9) * 2.5


def blue(dur: float, seed=0) -> np.ndarray:
    return np.diff(rng("blue", seed).standard_normal(n_of(dur) + 1)) * 0.7


def violet(dur: float, seed=0) -> np.ndarray:
    return np.diff(rng("violet", seed).standard_normal(n_of(dur) + 2), 2) * 0.5


def colour(name: str, dur: float, seed=0) -> np.ndarray:
    return {"white": white, "pink": pink, "brown": brown, "blue": blue, "violet": violet}[name](dur, seed)


def crackle(dur: float, density: float = 60.0, seed=0, decay_ms: float = 2.0) -> np.ndarray:
    """Sparse random clicks, `density` per second, each a short decaying noise grain (paper, fire, vinyl)."""
    r = rng("crackle", seed)
    n = n_of(dur)
    out = np.zeros(n)
    k = max(2, int(48 * decay_ms))
    grain_env = np.exp(-np.arange(k) / (k / 5))
    for i in np.nonzero(r.random(n) < density / 48000)[0]:
        g = (r.standard_normal(k) * grain_env) * r.random() ** 2
        out[i : i + k] += g[: n - i]
    return out
