"""Drum one-shots: kicks, snares, claps, hats, rims, toms."""
from __future__ import annotations

import numpy as np

from . import fx, noise
from .core import SR, Sound, n_of, put, rng, stereo, t_axis
from .master import finish
from .env import curve, exp_decay, swell
from .fam_hit import thump
from .filt import bandpass, highpass, lowpass
from .modal import contact, strike
from .osc import sine, square


def kick():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x, 1, 8), d, t)
    add("808-01", fx.saturate(thump(48, 1.2, .5, 2.8), 2.5), "long 808 boom kick", ("808", "long"))
    add("punchy-01", fx.saturate(thump(56, .45, .13, 3.0, .7, 1), 2.0), "tight punchy kick", ("punchy",))
    add("soft-01", thump(52, .5, .2, 2.0, .15, 2), "soft round kick", ("soft",))
    add("techno-01", fx.saturate(thump(50, .3, .09, 4.0, 1.0, 3), 3.0), "short hard techno kick", ("techno", "hard"))
    add("sub-01", thump(42, .9, .35, 1.6), "pure sub thump", ("sub",))
    x = fx.bitcrush(fx.saturate(thump(60, .5, .15, 3.2, .9, 4), 7.0), 9)
    add("dist-01", x, "overdriven, crushed kick", ("distorted",))
    return S


def snare():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x, 1, 8), d, t)
    def build(f, nz_lo, nz_tau, tone_tau, tone_g, d, seed):
        x = thump(f, d, tone_tau, 1.5) * tone_g + strike(f * 1.5, "membrane", d, .8, seed, .25) * .25
        put(x, bandpass(noise.white(d, seed), nz_lo, 9000) * exp_decay(d, nz_tau, .001), 0, .9)
        return x
    add("tight-01", build(210, 2500, .07, .05, .8, .25, 1), "short, tight snare", ("tight",))
    add("fat-01", fx.reverb(build(170, 1500, .16, .09, 1.0, .5, 2), .4, .5, .15, tail=.1), "fat snare with a room", ("fat",))
    d = .35
    t = t_axis(d)
    x = sine(curve([(0, 330), (1, 170)], d, log=True), d) * exp_decay(d, .06) + highpass(noise.white(d, 3), 3500) * exp_decay(d, .1) * .9
    add("electro-01", fx.saturate(x, 1.6), "synthetic electro snare", ("electro",))
    add("snap-01", fx.reverb(build(190, 4000, .1, .06, .6, .35, 4), .3, .6, .1, tail=.05), "bright rimmy snare", ("snappy",))
    d = .3
    add("brush-01", lowpass(noise.white(d, 5), 7000) * swell(d, .12, 1.5, 1.2) * .9, "brush swish", ("brush", "soft"))
    x = fx.reverb(build(200, 1800, .12, .08, 1.0, .3, 6), 1.2, .3, .6, tail=.4)
    x[int(SR * .32):] *= (np.linspace(1, 0, len(x) - int(SR * .32)) ** 3)[:, None]
    add("gated-01", x, "gated-reverb snare", ("gated", "80s"))
    return S


def clap():
    S = {}
    for i, (n, gap, tail, c) in enumerate([(4, .009, .12, 1500), (5, .011, .16, 1100), (3, .007, .09, 2000), (6, .013, .22, 1300)], 1):
        r = rng("clap", i)
        d = .4
        x = np.zeros(n_of(d))
        for k in range(n):
            put(x, bandpass(noise.white(.02, (i, k)), c * .7, c * 2.4) * exp_decay(.02, .004, .0003), k * gap * r.uniform(.8, 1.2), 1 - .1 * k)
        put(x, bandpass(noise.white(d, (i, 9)), c * .7, c * 2.4) * exp_decay(d, tail * .4, .0005), n * gap, .9)
        S[f"clap-0{i}"] = Sound(finish(fx.reverb(x, .35, .5, .12, tail=.05), 1, 8), "hand clap with " + str(n) + " layers", ("clap",))
    return S


def metallic(d, base, lo=7000.0, hi=14000.0):
    """The classic six-square-wave cymbal tone, band-limited to `lo`..`hi` Hz."""
    sq = sum(square(f * base, d) for f in (205.3, 304.4, 369.6, 522.7, 540.4, 800.0))
    return highpass(bandpass(sq, lo * .9, hi), lo)


def hat():
    S = {}
    def add(n, d, tau, base, desc, tags, lo=7000.0, hi=14000.0, g=1.0):
        x = metallic(d, base, lo, hi) * exp_decay(d, tau, .0004) + highpass(noise.white(d, n), lo + 1000) * exp_decay(d, tau, .0004) * .25
        S[n] = Sound(finish(x * g, 1, 8), desc, tags)
    add("closed-01", .12, .02, 1.0, "tight closed hat", ("closed",))
    add("closed-02", .15, .03, 1.15, "slightly longer, brighter closed hat", ("closed", "bright"), 9500, 17000)
    add("closed-03", .1, .015, .9, "very short dark tick hat", ("closed", "dark"), 4200, 9000)
    add("open-01", .6, .22, 1.0, "open hat that rings", ("open",), 6000, 13000)
    add("open-02", .9, .38, 1.2, "long sizzling open hat", ("open", "long"), 8500, 16000)
    add("pedal-01", .14, .04, .8, "soft pedal hat chick", ("pedal", "soft"), 3000, 7000)
    return S


def rim():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x, 1, 6), d, t)
    x = strike(1700, "plastic", .12, 1.0, 1, 1.5)
    put(x, thump(300, .1, .02, 1.3), 0, .7)
    put(x, contact(.005, 4500, 1), 0, .6)
    add("rim-01", x, "rim shot click", ("rimshot",))
    x = strike(900, "can", .25, .9, 2, .6)
    put(x, contact(.006, 3500, 2), 0, .7)
    add("rim-02", x, "metallic side-stick", ("sidestick", "metal"))
    d = .2
    add("clave-01", sine(2500, d) * exp_decay(d, .045, .0005) + .4 * sine(3720, d) * exp_decay(d, .02, .0005), "wooden clave", ("clave", "wood"))
    return S


def tom():
    S = {}
    for n, f in (("floor-01", 78), ("low-01", 110), ("mid-01", 150), ("high-01", 210)):
        d = .9 if f < 120 else .6
        x = thump(f, d, .22 * (f / 100) ** -.5, 1.7, .4, f) + .35 * strike(f * 1.2, "membrane", d, .6, f, .5)
        S[n] = Sound(finish(fx.reverb(x, .35, .6, .1, tail=.05), 1, 8), f"tom around {f} Hz", ("tom",))
    return S


REGISTRY = {
    "kick": ("kick drums: 808, punchy, soft, techno, sub, distorted", (0.5, 1.0), kick),
    "snare": ("snares: tight, fat, electro, snappy, brush, gated", (0.5, 1.0), snare),
    "clap": ("hand claps", (0.5, 1.0), clap),
    "hat": ("hi-hats: closed, open, pedal", (0.5, 1.0), hat),
    "rim": ("rim shots, side-sticks and clave", (0.5, 1.0), rim),
    "tom": ("toms from floor to high", (0.5, 1.0), tom),
}
