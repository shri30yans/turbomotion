"""Digital sounds and sparkle: glitch (stutter, chirp, data burst) and shimmer (sparkle clouds, glints, glissandi)."""
from __future__ import annotations

import numpy as np

from . import fx, grain, noise
from .core import SR, Sound, hz, n_of, put, rng, stereo, t_axis
from .master import finish
from .env import curve, exp_decay, swell
from .fam_ui import bell
from .filt import bandpass, highpass, lowpass
from .ks import pluck
from .modal import strike
from .osc import fm, saw, sine, square

PENTA = [0, 2, 4, 7, 9]  # C major pentatonic


def glitch():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x, 1, 5), d, t)
    for i, (f, period, reps, step) in enumerate([(220, .055, 6, 0), (330, .04, 8, 2), (150, .07, 5, -3)], 1):
        d = period * reps + .05
        x = np.zeros(n_of(d))
        for k in range(reps):
            src = saw(f * 2 ** (k * step / 12), period * .6) * exp_decay(period * .6, period * .4, .001)
            src += highpass(noise.white(period * .6, (i, k)), 2000) * exp_decay(period * .6, .01, .0005) * .4
            put(x, src, k * period, 1.0 - k * .08)
        add(f"stutter-0{i}", fx.bitcrush(fx.width(x, 1.3), 6 + i, 2 + i), "a sliced note repeated with a bitcrusher", ("stutter", "crush"))
    r = rng("chirp")
    for i, (f0, f1, n) in enumerate([(300, 5000, 3), (6000, 500, 4), (800, 3000, 5)], 1):
        d = .09 * n
        x = np.zeros(n_of(d))
        for k in range(n):
            ln = .06
            g = fm(curve([(0, f0 * (1 + .3 * k)), (1, f1)], ln, log=True), 2.0, 4, ln) * swell(ln, .01, 1, 1)
            put(x, g, k * .085)
        add(f"chirp-0{i}", fx.bitcrush(x, 5 + i, 2), "FM chirps, crushed", ("chirp", "fm"))
    for i, (lo, hi, burst) in enumerate([(800, 5000, .03), (2000, 7000, .018), (400, 2500, .05)], 1):
        d = .4
        rr = rng("data", i)
        x = np.zeros(n_of(d)); t = 0.0
        while t < d - .06:
            ln = burst * rr.uniform(.6, 1.6)
            fs = rr.choice(np.geomspace(lo, hi, 6), size=max(2, int(ln / .004)))
            seg = square(np.repeat(fs, int(ln * SR / len(fs)) + 1)[: n_of(ln)], ln) * .8
            put(x, seg, t); t += ln + rr.uniform(.01, .05)
        add(f"data-0{i}", fx.bitcrush(bandpass(x, lo * .8, hi * 1.5), 6, 2), "bursts of data", ("data", "burst"))
    return S


def shimmer():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x, 3, 12), d, t)
    for i, (n, span) in enumerate([(26, 1.0), (40, 1.4), (18, .8)], 1):
        r = rng("sparkle", i)
        d = span + .9
        x = np.zeros((n_of(d), 2))
        for k in range(n):
            m = 84 + int(PENTA[r.integers(0, 5)]) + (12 * int(r.integers(0, 2)) if i != 3 else 0)
            at = span * (k / n) ** (.8 if i == 2 else 1.3)
            g = np.sin(2 * np.pi * hz(m) * t_axis(.5)) * exp_decay(.5, r.uniform(.05, .18), .001) * r.uniform(.3, 1)
            pan_ = r.uniform(-.8, .8); put(x, np.stack([g * (1 - pan_) / 2, g * (1 + pan_) / 2], 1), at, .5)
        add(f"sparkle-0{i}", fx.reverb(x, 1.0, .3, .3, tail=.5), "a handful of high pentatonic pings " + ("rising", "dense", "short")[i - 1], ("sparkle", "pentatonic"))
    r = rng("shim")
    for i, (base, d) in enumerate([(hz("C6"), 1.6), (hz("G5"), 2.0)], 1):
        t = t_axis(d)
        x = sum(sine(base * m * (1 + .002 * r.standard_normal()), d, r.random()) * (.6 / m) * (.5 + .5 * np.sin(2 * np.pi * r.uniform(.3, 2) * t + r.random() * 6)) for m in (1, 1.5, 2, 3, 4))
        add(f"shimmer-0{i}", fx.reverb(fx.decorrelate(x * swell(d, d * .35, 1.8, 1.8), .5, i), 1.5, .3, .35), "a bright, slowly beating cluster", ("shimmer", "pad"))
    src = bell(hz("E6"), .5, .15)
    add("shimmer-03", grain.cloud(src, 1.8, 90, 45, 7, None, swell(1.8, .6, 1.6, 1.8), .9, 3) * 2.5, "granular cloud of bell grains", ("shimmer", "granular"))
    for i, (f0, d) in enumerate([(3200, 1.6), (4800, 1.8)], 1):
        g = strike(f0, "glass", d, .95, 70 + i, .5, spread=.5) * .8 + stereo(sine(f0 * 2.01, d) * exp_decay(d, .08)) * .15
        add(f"glint-0{i}", fx.reverb(g, 1.4, .3, .35, tail=.6), "a single star-like ting in a big room", ("glint", "ting"))
    for i, (low, up) in enumerate([(72, True), (96, False)], 1):
        ms = [low + o for o in (0, 2, 4, 7, 9, 12, 14, 16)]
        ms = ms if up else ms[::-1]
        d = 1.9
        x = np.zeros(n_of(d))
        for k, m in enumerate(ms): put(x, pluck(hz(m), 1.2, .995, .85, .2, (i, k)), k * .045, .7)
        add(f"gliss-0{i}", fx.reverb(fx.width(stereo(x), 1.3), 1.2, .4, .25, tail=.4), "pentatonic harp glissando " + ("up" if up else "down"), ("gliss", "harp"))
    return S


REGISTRY = {
    "glitch": ("digital stutters, chirps and data bursts", (0.5, 1.5), glitch),
    "shimmer": ("sparkle clouds, glints, shimmering beds and harp glissandi", (0.0, 1.5), shimmer),
}
