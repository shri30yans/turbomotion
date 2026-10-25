"""Pitched one-shots for melodies. Rendered at every third semitone over two octaves (C, D#, F#, A); the engine
pitches the nearest root by at most one semitone for the notes between."""
from __future__ import annotations

import numpy as np

from . import fx, noise
from .core import SR, Sound, hz, midi, n_of, note_name, put, t_axis
from .master import finish
from .env import curve, exp_decay
from .fam_hit import thump
from .filt import lowpass, sweep
from .ks import pluck as ks_pluck
from .modal import contact, strike
from .osc import fm, saw, sine, stack

ROOTS = (0, 3, 6, 9)   # semitones above C


def roots(lo: str, hi: str) -> list[int]:
    a, b = midi(lo), midi(hi)
    return [m for m in range(a, b + 1) if (m - a) % 3 == 0]


def p_pluck(m):
    f = hz(m)
    return ks_pluck(f, 1.4, .9965 - .0004 * (m - 48) / 12, .55, .18, "pl")


def p_bell(m):
    f = hz(m)
    d = 2.0
    t = t_axis(d)
    x = strike(f, "bell", d, .45, "pb", 1.0 * (1 - .07 * (m - 60) / 12))
    return x + fm(f * 2, 3.5, 1.5 * np.exp(-t / .05), d) * exp_decay(d, .4, .001) * .25


def p_marimba(m):
    f = hz(m)
    d = .9
    x = strike(f, "bar", d, .5, "pm", .55 * (1 - .12 * (m - 48) / 12))
    put(x, contact(.01, 1500 + f, "pm"), 0, .5)
    return x


def p_stab(m):
    f = hz(m)
    d = .5
    x = stack(saw, f, d, 5, 14, ("st", m))
    cut = curve([(0, f * 14), (.25, f * 3), (1, f * 2)], d, log=True)
    return fx.width(sweep(x, "lowpass", cut, 2.2) * exp_decay(d, .16, .004), 1.3)


def p_bass(m):
    f = hz(m)
    d = .8
    t = t_axis(d)
    x = sine(f * (1 + .06 * np.exp(-t / .03)), d) + .55 * sweep(saw(f, d), "lowpass", curve([(0, f * 9), (.3, f * 2.5), (1, f * 2)], d, log=True), 2.5)
    return fx.saturate(x * exp_decay(d, .42, .004), 1.6)


VOICES = {
    "pluck": ("Karplus-Strong plucked string", "C3", "C5", p_pluck),
    "bell": ("inharmonic struck bell with an FM glint", "C4", "C6", p_bell),
    "marimba": ("tuned wooden bar", "C3", "C5", p_marimba),
    "stab": ("detuned saw stab with a closing filter", "C3", "C5", p_stab),
    "bass": ("round sub bass with a saw edge", "C2", "C4", p_bass),
}


def voice(name: str) -> dict[str, Sound]:
    desc, lo, hi, fn = VOICES[name]
    return {note_name(m): Sound(finish(fn(m), 2, 10), f"{desc}, root {note_name(m)}", (name, "pitched")) for m in roots(lo, hi)}


REGISTRY = {name: (v[0] + " (pitched: use pluck/C4 ... notes)", (0.0, 0.5), (lambda n=name: voice(n))) for name, v in VOICES.items()}
PITCHED = {name: (midi(v[1]), midi(v[2])) for name, v in VOICES.items()}
