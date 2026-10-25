"""Air families: whoosh, swish, sweep. Noise and tones moved through swept filters, with stereo movement."""
from __future__ import annotations

import numpy as np

from . import fx, noise
from .core import Sound, n_of, pan, rng, reverse, stereo, t_axis
from .master import finish
from .env import curve, swell
from .filt import bandpass, filt, flanger, comb, highpass, lowpass, sweep
from .osc import fm, saw, sine, stack


def air(dur, peak, pts, q, colour="pink", seed=0, kind="bandpass", move=0.0, rise=2.0, fall=1.6, width=0.7):
    """Two independent noise channels through a swept filter under a swell; `move` pans the image (-1 left .. 1 right)."""
    f = curve(pts, dur, log=True)
    qq = np.full(n_of(dur), q) if np.isscalar(q) else curve(q, dur)
    env = swell(dur, peak, rise, fall)
    ch = [sweep(noise.colour(colour, dur, (seed, side)), kind, f * (1 + 0.03 * side), qq) * env for side in (0, 1)]
    mid = (ch[0] + ch[1]) / 2
    wide = np.stack([mid + (ch[0] - mid) * width * 2, mid + (ch[1] - mid) * width * 2], 1)
    if move:
        p = np.linspace(-move, move, n_of(dur))
        return wide * np.stack([np.cos((p + 1) * np.pi / 4), np.sin((p + 1) * np.pi / 4)], 1) * 1.4
    return wide


def whoosh():
    S, o = {}, {}
    def add(name, x, desc, tags): S[name] = Sound(finish(x), desc, tags)
    add("short-01", air(.42, .15, [(0, 500), (.35, 3200), (1, 900)], 1.1, "pink", 1, move=.5), "tight pass, mid air", ("short", "mid"))
    add("short-02", air(.35, .12, [(0, 900), (.4, 5000), (1, 1500)], 1.6, "white", 2, move=-.6), "very quick bright pass, right to left", ("short", "bright"))
    add("bright-01", air(.6, .22, [(0, 2500), (.4, 9000), (1, 3500)], .8, "white", 3, move=.7), "hissy high pass", ("bright", "hiss"))
    add("bright-02", air(.75, .3, [(0, 3500), (.5, 11000), (1, 5000)], .7, "blue", 4, move=-.7), "very bright, thin, glassy", ("bright", "thin"))
    d1 = air(.8, .3, [(0, 150), (.4, 900), (1, 200)], 1.4, "brown", 5, kind="lowpass", move=.4)
    add("dark-01", d1 + stereo(sine(np.linspace(70, 45, n_of(.8)), .8)) * swell(.8, .3)[:, None] * .5, "low rumbling pass with sub", ("dark", "sub"))
    add("dark-02", air(1.0, .38, [(0, 250), (.4, 1100), (1, 300)], 2.0, "brown", 6, move=-.5), "dark, hollow, resonant", ("dark", "hollow"))
    add("long-01", air(1.5, .6, [(0, 300), (.45, 2400), (1, 500)], .8, "pink", 7, move=.8, rise=2.4), "slow wide pass left to right", ("long", "wide"))
    add("long-02", air(1.8, .9, [(0, 400), (.5, 3500), (1, 350)], 1.2, "pink", 8, move=-.9, rise=3, fall=1.2), "long swell and fade, right to left", ("long", "swell"))
    a1 = air(1.1, .4, [(0, 1200), (.5, 2800), (1, 1500)], .6, "pink", 9, move=.2)
    a1 = fx.reverb(a1 + air(1.1, .4, [(0, 4000), (.5, 8000), (1, 5000)], 1.0, "white", 10) * .25, 1.0, .5, .35)
    add("airy-01", a1, "breathy pass with a soft room", ("airy", "reverb"))
    r = rng("airy2")
    f = curve([(0, 700), (.5, 1500), (1, 800)], 1.3, log=True)
    b = sum(sweep(noise.pink(1.3, ("a2", k)), "bandpass", f * m, 7) * a * (.6 + .4 * np.sin(6.28 * (.9 + k * .4) * t_axis(1.3) + r.random() * 6)) for k, (m, a) in enumerate([(1, 1), (1.7, .8), (2.6, .6)]))
    add("airy-02", fx.decorrelate(b * swell(1.3, .5)), "whispered, vowel-like air", ("airy", "vocal"))
    rv = fx.reverb(air(.5, .08, [(0, 3000), (.3, 1500), (1, 400)], 1.0, "pink", 11, rise=1.0, fall=3.0), 1.0, .4, .45)
    add("reverse-01", reverse(rv), "a splash played backwards: sucks in then stops", ("reverse", "suck"))
    rv = fx.reverb(air(.6, .06, [(0, 1200), (.3, 600), (1, 200)], 1.2, "brown", 12, rise=1.0, fall=3.0), 1.4, .6, .45)
    add("reverse-02", reverse(rv), "dark reversed thud-swell", ("reverse", "dark"))
    return S


def swish():
    S = {}
    for i, (dur, up, lo, hi, q) in enumerate([(.18, 1, 1500, 8000, 2.5), (.22, 0, 7000, 1600, 3.0), (.26, 1, 800, 5000, 2.0), (.16, 0, 9000, 2500, 3.5)], 1):
        pts = [(0, lo), (.55, hi), (1, lo * 1.2)] if up else [(0, lo), (.5, hi), (1, hi * .8)]
        x = air(dur, dur * .4, pts, q, "white", 20 + i, move=.8 if i % 2 else -.8, rise=1.6, fall=1.3)
        z = fm(curve([(0, 3000 + i * 400), (1, 7000)], dur, log=True), 1.5, 3, dur) * swell(dur, dur * .4) * .08
        S[f"whip-0{i}"] = Sound(finish(x + stereo(z)), "fast whip through the air " + ("rising" if up else "falling"), ("whip", "fast"))
    for i, (dur, lo, hi) in enumerate([(.2, 500, 3000), (.28, 300, 2200), (.24, 900, 4500), (.3, 400, 2800)], 1):
        x = air(dur, dur * .55, [(0, lo), (.6, hi), (1, lo)], 1.3, "pink", 30 + i, move=.5 * (1 if i % 2 else -1), rise=2.2, fall=.7)
        snap = highpass(noise.white(.012, i), 2500) * np.exp(-t_axis(.012) / .003)
        x[int(dur * 48000 * .78):][: len(snap)] += stereo(snap[: len(x) - int(dur * 48000 * .78)]) * .5
        S[f"flick-0{i}"] = Sound(finish(x), "cloth flick with a snap at the end", ("flick", "cloth"))
    return S


def sweep_():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x), d, t)
    for k, (name, up) in enumerate([("noise-up", 1), ("noise-down", 0)], 1):
        pts = [(0, 250), (1, 7000)] if up else [(0, 7000), (1, 250)]
        add(f"{name}-01", air(1.5, .85 if up else .4, pts, 6, "white", 40 + k, move=.8 * (1 if up else -1), rise=1.5, fall=1.2), "narrow noise band " + ("climbing" if up else "falling"), ("noise", "resonant"))
    for k, (name, up) in enumerate([("tonal-up", 1), ("tonal-down", 0)], 1):
        f = curve([(0, 110), (1, 220)] if up else [(0, 220), (1, 110)], 1.6, log=True)
        s = stack(saw, f, 1.6, 5, 18, 50 + k) * swell(1.6, .8, 1.8, 1.4)
        cut = curve([(0, 300), (.5, 5000), (1, 400)] if up else [(0, 5000), (1, 300)], 1.6, log=True)
        add(f"{name}-01", fx.decorrelate(sweep(s, "lowpass", cut, 2.5), .5, k), "detuned saws through an opening filter", ("tonal", "synth"))
    add("flange-01", flanger(air(1.4, .6, [(0, 600), (.5, 6000), (1, 1000)], .7, "white", 60), .8, 3, 3.5, .8, .5), "noise through a deep flanger", ("flange", "jet"))
    add("flange-02", comb(air(1.5, .5, [(0, 400), (.5, 4000), (1, 600)], .8, "pink", 61, move=.8), 2.2, .75, .7), "ringing comb pass, pipe-like", ("comb", "metallic"))
    add("phase-01", flanger(air(1.7, .8, [(0, 800), (.6, 3000), (1, 700)], .9, "pink", 62), .35, 6, 8, .7, .2), "slow phaser wash", ("phase", "slow"))
    f = curve([(0, 400), (.3, 1800), (.5, 900), (.7, 3000), (1, 1200)], 1.3, log=True)
    x = sweep(noise.white(1.3, 63), "bandpass", f, 12) * swell(1.3, .65, 1.4, 1.4)
    add("radio-01", fx.bitcrush(fx.decorrelate(x * 1.5), 7, 2), "tuning across a radio dial", ("radio", "lofi"))
    return S


REGISTRY = {
    "whoosh": ("air moving past: short, bright, dark, long, airy and reversed", (0.5, 1.0), whoosh),
    "swish": ("fast whips and cloth flicks", (0.7, 1.0), swish),
    "sweep": ("long filter passes: noise, tonal, flanged, radio", (0.4, 1.0), sweep_),
}
