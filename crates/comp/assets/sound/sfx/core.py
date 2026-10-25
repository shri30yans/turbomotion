"""Shared basics: sample rate, seeds, stereo helpers, fades, mixing, normalisation, WAV output.

A signal is a float64 numpy array: mono `(n,)` or stereo `(n, 2)`. Everything here is deterministic given a seed.
"""
from __future__ import annotations

import zlib

import numpy as np
from scipy.io import wavfile
from scipy.signal import butter, sosfiltfilt

SR = 48_000


def seed_of(*parts) -> int:
    """A stable 32-bit seed from names/numbers (never Python's randomised hash)."""
    return zlib.crc32("/".join(map(str, parts)).encode())


def rng(*parts) -> np.random.Generator:
    return np.random.default_rng(seed_of(*parts))


def n_of(dur: float) -> int:
    return max(1, int(round(dur * SR)))


def t_axis(dur: float) -> np.ndarray:
    return np.arange(n_of(dur)) / SR


def db(x: float) -> float:
    return 10 ** (x / 20)


def to_db(x: float) -> float:
    return 20 * np.log10(max(x, 1e-12))


def stereo(x: np.ndarray) -> np.ndarray:
    return np.stack([x, x], 1) if x.ndim == 1 else x


def mono(x: np.ndarray) -> np.ndarray:
    return x.mean(1) if x.ndim == 2 else x


def pan(x: np.ndarray, p) -> np.ndarray:
    """Constant-power pan of a mono signal; `p` is -1..1, a number or a curve of the same length."""
    a = (np.asarray(p, dtype=float) + 1) * np.pi / 4
    return np.stack([x * np.cos(a), x * np.sin(a)], 1)


def fit(x: np.ndarray, n: int) -> np.ndarray:
    """Cut or zero-pad the first axis to `n` samples."""
    if len(x) >= n:
        return x[:n]
    return np.concatenate([x, np.zeros((n - len(x),) + x.shape[1:])])


def fade(x: np.ndarray, in_ms: float = 3, out_ms: float = 4) -> np.ndarray:
    """Short raised-cosine fades so a file never starts or ends on a step."""
    x = x.copy()
    for ms, tail in ((in_ms, False), (out_ms, True)):
        k = min(len(x) // 2, max(2, int(SR * ms / 1000)))
        ramp = 0.5 - 0.5 * np.cos(np.linspace(0, np.pi, k))
        ramp = ramp[::-1] if tail else ramp
        sl = slice(len(x) - k, None) if tail else slice(0, k)
        x[sl] *= ramp if x.ndim == 1 else ramp[:, None]
    return x


def reverse(x: np.ndarray) -> np.ndarray:
    return x[::-1].copy()


def mix(*parts, dur: float | None = None) -> np.ndarray:
    """Layer signals. Each part is `sig`, `(sig, at_seconds)` or `(sig, at_seconds, gain)`; result is stereo."""
    items = []
    for p in parts:
        sig, at, g = (p, 0.0, 1.0) if isinstance(p, np.ndarray) else (p[0], p[1], p[2] if len(p) > 2 else 1.0)
        items.append((stereo(sig) * g, int(round(at * SR))))
    n = max(i + len(s) for s, i in items) if dur is None else n_of(dur)
    out = np.zeros((n, 2))
    for s, i in items:
        seg = s[: max(0, n - i)]
        out[i : i + len(seg)] += seg
    return out


def write_wav(path, x: np.ndarray, as_mono: bool = False) -> None:
    x = mono(x) if as_mono else x
    wavfile.write(str(path), SR, (np.clip(x, -1, 1) * 32767).round().astype("<i2"))


class Sound:
    """One rendered variant: its samples and what it is for."""

    def __init__(self, x: np.ndarray, desc: str, tags=(), force_mono: bool = False):
        self.x, self.desc, self.tags, self.force_mono = x, desc, tuple(tags), force_mono


_PC = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}


def midi(note) -> int:
    """`60`, `"C4"`, `"F#3"`, `"Bb2"` -> MIDI number (C4 = 60 = 261.63 Hz)."""
    if isinstance(note, (int, float, np.integer, np.floating)):
        return int(note)
    s = note.strip()
    pc = _PC[s[0].upper()] + s[1:-1].count("#") - s[1:-1].count("b")
    return pc + 12 * (int(s[-1]) + 1)


def hz(note) -> float:
    return 440.0 * 2 ** ((midi(note) - 69) / 12)


def note_name(m: int) -> str:
    return ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"][m % 12] + str(m // 12 - 1)


def put(buf: np.ndarray, sig: np.ndarray, at: float = 0.0, gain: float = 1.0) -> np.ndarray:
    """Add `sig` into `buf` at `at` seconds (in place, clipped to the buffer; a mono sig is spread over both ears)."""
    i = int(round(at * SR))
    if i >= len(buf) or i < 0:
        return buf
    seg = sig[: len(buf) - i]
    if buf.ndim == 2 and seg.ndim == 1:
        seg = seg[:, None]
    buf[i : i + len(seg)] += seg * gain
    return buf
