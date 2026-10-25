"""Granular textures: scatter short windowed grains of a source signal into a cloud."""
from __future__ import annotations

import numpy as np

from .core import SR, n_of, rng, pan


def cloud(source: np.ndarray, dur: float, density: float = 80.0, grain_ms: float = 40.0, pitch_spread: float = 0.0,
          pos_curve=None, gain_curve=None, width: float = 0.8, seed=0) -> np.ndarray:
    """Stereo cloud from mono `source`. `density` grains/s; `pitch_spread` in semitones (random +/-); `pos_curve` is
    the 0..1 read position over time (default random); `width` spreads grains across the stereo field.
    """
    r = rng("grain", seed)
    n = n_of(dur)
    out = np.zeros((n, 2))
    g_len = int(SR * grain_ms / 1000)
    win = np.hanning(g_len)
    count = int(dur * density)
    for i in range(count):
        at = int(r.random() * (n - 1)) if gain_curve is None else int(np.clip(r.random(), 0, 1) * (n - 1))
        gain = 1.0 if gain_curve is None else float(gain_curve[min(at, len(gain_curve) - 1)])
        if gain < 1e-4:
            continue
        rate = 2 ** (r.uniform(-pitch_spread, pitch_spread) / 12)
        p = r.random() if pos_curve is None else float(np.clip(pos_curve[min(at, len(pos_curve) - 1)] + r.normal(0, 0.02), 0, 1))
        start = int(p * max(1, len(source) - g_len * 2))
        idx = start + np.arange(g_len) * rate
        seg = np.interp(idx, np.arange(len(source)), source) * win * gain
        g = pan(seg, r.uniform(-width, width))
        end = min(n, at + g_len)
        out[at:end] += g[: end - at]
    return out / np.sqrt(max(1.0, density * grain_ms / 1000))
