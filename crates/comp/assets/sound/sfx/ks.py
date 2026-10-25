"""Karplus-Strong plucked strings, tuned exactly by running the loop at an adjusted rate and resampling."""
from __future__ import annotations

import numpy as np
from scipy.signal import lfilter, resample

from .core import SR, n_of, rng
from .filt import lowpass


def pluck(freq: float, dur: float, decay: float = 0.996, brightness: float = 0.6, pick: float = 0.2, seed=0, stretch: float = 0.5):
    """A plucked string at `freq` Hz. `decay` < 1 is the loop gain per period; `brightness` 0..1 is the excitation's
    treble; `pick` is the pick position as a fraction of the string (a comb that removes harmonics).
    """
    n_loop = SR / freq - stretch
    N = max(4, int(n_loop))
    loop_sr = freq * (N + stretch)                      # the rate at which the integer loop plays exactly `freq`
    n_out = int((dur + 0.05) * loop_sr)
    r = rng("ks", seed, round(freq, 2))
    x = np.zeros(n_out)
    exc = r.standard_normal(N)
    exc = lowpass(exc, 600 + 9000 * brightness * loop_sr / SR) if brightness < 0.98 else exc
    exc -= exc.mean()
    k = max(1, int(pick * N))
    exc = exc - np.concatenate([np.zeros(k), exc[:-k]])  # pick-position comb
    x[:N] = exc
    a = np.zeros(N + 2)
    a[0], a[N], a[N + 1] = 1, -decay * 0.5, -decay * 0.5
    y = lfilter([1.0], a, x)
    y = resample(y, int(len(y) * SR / loop_sr))[: n_of(dur)]
    return y / (np.abs(y).max() + 1e-9)
