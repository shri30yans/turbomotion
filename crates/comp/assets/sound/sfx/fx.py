"""Effects: saturation, wavefold, bitcrush, delay, Schroeder reverb with stereo decorrelation, width, doubling."""
from __future__ import annotations

import numpy as np
from scipy.signal import lfilter

from .core import SR, fit, mono, stereo, rng


def saturate(x, drive: float = 3.0):
    return np.tanh(x * drive) / np.tanh(drive)


def fold(x, amount: float = 2.0):
    return np.sin(x * amount * np.pi / 2)


def bitcrush(x, bits: int = 8, hold: int = 1):
    y = np.round(x * (2 ** (bits - 1))) / (2 ** (bits - 1))
    if hold > 1:
        y = np.repeat(y[::hold], hold, axis=0)[: len(x)]
    return y


def delay(x, ms: float, feedback: float = 0.35, mix: float = 0.3, tail: float = 0.5):
    """Feedback echo: the dry signal plus repeats every `ms`, until they fall 60 dB (or `tail` seconds)."""
    x = stereo(x)
    d = int(SR * ms / 1000)
    out = np.concatenate([x, np.zeros((int(tail * SR) + d, 2))])
    wet = np.zeros_like(out)
    k, g = 1, feedback
    while g > 1e-3 and k * d < len(out) - len(x):
        wet[k * d : k * d + len(x)] += x * g
        k, g = k + 1, g * feedback
    return out * (1 - mix * 0.5) + wet * mix


_COMBS = [(1116, 1188), (1277, 1356), (1422, 1491), (1557, 1617), (1687, 1760), (1789, 1861)]
_ALLP = [(556, 579), (441, 463), (341, 373)]


def _comb(x, d, g, damp):
    """y[n] = x[n] + g((1-damp) y[n-d] + damp y[n-d-1]), computed one delay-length block at a time."""
    y = np.concatenate([np.zeros(d + 1), x.astype(float)])
    for s in range(d + 1, len(y), d):
        e = min(len(y), s + d)
        y[s:e] += g * ((1 - damp) * y[s - d : e - d] + damp * y[s - d - 1 : e - d - 1])
    return y[d + 1 :]


def _allpass(x, d, g=0.5):
    """y[n] = -g x[n] + x[n-d] + g y[n-d]."""
    xp = np.concatenate([np.zeros(d), x])
    y = np.zeros(len(xp))
    for s in range(d, len(xp), d):
        e = min(len(xp), s + d)
        y[s:e] = -g * xp[s:e] + xp[s - d : e - d] + g * y[s - d : e - d]
    return y[d:]


def reverb(x, decay: float = 1.5, damp: float = 0.35, mix: float = 0.3, pre_ms: float = 0.0, size: float = 1.0, tail: float | None = None):
    """Stereo Schroeder reverb: six damped combs then three allpasses per ear, different lengths per ear.

    `decay` is the RT60 in seconds; the output is longer than the input by `tail` (default `decay`) seconds.
    """
    src = mono(x)
    tail = decay if tail is None else tail
    src = np.concatenate([np.zeros(int(SR * pre_ms / 1000)), src, np.zeros(int(SR * tail))])
    outs = []
    for side in (0, 1):
        acc = np.zeros(len(src))
        for lens in _COMBS:
            d = max(8, int(lens[side] * size))
            acc += _comb(src, d, 10 ** (-3 * d / SR / decay), damp)
        acc /= len(_COMBS)
        for lens in _ALLP:
            acc = _allpass(acc, max(4, int(lens[side] * size)))
        outs.append(acc)
    wet = np.stack(outs, 1)
    dry = fit(stereo(x), len(wet))
    return dry * (1 - mix * 0.5) + wet * mix * 1.6


def width(x, w: float = 1.3):
    """Mid/side width: 1 unchanged, 0 mono, >1 wider."""
    x = stereo(x)
    m, s = (x[:, 0] + x[:, 1]) / 2, (x[:, 0] - x[:, 1]) / 2
    return np.stack([m + s * w, m - s * w], 1)


def decorrelate(x, amount: float = 0.6, seed=0):
    """Make a mono signal wide by passing each ear through different allpass chains (no level change)."""
    r = rng("decor", seed)
    src = mono(x)
    chans = []
    for _ in range(2):
        y = src
        for _ in range(4):
            d = int(r.integers(7, 90))
            g = amount * 0.8
            b = np.zeros(d + 1)
            a = np.zeros(d + 1)
            a[0], a[d] = 1, -g
            b[0], b[d] = -g, 1
            y = lfilter(b, a, y)
        chans.append(y)
    return np.stack(chans, 1)


def haas(x, ms: float = 12.0):
    """Delay the right ear a little for a sense of space."""
    x = stereo(x)
    d = int(SR * ms / 1000)
    return np.stack([x[:, 0], np.concatenate([np.zeros(d), x[: len(x) - d, 1]])], 1)
