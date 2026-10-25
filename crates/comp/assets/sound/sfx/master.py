"""Mastering: loudness, level, DC removal, tail trimming and the final fades applied to every rendered sound."""
from __future__ import annotations

import numpy as np
from scipy.signal import butter, lfilter, sosfiltfilt

from .core import SR, db, fade, mono, to_db


def dc_block(x: np.ndarray, fc: float = 18.0) -> np.ndarray:
    sos = butter(2, fc, "high", fs=SR, output="sos")
    return sosfiltfilt(sos, x, axis=0)


def trim_tail(x: np.ndarray, floor_db: float = -60.0, keep_ms: float = 20.0) -> np.ndarray:
    """Drop trailing near-silence (relative to the peak)."""
    a = np.abs(x).max(1) if x.ndim == 2 else np.abs(x)
    idx = np.nonzero(a > a.max() * db(floor_db))[0]
    end = (idx[-1] + int(SR * keep_ms / 1000)) if len(idx) else len(x)
    return x[: min(len(x), end)]


def envelope(x: np.ndarray, win_ms: float = 10.0) -> np.ndarray:
    """RMS envelope, one value per window hop (half a window)."""
    m = mono(x)
    w = max(8, int(SR * win_ms / 1000))
    hop = w // 2
    k = (len(m) - w) // hop + 1
    if k < 1:
        return np.array([np.sqrt(np.mean(m**2))])
    idx = np.arange(w)[None, :] + hop * np.arange(k)[:, None]
    return np.sqrt((m[idx] ** 2).mean(1))


def peak_time(x: np.ndarray) -> float:
    """Seconds to the loudest 10 ms of the sound: where it should land on a transition's midpoint."""
    e = envelope(x, 10.0)
    return float(np.argmax(e) * (SR * 0.005) / SR)


def _weigh(x: np.ndarray) -> np.ndarray:
    """Crude loudness weighting: ears are deaf to deep bass, so a first-order highpass at 250 Hz."""
    a = np.exp(-2 * np.pi * 250 / SR)
    return lfilter([(1 + a) / 2, -(1 + a) / 2], [1, -a], x, axis=0)


def level(x: np.ndarray, peak_db: float = -1.0, rms_ceiling_db: float = -17.0) -> np.ndarray:
    """Peak at -1 dBFS, but sustained loud sounds are pulled down so their active RMS stays under a common ceiling.

    Click-like sounds end at the peak target; noisy sustained ones end lower, so families sit at similar loudness.
    """
    x = dc_block(x)
    pk = np.abs(x).max()
    if pk < 1e-9:
        return x
    g = db(peak_db) / pk
    e = envelope(_weigh(x * g), 20.0)
    active = e[e > e.max() * db(-30)]
    rms = np.sqrt(np.mean(active**2)) if len(active) else 0.0
    if rms > 0 and to_db(rms) > rms_ceiling_db:
        g *= db(rms_ceiling_db) / rms
    return x * g


def finish(x: np.ndarray, in_ms: float = 3, out_ms: float = 6, trim_db: float = -42.0) -> np.ndarray:
    """Trim the dead tail, level, remove the mean, fade the edges (so a file starts and ends at exactly zero)."""
    x = level(trim_tail(x, trim_db))
    x = x - x.mean(0)
    return fade(x, in_ms, out_ms)
