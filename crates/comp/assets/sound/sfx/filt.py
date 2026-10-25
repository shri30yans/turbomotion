"""Filters: RBJ biquads (fixed or swept per block), Butterworth shortcuts, resonators, comb/flanger."""
from __future__ import annotations

import numpy as np
from scipy.signal import butter, lfilter, sosfilt

from .core import SR


def biquad(kind: str, f: float, q: float = 0.707, gain_db: float = 0.0):
    """RBJ cookbook coefficients `(b, a)` for lowpass, highpass, bandpass, notch, peak."""
    f = float(np.clip(f, 20.0, SR * 0.45))
    w = 2 * np.pi * f / SR
    c, al = np.cos(w), np.sin(w) / (2 * max(q, 0.05))
    A = 10 ** (gain_db / 40)
    b, a = {
        "lowpass": ([(1 - c) / 2, 1 - c, (1 - c) / 2], [1 + al, -2 * c, 1 - al]),
        "highpass": ([(1 + c) / 2, -(1 + c), (1 + c) / 2], [1 + al, -2 * c, 1 - al]),
        "bandpass": ([al, 0, -al], [1 + al, -2 * c, 1 - al]),
        "notch": ([1, -2 * c, 1], [1 + al, -2 * c, 1 - al]),
        "peak": ([1 + al * A, -2 * c, 1 - al * A], [1 + al / A, -2 * c, 1 - al / A]),
    }[kind]
    a0 = a[0]
    return np.array(b) / a0, np.array(a) / a0


def filt(x: np.ndarray, kind: str, f: float, q: float = 0.707, gain_db: float = 0.0) -> np.ndarray:
    b, a = biquad(kind, f, q, gain_db)
    return lfilter(b, a, x, axis=0)


def sweep(x: np.ndarray, kind: str, f, q=0.9, block: int = 96) -> np.ndarray:
    """A biquad whose cutoff `f` (number or per-sample curve) and `q` move over time, with state carried across blocks."""
    n = len(x)
    f = np.full(n, float(f)) if np.ndim(f) == 0 else np.asarray(f, dtype=float)
    q = np.full(n, float(q)) if np.ndim(q) == 0 else np.asarray(q, dtype=float)
    out = np.zeros_like(x, dtype=float)
    zi = None
    for s in range(0, n, block):
        e = min(n, s + block)
        m = (s + e) // 2
        b, a = biquad(kind, f[m], q[m])
        if zi is None:
            zi = np.zeros((2,) + x.shape[1:])
        out[s:e], zi = lfilter(b, a, x[s:e], axis=0, zi=zi)
    return out


def lowpass(x, fc, order=2):
    return sosfilt(butter(order, min(fc, SR * 0.45), "low", fs=SR, output="sos"), x, axis=0)


def highpass(x, fc, order=2):
    return sosfilt(butter(order, max(fc, 10), "high", fs=SR, output="sos"), x, axis=0)


def bandpass(x, lo, hi, order=2):
    return sosfilt(butter(order, [max(lo, 10), min(hi, SR * 0.45)], "band", fs=SR, output="sos"), x, axis=0)


def shelf_high(x, fc, gain_db):
    """Boost or cut everything above `fc` by adding a scaled highpassed copy."""
    return x + (10 ** (gain_db / 20) - 1) * highpass(x, fc)


def resonator(x, f, decay_s, gain=1.0):
    """A ringing mode at `f`: an impulse through this rings for `decay_s` seconds (T60)."""
    r = 10 ** (-3 / (decay_s * SR))
    w = 2 * np.pi * f / SR
    return lfilter([gain * np.sin(w)], [1, -2 * r * np.cos(w), r * r], x, axis=0)


def comb(x, delay_ms: float, feedback: float = 0.6, mix: float = 1.0):
    """Feedback comb filter (metallic ringing, flange)."""
    d = max(1, int(SR * delay_ms / 1000))
    a = np.zeros(d + 1)
    a[0], a[d] = 1, -feedback
    return (1 - mix) * x + mix * lfilter([1], a, x, axis=0)


def flanger(x, rate_hz: float = 0.7, depth_ms: float = 2.5, base_ms: float = 3.0, mix: float = 0.5, fb: float = 0.4):
    """Short modulated delay mixed with the input; mono or stereo."""
    n = len(x)
    t = np.arange(n) / SR
    d = (base_ms + depth_ms * np.sin(2 * np.pi * rate_hz * t)) * SR / 1000
    idx = np.arange(n) - d
    i0 = np.clip(np.floor(idx).astype(int), 0, n - 1)
    fr = (idx - np.floor(idx)).clip(0, 1)
    if x.ndim == 2:
        fr = fr[:, None]
    wet = x[i0] * (1 - fr) + x[np.clip(i0 + 1, 0, n - 1)] * fr
    return x + mix * (wet + fb * wet[np.clip(np.arange(n) - int(SR * 0.004), 0, n - 1)])
