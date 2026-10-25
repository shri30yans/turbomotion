"""Impacts (sub booms, punches, struck metal/wood/glass, cinematic slams) and tonal stingers in C minor and F major."""
from __future__ import annotations

import numpy as np

from . import fx, noise
from .core import SR, Sound, hz, mix, n_of, put, rng, reverse, stereo, t_axis
from .master import finish
from .env import curve, exp_decay, swell
from .filt import filt, highpass, lowpass, sweep
from .ks import pluck
from .modal import contact, strike
from .osc import fm, saw, sine, stack


def thump(f0, d, tau, drop=1.8, click=0.0, seed=0):
    """Sine body that falls from `f0*drop` to `f0`, with an optional beater click."""
    t = t_axis(d)
    f = f0 + f0 * (drop - 1) * np.exp(-t / 0.045)
    x = sine(f, d) * exp_decay(d, tau, .002)
    return x + _click(x, click, seed)


def _click(x, amount, seed):
    c = np.zeros(len(x))
    if amount:
        k = contact(.012, 2500, seed)
        c[: len(k)] = k * amount
    return c


def impact():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x, 1, 12), d, t)
    for i, (f0, d, tau, sat) in enumerate([(48, 2.0, .7, 1.5), (36, 2.8, 1.0, 2.5)], 1):
        x = thump(f0, d, tau, 2.2) + lowpass(noise.brown(d, 100 + i), 180) * exp_decay(d, .12, .001) * .5
        add(f"sub-0{i}", fx.reverb(fx.saturate(x, sat), .7, .6, .12, tail=.2), "deep sub boom, long body", ("sub", "boom"))
    d = .45
    add("punch-01", fx.saturate(thump(62, d, .11, 3.0, .6, 1) + .25 * thump(180, d, .03, 1.5), 2.0), "tight kick-like punch", ("punch", "tight"))
    d = .6
    x = thump(52, d, .16, 3.4, .8, 2) + .4 * lowpass(noise.white(d, 3), 900) * exp_decay(d, .05) + .3 * strike(150, "wood", d, .8, 3, .6)
    add("punch-02", fx.reverb(fx.saturate(x, 2.5), .35, .5, .12, tail=.1), "fat punch with a body knock", ("punch", "fat"))
    for i, (f0, hard) in enumerate([(190, .9), (330, .7)], 1):
        m = strike(f0, "metal", 2.2, hard, i, 1.2, spread=.7)
        x = m * .9
        put(x, thump(f0 / 3, .5, .08, 2), 0, .6)
        put(x, contact(.005, 5000, i), 0, .6)
        add(f"metal-0{i}", fx.reverb(x, .5, .4, .15, tail=.3), "struck metal plate with a ring", ("metal", "ring"))
    for i, (f0, mat, hard, d) in enumerate([(140, "wood", .8, .6), (90, "bar", .5, .8)], 1):
        x = strike(f0, mat, d, hard, 10 + i, 1.0)
        put(x, thump(f0 / 2.2, .3, .06, 1.8), 0, .8)
        put(x, contact(.006, 3500, i))
        add(f"wood-0{i}", fx.reverb(x, .4, .6, .1, tail=.1), "wooden knock" if i == 1 else "hollow wooden thud", ("wood", "knock"))
    for i, f0 in enumerate([1300, 2200], 1):
        g = strike(f0, "glass", 1.8, .95, 20 + i, 1.1, spread=.6)
        deb = highpass(noise.crackle(.5, 400, i, 1.0), 4000)
        put(g, deb, 0, .35)
        put(g, contact(.004, 7000, i))
        add(f"glass-0{i}", fx.reverb(g, .8, .3, .25, tail=.4), "glass strike with a few shards", ("glass", "shatter"))
    d = 3.4
    sub = thump(40, d, 1.0, 2.4) + lowpass(noise.brown(d, 30), 200) * exp_decay(d, .15) * .5
    hit = strike(95, "metal", d, .8, 31, 2.0, spread=.8) * .5
    pre = reverse(fx.reverb(lowpass(noise.white(.6, 32), 4000) * exp_decay(.6, .1), 1.0, .5, .5, tail=.0))[: n_of(.5)] * .4
    add("cine-01", fx.reverb(mix(pre, (sub, .5), (hit, .5)), 1.8, .5, .3), "cinematic slam: reverse suck-in, sub, metal and a 2 s tail", ("cinematic", "tail"))
    d = 3.0
    brass = sweep(stack(saw, hz(36), d, 5, 20, 40), "lowpass", curve([(0, 2500), (.2, 300)], d, log=True), 1.5) * exp_decay(d, .6, .03)
    sub = thump(34, d, .9, 2.0)
    add("cine-02", fx.reverb(mix((sub, 0, .9), (brass, 0, .6), (highpass(noise.white(d, 41), 3000) * exp_decay(d, .3) * .25, 0)), 2.0, .55, .28), "low brass-like blast over a sub", ("cinematic", "brass"))
    return S


CHORDS = {"cmin": (48, [0, 3, 7, 12], "C minor"), "fmaj": (41, [0, 7, 12, 16], "F major")}


def stinger():
    S = {}
    for key, (root, iv, label) in CHORDS.items():
        notes = [root + i + 12 for i in iv]
        def add(k, x, d, tags): S[f"{key}-0{k}"] = Sound(finish(x, 2, 12), f"{label} {d}", (key,) + tags)
        d = 1.2
        br = sum(stack(saw, hz(m), d, 4, 14, (key, m)) for m in notes) / 4
        br = sweep(br, "lowpass", curve([(0, 600), (.1, 3500), (1, 400)], d, log=True), 1.1) * exp_decay(d, .35, .012)
        add(1, fx.reverb(fx.width(br, 1.3), 1.0, .5, .2), "brass-like stab", ("brass",))
        d = 2.0
        pl = np.zeros(n_of(d))
        for k, m in enumerate(notes):
            put(pl, pluck(hz(m), d - k * .035, .9975, .7, .15, (key, m)), k * .035)
        add(2, fx.reverb(fx.width(stereo(pl) , 1.2), 1.2, .4, .2), "strummed pluck chord", ("pluck",))
        d = 3.2
        b = sum(strike(hz(m + 12), "bell", d, .6, (key, m), 1.0, spread=.6) * .5 for m in notes[:3])
        add(3, fx.reverb(b, 1.6, .4, .25), "bell chord that rings out", ("bell",))
        d = 2.0
        pd = sum(stack(saw, hz(m), d, 4, 16, (key, "p", m)) for m in notes) * swell(d, .25, 1.5, 1.6)
        add(4, fx.reverb(fx.width(sweep(pd, "lowpass", 1400, 1.0), 1.5), 1.8, .5, .35), "soft pad hit with a long bloom", ("pad",))
        d = 1.8
        low = thump(hz(root - 12), d, .8, 1.6) + sweep(sum(stack(saw, hz(m - 12), d, 3, 10, (key, "l", m)) for m in notes[:3]), "lowpass", 500, 1.3) * exp_decay(d, .5, .01) * .7
        add(5, fx.reverb(fx.saturate(low, 1.8), .8, .5, .15, tail=.3), "low drone hit", ("low", "dark"))
        d = 2.2
        t = t_axis(d)
        ep = sum(fm(hz(m), 1.0, 1.8 * np.exp(-t / .35), d) * exp_decay(d, .9, .004) + .2 * fm(hz(m), 14.0, .6 * np.exp(-t / .05), d) * exp_decay(d, .2, .002) for m in notes) / 3
        add(6, fx.reverb(fx.width(ep, 1.4), 1.0, .4, .2), "electric-piano chord", ("keys", "fm"))
    return S


REGISTRY = {
    "impact": ("hits: sub booms, punches, struck metal, wood, glass and cinematic slams", (0.0, 1.0), impact),
    "stinger": ("tonal chord hits in C minor (cmin) and F major (fmaj)", (0.0, 1.0), stinger),
}
