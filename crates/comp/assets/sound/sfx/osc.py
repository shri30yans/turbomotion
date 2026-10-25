"""Oscillators: sine, saw, square, triangle, FM, additive, detuned stacks. Frequencies may be a number or a per-sample curve."""
from __future__ import annotations

import numpy as np

from .core import SR, n_of, rng


def _f(freq, n: int) -> np.ndarray:
    f = np.asarray(freq, dtype=float)
    return np.full(n, float(f)) if f.ndim == 0 else np.interp(np.linspace(0, 1, n), np.linspace(0, 1, len(f)), f)


def phase(freq, n: int) -> np.ndarray:
    """Phase in cycles (0..) from a frequency in Hz that may change over time."""
    return np.cumsum(_f(freq, n) / SR)


def sine(freq, dur: float, ph0: float = 0.0) -> np.ndarray:
    n = n_of(dur)
    return np.sin(2 * np.pi * (phase(freq, n) + ph0))


def _blep(t: np.ndarray, dt: np.ndarray) -> np.ndarray:
    """PolyBLEP residual that removes the aliasing step at a waveform discontinuity."""
    out = np.zeros_like(t)
    a = t < dt
    x = t[a] / dt[a]
    out[a] = x + x - x * x - 1
    b = t > 1 - dt
    x = (t[b] - 1) / dt[b]
    out[b] = x * x + x + x + 1
    return out


def saw(freq, dur: float, ph0: float = 0.0) -> np.ndarray:
    n = n_of(dur)
    dt = np.clip(_f(freq, n) / SR, 1e-6, 0.45)
    t = (phase(freq, n) + ph0) % 1.0
    return 2 * t - 1 - _blep(t, dt)


def square(freq, dur: float, width: float = 0.5, ph0: float = 0.0) -> np.ndarray:
    n = n_of(dur)
    dt = np.clip(_f(freq, n) / SR, 1e-6, 0.45)
    t = (phase(freq, n) + ph0) % 1.0
    y = np.where(t < width, 1.0, -1.0)
    return y + _blep(t, dt) - _blep((t - width) % 1.0, dt)


def triangle(freq, dur: float, ph0: float = 0.0) -> np.ndarray:
    return 2 * np.abs(saw(freq, dur, ph0 + 0.25)) - 1


def fm(fc, ratio: float, index, dur: float, feedback: float = 0.0) -> np.ndarray:
    """Two-operator FM: carrier at `fc`, modulator at `fc * ratio`, `index` (number or curve) is the depth in radians."""
    n = n_of(dur)
    fcn = _f(fc, n)
    mod = np.sin(2 * np.pi * phase(fcn * ratio, n))
    if feedback:
        mod = np.sin(2 * np.pi * phase(fcn * ratio, n) + feedback * np.concatenate([[0], mod[:-1]]))
    return np.sin(2 * np.pi * phase(fcn, n) + _f(index, n) * mod)


def additive(freqs, amps, dur: float, decays=None) -> np.ndarray:
    """A sum of sine partials; `decays` (seconds, per partial) make each one fade independently."""
    n = n_of(dur)
    t = np.arange(n) / SR
    out = np.zeros(n)
    for i, (f, a) in enumerate(zip(freqs, amps)):
        if f * 2 >= SR:
            continue
        env = np.exp(-t / decays[i]) if decays is not None else 1.0
        out += a * env * np.sin(2 * np.pi * f * t + 6.2832 * (i * 0.37 % 1))
    return out


def stack(kind, freq, dur: float, voices: int = 5, detune_cents: float = 12, seed: int | str = 0) -> np.ndarray:
    """Detuned unison of `saw`/`square`/`sine` voices (a 'supersaw' when kind is saw)."""
    r = rng("stack", seed)
    out = np.zeros(n_of(dur))
    for v in range(voices):
        c = (v - (voices - 1) / 2) / max(1, (voices - 1) / 2) * detune_cents
        f = np.asarray(freq, dtype=float) * 2 ** (c / 1200)
        out += kind(f, dur, ph0=r.random()) if kind is not sine else sine(f, dur, r.random())
    return out / voices
