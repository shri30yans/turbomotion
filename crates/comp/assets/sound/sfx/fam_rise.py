"""Risers, downers and swells: sounds that build to, fall from, or bloom around a moment."""
from __future__ import annotations

import numpy as np

from . import fx, noise
from .core import SR, Sound, hz, n_of, rng, reverse, stereo, t_axis
from .master import finish
from .env import curve, lfo, ramp_up, swell
from .fam_air import air
from .filt import filt, highpass, lowpass, sweep
from .osc import fm, saw, sine, square, stack


def shepard(base, dur, up=True, voices=9, seed=0):
    """Shepard-Risset glissando: octave-spaced sines under a fixed bell over log-frequency, gliding one octave."""
    n = n_of(dur)
    t = np.arange(n) / SR
    out = np.zeros(n)
    for v in range(voices):
        pos = (v + (t / dur if up else 1 - t / dur))          # octaves above base
        f = base * 2.0 ** pos
        amp = np.exp(-0.5 * ((pos - voices / 2) / (voices / 5)) ** 2)
        out += amp * np.sin(2 * np.pi * np.cumsum(f) / SR + v)
    return out / voices * 2


def riser():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x, 3, 8), d, t)
    d = 2.0
    add("noise-01", air(d, d - .08, [(0, 300), (1, 9000)], [(0, 2), (1, .7)], "white", 70, move=.3, rise=2.4, fall=8) , "white noise opening upward", ("noise", "classic"))
    d = 2.4
    x = highpass(noise.pink(d, 71), 200) * lfo(curve([(0, 4), (1, 30)], d), d, .8)
    x = sweep(x, "highpass", curve([(0, 200), (1, 6000)], d, log=True), .9) * ramp_up(d, 2, .05)
    add("noise-02", fx.decorrelate(x * 2, .5, 1), "noise with an accelerating tremolo", ("noise", "pulse"))
    d = 1.5
    f = curve([(0, 400), (1, 6500)], d, log=True)
    x = (sweep(noise.white(d, 72), "bandpass", f, 9) + .6 * sweep(noise.white(d, 73), "bandpass", f * 2, 9)) * ramp_up(d, 2.5, .04)
    add("noise-03", fx.decorrelate(x, .5, 2), "narrow whistling band climbing two octaves", ("noise", "whistle"))
    d = 2.2
    f = curve([(0, 110), (1, 440)], d, log=True)
    x = sweep(stack(saw, f, d, 6, 20, 74), "lowpass", curve([(0, 250), (1, 7000)], d, log=True), 1.6) * ramp_up(d, 1.8, .05)
    add("tonal-01", fx.decorrelate(x, .5, 3), "detuned saw stack rising two octaves", ("tonal", "synth"))
    d = 1.8
    f = curve([(0, 200), (1, 800)], d, log=True)
    x = (square(f, d, .35) + .7 * square(f * 1.5, d, .5)) * .5
    add("tonal-02", fx.width(sweep(x, "lowpass", curve([(0, 400), (1, 5000)], d, log=True), 1.2) * ramp_up(d, 2, .04), 1.4), "square and fifth sliding up", ("tonal", "hollow"))
    d = 2.6
    x = fm(curve([(0, 55), (1, 880)], d, log=True), 2.0, curve([(0, .3), (1, 7)], d), d) * ramp_up(d, 2.2, .06)
    add("tonal-03", fx.reverb(x * .9, .8, .5, .2, tail=.3), "cinematic FM growl climbing to a scream", ("tonal", "cinematic"))
    add("shepard-01", fx.decorrelate(shepard(40, 3.0, True, seed=1) * ramp_up(3.0, 1.0, .08), .4), "endless rising Shepard-Risset tone", ("shepard", "endless"))
    add("shepard-02", fx.decorrelate(shepard(55, 2.0, True, 8, 2) * ramp_up(2.0, 1.6, .06), .4, 2), "quicker Shepard rise", ("shepard", "quick"))
    d = 2.0
    ts = np.cumsum(np.linspace(.25, .02, 40))
    ts = ts[ts < d - .08]
    x = np.zeros(n_of(d))
    for k, t0 in enumerate(ts):
        c = highpass(noise.white(.03, k), 3000) * np.exp(-t_axis(.03) / .006) * (.3 + .7 * t0 / d)
        i = int(t0 * SR); x[i : i + len(c)] += c[: len(x) - i]
    x += sweep(noise.pink(d, 75), "bandpass", curve([(0, 400), (1, 5000)], d, log=True), 2) * ramp_up(d, 2.5, .05) * .7
    add("pulse-01", fx.decorrelate(x, .4), "accelerating ticks over rising noise", ("pulse", "tension"))
    d = 1.6
    ts = np.cumsum(np.linspace(.12, .012, 80)); ts = ts[ts < d - .05]
    x = np.zeros(n_of(d))
    for k, t0 in enumerate(ts):
        c = filt(noise.white(.05, 100 + k), "highpass", 1500) * np.exp(-t_axis(.05) / .012) * (.3 + .7 * t0 / d)
        i = int(t0 * SR); x[i : i + len(c)] += c[: len(x) - i]
    add("pulse-02", fx.decorrelate(x * 1.5, .3, 5), "snare-roll style burst rate climbing", ("pulse", "roll"))
    for k, (d, kind) in enumerate([(.8, "noise"), (1.0, "tonal"), (.9, "shepard")], 1):
        if kind == "noise":
            x = fx.decorrelate(sweep(noise.white(d, 110), "bandpass", curve([(0, 300), (1, 8000)], d, log=True), 2.5) * ramp_up(d, 2, .03), .5)
        elif kind == "tonal":
            x = sweep(stack(saw, curve([(0, 200), (1, 800)], d, log=True), d, 4, 16, 111), "lowpass", curve([(0, 400), (1, 6000)], d, log=True), 1.5) * ramp_up(d, 1.6, .03)
        else:
            x = shepard(60, d, True, 8, 4) * ramp_up(d, 1.2, .03)
        add(f"quick-0{k}", fx.decorrelate(x, .5, k), "short build for a transition: " + kind, ("quick", kind))
    d = 2.8
    f = curve([(0, 38), (1, 76)], d, log=True)
    x = lowpass(saw(f, d), 600) * ramp_up(d, 2, .05) + highpass(noise.pink(d, 76), 2500) * ramp_up(d, 3.0, .05) * .5
    add("drone-01", fx.reverb(x, .6, .5, .15, tail=.2), "low drone rising an octave with a hiss on top", ("drone", "low"))
    return S


def downer():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x, 3, 10), d, t)
    d = .9
    add("drop-01", fx.saturate(sine(curve([(0, 800), (1, 35)], d, log=True), d) * swell(d, .004, 1, 1.0), 2.5), "sine plunging nine octaves", ("sine", "plunge"))
    d = 1.2
    add("drop-02", fx.width(sweep(saw(curve([(0, 400), (1, 50)], d, log=True), d), "lowpass", curve([(0, 3000), (1, 90)], d, log=True), 1.5) * np.exp(-t_axis(d) / .8), 1.4), "saw falling into the floor", ("saw", "fall"))
    d = 1.0
    add("drop-03", fx.decorrelate(fm(curve([(0, 1200), (1, 60)], d, log=True), 1.0, curve([(0, 6), (1, .2)], d), d) * np.exp(-t_axis(d) / .5), .4), "FM bleep falling and calming", ("fm", "fall"))
    d = 1.2
    add("noise-01", air(d, .02, [(0, 8000), (1, 200)], 3, "white", 80, rise=1, fall=2.2, move=-.5), "bright noise sliding down", ("noise", "slide"))
    d = 1.6
    x = sweep(noise.pink(d, 81), "lowpass", curve([(0, 5000), (1, 120)], d, log=True), 1.2) * np.exp(-t_axis(d) / .7)
    add("noise-02", fx.width(x * 1.5, 1.3), "noise filter closing into a rumble", ("noise", "close"))
    add("shepard-01", fx.decorrelate(shepard(40, 2.2, False, seed=3) * np.exp(-t_axis(2.2) / 1.2), .4), "Shepard tone sinking forever", ("shepard", "sink"))
    d = 1.4
    t = t_axis(d)
    rate = np.maximum(1 - (t / d) ** 1.5, .02)
    ph = np.cumsum(rate) / SR
    x = sum(np.sin(2 * np.pi * hz(m) * ph * SR / 1) * .3 for m in (48, 51, 55, 60))
    x = sweep(x + .3 * np.sign(x), "lowpass", 3000 * rate + 100, 1.0) * np.clip(rate * 4, 0, 1)
    add("tapestop-01", fx.width(x, 1.2), "a chord on a tape that is switched off", ("tape", "stop"))
    return S


def swell_():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x, 8, 15), d, t)
    def chord(ms, d, pk, cut0=250, cut1=3500, vib=0, seed=0):
        out = 0
        for k, m in enumerate(ms):
            f = hz(m) * (1 + vib * .01 * np.sin(2 * np.pi * (5.2 + k * .3) * t_axis(d)))
            out = out + stack(saw, f, d, 4, 14, (seed, k))
        return sweep(out / len(ms), "lowpass", curve([(0, cut0), (pk / d, cut1), (1, cut0)], d, log=True), 1.0) * swell(d, pk, 2.2, 1.6)
    add("pad-cm", fx.reverb(fx.width(chord([48, 55, 63, 67], 2.6, 1.4), 1.5), 1.4, .5, .3), "C minor pad blooming and fading", ("pad", "cmin"))
    add("pad-fm", fx.reverb(fx.width(chord([41, 57, 60, 65], 2.4, 1.2, 300, 4000, seed=1), 1.5), 1.4, .5, .3), "F major pad", ("pad", "fmaj"))
    add("strings-01", fx.reverb(fx.width(chord([55, 62, 67, 70, 74], 2.2, 1.2, 600, 6000, .8, 2), 1.4), 1.0, .4, .25), "bowed-string swell with vibrato (C minor 7)", ("strings", "cmin"))
    d = 2.4; t = t_axis(d); out = 0
    for k, m in enumerate([48, 55, 60, 63]):
        src = stack(saw, hz(m) * (1 + .006 * np.sin(2 * np.pi * 5.5 * t + k)), d, 3, 10, (7, k))
        out = out + sum(sweep(src, "bandpass", f0, 8) * a for f0, a in ((750, 1), (1150, .6), (2800, .25)))
    add("choir-01", fx.reverb(fx.width(out * swell(d, 1.2, 2, 1.6), 1.5), 1.6, .5, .3), "vowel-filtered 'ah' choir on C minor", ("choir", "vocal"))
    d = 1.8
    x = highpass(noise.white(d, 90), 3500) * lfo(curve([(0, 5), (1, 12)], d), d, .35) * ramp_up(d, 2.2, .02) + highpass(noise.white(d, 91), 7000) * ramp_up(d, 1.6, .02) * .6
    add("cymbal-01", fx.decorrelate(x, .5), "reverse cymbal", ("cymbal", "reverse"))
    d = 2.4
    x = sweep(noise.white(d, 92), "bandpass", curve([(0, 2000), (1, 9000)], d, log=True), .6) * ramp_up(d, 1.7, .03)
    add("cymbal-02", fx.decorrelate(x * 1.5, .5, 2), "slower swelling noise wash", ("cymbal", "wash"))
    return S


REGISTRY = {
    "riser": ("builds to a moment: noise, tonal, Shepard, pulses", (0.0, 1.0), riser),
    "downer": ("falls away: plunges, closing filters, tape stop", (0.0, 1.0), downer),
    "swell": ("pads, strings, choir and cymbal blooms", (0.0, 1.0), swell_),
}
