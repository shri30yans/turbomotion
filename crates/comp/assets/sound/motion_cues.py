"""Motion-design cues: the sounds of transitions and UI moves. Each returns mono samples at 44.1 kHz; `make_kit.py`
writes them. Peaks are placed where `timeline/catalogue.rs::cue_peak` expects them, so a transition's sound lands
on its midpoint.

whoosh  air past the lens, peak 0.28 s     swish  a fast whip, peak 0.10 s     sweep  a long pass, peak 0.55 s
riser   a build into a hit, peak 0.85 s    pop    a soft bubble pop           click  a UI click
tick    a dry clock tick                   glitch digital bursts, peak 0.06   boom   a sub drop for big reveals
type    one keystroke
"""
from __future__ import annotations

import math

SR = 44100


def _noise(i: int, seed: int) -> float:
    x = (i * 1103515245 + seed * 12345 + 1) & 0x7FFFFFFF
    x = (x * 1103515245 + 12345) & 0x7FFFFFFF
    return (x / 0x7FFFFFFF) * 2.0 - 1.0


def _swell(t: float, peak: float, length: float) -> float:
    """0 → 1 at `peak` → 0 at `length`, rising on a square and falling on a square root."""
    if t <= 0 or t >= length:
        return 0.0
    return (t / peak) ** 2 if t < peak else (1 - (t - peak) / (length - peak)) ** 1.5


def _air(length: float, peak: float, seed: int, bright: float) -> list[float]:
    """Noise through a one-pole low-pass whose cutoff follows the swell: dull at the edges, bright at the peak."""
    n = int(SR * length)
    out, lp, lp2 = [0.0] * n, 0.0, 0.0
    for i in range(n):
        t = i / SR
        e = _swell(t, peak, length)
        k = 0.02 + bright * e
        lp += k * (_noise(i, seed) - lp)
        lp2 += k * (lp - lp2)
        out[i] = lp2 * e
    return out


def whoosh() -> list[float]:
    return _air(0.62, 0.28, 5, 0.35)


def swish() -> list[float]:
    return _air(0.3, 0.1, 9, 0.6)


def sweep() -> list[float]:
    a, b = _air(1.15, 0.55, 13, 0.22), _air(1.15, 0.6, 17, 0.3)
    return [x * 0.6 + y * 0.5 for x, y in zip(a, b)]


def riser() -> list[float]:
    n = int(SR * 1.2)
    out = [0.0] * n
    ph = 0.0
    for i in range(n):
        t = i / SR
        build = min(1.0, t / 0.85) ** 2.2 if t < 0.85 else 0.0
        ph += 2 * math.pi * (180 + 900 * min(t, 0.85) ** 2) / SR
        tone = math.sin(ph) * build * 0.35
        hiss = _noise(i, 23) * build * 0.25
        hit = 0.0
        if t >= 0.85:
            u = t - 0.85
            hit = math.sin(2 * math.pi * (60 + 40 * math.exp(-u * 30)) * u) * math.exp(-u * 9) + _noise(i, 29) * math.exp(-u * 40) * 0.5
        out[i] = tone + hiss + hit
    return out


def pop() -> list[float]:
    n = int(SR * 0.16)
    return [math.sin(2 * math.pi * (900 * math.exp(-i / SR * 30) + 260) * i / SR) * math.exp(-i / SR * 34) for i in range(n)]


def click() -> list[float]:
    n = int(SR * 0.05)
    return [(_noise(i, 31) * 0.5 + math.sin(2 * math.pi * 3200 * i / SR)) * math.exp(-i / SR * 160) for i in range(n)]


def tick() -> list[float]:
    n = int(SR * 0.06)
    return [(math.sin(2 * math.pi * 1800 * i / SR) * 0.7 + _noise(i, 37) * 0.3) * math.exp(-i / SR * 120) for i in range(n)]


def glitch() -> list[float]:
    n = int(SR * 0.36)
    out = [0.0] * n
    hold = 0.0
    for i in range(n):
        t = i / SR
        burst = 1.0 if int(t * 34) % 3 != 1 else 0.0
        if i % (6 + int(t * 50) % 20) == 0:
            hold = _noise(i, 41)  # sample-and-hold: crushed, stepped noise
        out[i] = (hold * 0.7 + math.copysign(0.3, math.sin(2 * math.pi * 120 * t))) * burst * math.exp(-max(0.0, t - 0.06) * 7)
    return out


def boom() -> list[float]:
    n = int(SR * 1.4)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        f = 38 + 70 * math.exp(-t * 14)
        out[i] = math.sin(2 * math.pi * f * t) * math.exp(-t * 2.6) + _noise(i, 43) * math.exp(-t * 60) * 0.4
    return out


def keystroke() -> list[float]:
    n = int(SR * 0.045)
    return [(_noise(i, 47) * 0.6 + math.sin(2 * math.pi * 2400 * i / SR) * 0.4) * math.exp(-i / SR * 140) for i in range(n)]


CUES = {'whoosh': whoosh, 'swish': swish, 'sweep': sweep, 'riser': riser, 'pop': pop, 'click': click, 'tick': tick,
        'glitch': glitch, 'boom': boom, 'type': keystroke}
