"""Envelopes and curves. All return arrays of `n_of(dur)` samples."""
from __future__ import annotations

import numpy as np

from .core import SR, n_of


def curve(points, dur: float, log: bool = False, ease: float = 1.0) -> np.ndarray:
    """Piecewise-linear (or log-frequency) curve through `(t_fraction, value)` points over `dur`.

    `ease` > 1 bends each segment slow-then-fast; log=True interpolates in log space (right for frequencies and gains).
    """
    n = n_of(dur)
    ts, vs = zip(*points)
    x = np.linspace(0, 1, n)
    if log:
        return np.exp(np.interp(x, ts, np.log(np.maximum(vs, 1e-9))))
    y = np.interp(x, ts, vs)
    if ease != 1.0:
        lo, hi = float(min(vs)), float(max(vs))
        y = lo + (hi - lo) * ((y - lo) / max(hi - lo, 1e-12)) ** ease
    return y


def exp_decay(dur: float, tau: float, attack: float = 0.0015) -> np.ndarray:
    t = np.arange(n_of(dur)) / SR
    return np.exp(-t / tau) * np.minimum(1.0, t / max(attack, 1e-5))


def adsr(dur: float, a: float, d: float, s: float, r: float) -> np.ndarray:
    n = n_of(dur)
    t = np.arange(n) / SR
    y = np.where(t < a, t / max(a, 1e-5), 1 - (1 - s) * np.minimum(1, (t - a) / max(d, 1e-5)))
    return y * np.clip((dur - t) / max(r, 1e-5), 0, 1)


def swell(dur: float, peak: float, rise: float = 2.0, fall: float = 1.5) -> np.ndarray:
    """0 -> 1 at `peak` seconds -> 0 at the end; rises as t^rise, falls as (1-t)^fall."""
    t = np.arange(n_of(dur)) / SR
    up = np.clip(t / max(peak, 1e-5), 0, 1) ** rise
    down = np.clip(1 - (t - peak) / max(dur - peak, 1e-5), 0, 1) ** fall
    return np.where(t < peak, up, down)


def ramp_up(dur: float, power: float = 2.0, tail: float = 0.0) -> np.ndarray:
    """Builds to 1 at `dur - tail` then is zero (a riser's climb, ending on a hard cut)."""
    n = n_of(dur)
    t = np.arange(n) / SR
    return np.clip(t / max(dur - tail, 1e-5), 0, 1) ** power * (t < dur - tail + 1e-9)


def lfo(rate, dur: float, depth: float = 1.0, shape: str = "sine", ph0: float = 0.0) -> np.ndarray:
    """Unipolar 0..1 low-frequency oscillator (`rate` may be a curve)."""
    from .osc import phase

    n = n_of(dur)
    ph = phase(rate, n) + ph0
    w = {"sine": 0.5 - 0.5 * np.cos(2 * np.pi * ph), "tri": np.abs(2 * (ph % 1) - 1), "saw": ph % 1, "sq": (ph % 1 < 0.5) * 1.0}[shape]
    return 1 - depth + depth * w


def gate_pattern(dur: float, times, width: float = 0.02) -> np.ndarray:
    """1 inside each `[t, t + width)` window; used for stutters and ratchets."""
    g = np.zeros(n_of(dur))
    for t in times:
        g[int(t * SR) : int((t + width) * SR)] = 1
    return g
