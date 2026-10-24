"""Generates showcase/music.wav: a 32-second, 120 bpm track made from scratch (kick, hats, bass, pad). No samples, no
licence questions. Run once: python3 make_audio.py"""
import math, wave
from pathlib import Path
import numpy as np

sr, dur, bpm = 44100, 32.0, 120
n = int(sr * dur)
t = np.arange(n) / sr
beat = 60 / bpm
out = np.zeros(n)

def add(sig, at):
    i = int(at * sr)
    if i < n:
        out[i:i + len(sig)] += sig[: n - i]

def kick():
    s = np.arange(int(sr * 0.45)) / sr
    return np.sin(2 * math.pi * (45 + 110 * np.exp(-s * 38)) * s) * np.exp(-s * 7) * 0.9

def hat():
    s = np.arange(int(sr * 0.06)) / sr
    return np.random.default_rng(3).normal(0, 1, len(s)) * np.exp(-s * 70) * 0.12

def bass(f):
    s = np.arange(int(sr * beat * 1.9)) / sr
    return (np.sin(2 * math.pi * f * s) + 0.35 * np.sin(2 * math.pi * 2 * f * s)) * np.exp(-s * 2.2) * 0.32

roots = [55.0, 55.0, 43.65, 49.0]  # A1 A1 F1 G1, one chord per bar
for b in range(int(dur / beat)):
    bar, in_bar = divmod(b, 4)
    energy = 0.0 if bar < 1 else 1.0
    if bar >= 1:
        add(kick(), b * beat)
        add(hat(), b * beat + beat / 2)
    if in_bar in (0, 2) and bar >= 1:
        add(bass(roots[bar % 4]), b * beat)
for bar in range(int(dur / (4 * beat))):
    f = roots[bar % 4] * 4
    s = np.arange(int(sr * 4 * beat)) / sr
    env = np.minimum(1, s * 3) * np.minimum(1, (4 * beat - s) * 3)
    pad = sum(np.sin(2 * math.pi * f * r * s + k) for k, r in enumerate((1.0, 1.2599, 1.4983))) * env * 0.05
    add(pad, bar * 4 * beat)
out *= np.minimum(1, t / 0.5) * np.minimum(1, (dur - t) / 2.0)
out = np.tanh(out * 1.2)
pcm = (np.clip(out, -1, 1) * 32767 * 0.85).astype(np.int16)
with wave.open(str(Path(__file__).with_name("music.wav")), "wb") as w:
    w.setnchannels(1); w.setsampwidth(2); w.setframerate(sr); w.writeframes(pcm.tobytes())
print("music.wav", dur, "s")
