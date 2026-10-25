"""Physical textures: paper and cloth; mechanical (shutter, servo, switch, ratchet, clock tick)."""
from __future__ import annotations

import numpy as np

from . import fx, noise
from .core import SR, Sound, n_of, put, rng, stereo, t_axis
from .master import finish
from .env import curve, exp_decay, lfo, swell
from .fam_air import air
from .filt import bandpass, highpass, lowpass, sweep
from .modal import contact, strike
from .osc import saw, sine, square


def paper():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x, 3, 12), d, t)
    for i, (dens, lo, hi, d) in enumerate([(300, 2500, 9000, .7), (180, 1500, 7000, .9), (450, 3500, 10000, .6)], 1):
        r = rng("rustle", i)
        wob = lfo(r.uniform(5, 11), d, .9) ** 2 * lfo(r.uniform(1, 3), d, .7)
        ch = [bandpass(noise.crackle(d, dens, (i, s), 1.5 + s), lo, hi) * wob * swell(d, d * .4, 1.2, 1.3) for s in (0, 1)]
        add(f"rustle-0{i}", np.stack(ch, 1) * 4, "dry paper rustle", ("rustle", "paper"))
    for i, (c, d, pk) in enumerate([(2500, .35, .12), (1400, .5, .2), (3800, .28, .08)], 1):
        x = air(d, pk, [(0, c * .4), (.4, c), (1, c * .5)], .6, "pink", 90 + i, kind="lowpass", move=.4 * (1 if i % 2 else -1), rise=1.7, fall=1.6)
        x = x + stereo(noise.crackle(d, 120, i, 2.0) * swell(d, pk, 1, 1)) * .5
        add(f"cloth-0{i}", x, "fabric brushing past " + ("(medium)", "(slow, heavy)", "(light, fast)")[i - 1], ("cloth", "fabric"))
    for i, (d, c) in enumerate([(.22, 6000), (.3, 4000)], 1):
        x = air(d, d * .45, [(0, 2500), (.5, c), (1, c * 1.2)], 1.1, "white", 95 + i, move=.5, rise=1.4, fall=1.5)
        flap = lowpass(noise.white(.06, 4 + i), 900) * exp_decay(.06, .018, .001)
        put(x, flap, d * .8, .9); put(x, contact(.01, 3000, i), d * .8, .5)
        add(f"flip-0{i}", x, "a page turned over", ("flip", "page"))
    d = .9
    ev = np.exp(-t_axis(d) / .35)
    r = rng("crumple")
    cr = np.zeros(n_of(d))
    for k in range(260):
        at = d * r.random() ** 1.6
        put(cr, bandpass(noise.white(.012, k), 1500, 8000) * np.exp(-t_axis(.012) / .003), at, r.random() ** 2)
    add("crumple-01", fx.decorrelate(cr * 2, .4), "paper balled up", ("crumple", "dense"))
    d = .55
    t = t_axis(d)
    tear = highpass(noise.pink(d, 7), 800) * (0.4 + 0.6 * (square(70 + 30 * np.sin(2 * np.pi * 3 * t), d) > 0)) * swell(d, d * .3, 1, 1.2)
    add("tear-01", fx.decorrelate(tear, .4), "a ripping tear", ("tear",))
    return S


def mech():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x, 1, 8), d, t)
    for i, (f, gap, mirror) in enumerate([(2200, .028, False), (3100, .035, False), (1800, .04, True)], 1):
        x = np.zeros(n_of(.2))
        for at, ff, g in ((0, f, 1.0), (gap, f * .72, .8)):
            put(x, strike(ff, "metal", .1, .9, (i, at), .08), at, g * .6); put(x, contact(.006, 4500, (i, at)), at, g * .7)
        if mirror: put(x, sine(120, .08) * exp_decay(.08, .02, .001), 0, .8)
        add(f"shutter-0{i}", x, "camera shutter, two clicks" + (" and a mirror slap" if mirror else ""), ("shutter", "camera"))
    for i, (f0, d, shape) in enumerate([(120, .7, "saw"), (190, .9, "saw"), (150, .8, "step")], 1):
        t = t_axis(d)
        f = f0 * (1 + 1.2 * np.minimum(1, t / .12) * np.clip((d - t) / .25, 0, 1)) * (1 + .02 * np.sin(2 * np.pi * 9 * t))
        w = (saw(f, d) if shape == "saw" else square(np.round(f / 30) * 30, d, .3))
        w = sweep(w, "lowpass", 600 + 1.4 * f, 2.5) * (1 + .5 * np.sin(2 * np.pi * f * .5 * t))
        w += bandpass(noise.white(d, 100 + i), 1500, 4500) * .12
        add(f"servo-0{i}", fx.width(w * np.minimum(1, t / .03) * np.clip((d - t) / .15, 0, 1), 1.2), "small motor spinning up and down", ("servo", "motor"))
    for i, (a, b) in enumerate([(1500, 900), (2300, 1400)], 1):
        x = np.zeros(n_of(.15))
        put(x, strike(a, "plastic", .06, .9, i, .7), 0, .8); put(x, contact(.006, 4000, i), 0); put(x, strike(b, "metal", .09, .8, 10 + i, .1), .04, .5); put(x, sine(160, .05) * exp_decay(.05, .015), .04, .6)
        add(f"switch-0{i}", x, "click-clack toggle switch", ("switch", "toggle"))
    for i, (n, d0, d1) in enumerate([(12, .11, .02), (16, .07, .016)], 1):
        r = rng("ratchet", i)
        ts = np.cumsum(np.linspace(d0, d1, n)[::-1] if i == 1 else np.linspace(d1, d0, n))
        x = np.zeros(n_of(ts[-1] + .15))
        for k, t0 in enumerate(ts):
            put(x, strike(r.uniform(1800, 2600), "plastic", .05, .9, (i, k), .5), t0, .6); put(x, contact(.005, 3500, (i, k)), t0, .7)
        add(f"ratchet-0{i}", x, "a ratchet " + ("slowing down", "speeding up")[i - 1], ("ratchet", "clicks"))
    x = np.zeros(n_of(.12)); put(x, strike(2400, "wood", .08, 1.0, 1, .5)); put(x, contact(.004, 5000, 1), 0, .8)
    add("tick-01", x, "dry clock tick", ("tick", "clock"))
    x = np.zeros(n_of(.4)); put(x, strike(2200, "wood", .08, 1.0, 2, .5)); put(x, contact(.004, 4800, 2), 0, .8); put(x, strike(1500, "wood", .1, .9, 3, .6), .22, .8); put(x, contact(.004, 3000, 3), .22, .6)
    add("tick-02", x, "tick then a lower tock", ("tick", "clock"))
    return S


REGISTRY = {
    "paper": ("paper rustles, cloth brushes, page flips, crumples and tears", (0.0, 1.5), paper),
    "mech": ("camera shutters, servos, switches, ratchets and clock ticks", (0.5, 1.5), mech),
}
