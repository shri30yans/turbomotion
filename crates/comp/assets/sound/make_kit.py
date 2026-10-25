"""Write the five sound-kit cues as mono 44.1 kHz WAVs. Stdlib only. Distinct timbres:

card-in  soft whoosh + muted wood tick
stamp    heavy low impact with grit
counter  bright metallic ping
loss     detuned low brass, beating
reveal   rising shimmer

Motion cues (transitions and UI moves) live in motion_cues.py.

Run from this folder: python3 make_kit.py
"""
from __future__ import annotations

import math
import os
import struct
import wave

SR = 44100
DIR = os.path.dirname(os.path.abspath(__file__))


def write(name: str, samples: list[float]) -> None:
    peak = max((abs(x) for x in samples), default=1.0) or 1.0
    gain = 0.86 / peak
    pcm = b"".join(struct.pack("<h", max(-32767, min(32767, int(x * gain * 32767)))) for x in samples)
    path = os.path.join(DIR, f"{name}.wav")
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(pcm)
    print(f"{name}.wav  {len(samples) / SR:.2f}s")


def env(t: float, attack: float, hold: float, release: float) -> float:
    if t < 0:
        return 0.0
    if t < attack:
        return t / attack
    if t < attack + hold:
        return 1.0
    u = (t - attack - hold) / max(release, 1e-6)
    return max(0.0, 1.0 - u)


def noise(i: int, seed: int) -> float:
    x = (i * 1103515245 + seed) & 0x7FFFFFFF
    return (x / 0x7FFFFFFF) * 2.0 - 1.0


def card_in() -> list[float]:
    n = int(SR * 0.32)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        whoosh = noise(i, 3) * env(t, 0.02, 0.04, 0.22)
        # band-ish: mix with a delayed copy
        if i > 8:
            whoosh = (whoosh + out[i - 8] * 0.35) * 0.55
        tick = math.sin(2 * math.pi * 210 * t) * math.exp(-t * 38) * env(t - 0.11, 0.002, 0.01, 0.08)
        out[i] = whoosh * 0.45 + tick * 0.7
    return out


def stamp() -> list[float]:
    n = int(SR * 0.38)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        f = 78 * math.exp(-t * 9) + 36
        body = math.sin(2 * math.pi * f * t) * math.exp(-t * 7)
        grit = noise(i, 11) * math.exp(-t * 28) * 0.55
        click = math.sin(2 * math.pi * 920 * t) * math.exp(-t * 55) * 0.25
        out[i] = body * 0.95 + grit + click
    return out


def counter() -> list[float]:
    n = int(SR * 0.14)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        a = math.sin(2 * math.pi * 2480 * t) * math.exp(-t * 22)
        b = math.sin(2 * math.pi * 4960 * t) * math.exp(-t * 30) * 0.35
        out[i] = (a + b) * env(t, 0.001, 0.01, 0.12)
    return out


def loss() -> list[float]:
    n = int(SR * 0.62)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        a = math.sin(2 * math.pi * 98 * t)
        b = math.sin(2 * math.pi * 104.5 * t)
        c = math.sin(2 * math.pi * 49 * t) * 0.4
        out[i] = (a + b + c) * 0.5 * env(t, 0.02, 0.12, 0.46)
    return out


def reveal() -> list[float]:
    n = int(SR * 0.55)
    out = [0.0] * n
    for i in range(n):
        t = i / SR
        # rising high-passed noise
        hp = noise(i, 21) - (noise(i - 1, 21) if i else 0.0)
        sweep = hp * (0.15 + 0.85 * (t / 0.55)) * env(t, 0.04, 0.2, 0.28)
        bell = math.sin(2 * math.pi * 523 * t) * math.exp(-t * 4.5) * 0.28
        sparkle = math.sin(2 * math.pi * 2093 * t + 0.4) * math.exp(-t * 8) * 0.12
        out[i] = sweep * 0.65 + bell + sparkle
    return out


if __name__ == "__main__":
    write("card-in", card_in())
    write("stamp", stamp())
    write("counter", counter())
    write("loss", loss())
    write("reveal", reveal())
    from motion_cues import CUES
    for name, make in CUES.items():
        write(name, make())
