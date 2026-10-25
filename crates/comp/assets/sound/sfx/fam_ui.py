"""Interface sounds: clicks, taps, toggles, notifications; keyboard keys and mouse; bubbles and pops."""
from __future__ import annotations

import numpy as np

from . import fx, noise
from .core import SR, Sound, hz, n_of, put, rng, stereo, t_axis
from .master import finish
from .env import curve, exp_decay
from .filt import bandpass, highpass, lowpass, sweep
from .modal import contact, strike
from .osc import fm, sine, triangle


def tone(f, dur, tau, attack=.002, wave=sine):
    return wave(f, dur) * exp_decay(dur, tau, attack)


def bell(f, dur=.7, tau=.25, bright=1.0):
    t = t_axis(dur)
    return fm(f, 3.5, 2.0 * bright * np.exp(-t / .06), dur) * exp_decay(dur, tau, .002)


def ui():
    S = {}
    def add(n, x, d, t, small=1): S[n] = Sound(finish(x, 1, 8), d, t)
    for i, (f, c, tau) in enumerate([(1800, 4000, .006), (2600, 6000, .004), (1300, 3000, .009)], 1):
        d = .06
        x = tone(f, d, tau) * .7
        put(x, contact(.008, c, i), 0, .6)
        add(f"click-0{i}", x, ("crisp", "soft", "round")[i - 1] + " interface click", ("click",))
    for i, (f, mat) in enumerate([(220, None), (150, None)], 1):
        d = .12
        x = tone(f, d, .035, .001)
        put(x, contact(.01, 1200 * i, 10 + i), 0, .5)
        add(f"tap-0{i}", x, "soft fingertip tap" if i == 1 else "low knuckle tap", ("tap",))
    for name, a, b in (("toggle-on", 1100, 1650), ("toggle-off", 1500, 880)):
        d = .16
        x = np.zeros(n_of(d)); put(x, tone(a, .08, .02), 0, .6); put(x, tone(b, .1, .03), .045, .7); put(x, contact(.006, 5000, 3), 0, .3)
        add(name, x, "switch " + name[7:] + ": two quick blips " + ("rising" if a < b else "falling"), ("toggle",))
    add("hover-01", tone(2600, .05, .014, .008) * .5, "barely-there high blip", ("hover",))
    add("hover-02", bandpass(noise.white(.05, 5), 3500, 7000) * exp_decay(.05, .012, .006) * .6, "airy tick", ("hover",))
    seqs = [("notify-01", [(hz("G5"), 0), (hz("C6"), .11)], "two-note chime up a fourth"),
            ("notify-02", [(hz("E5"), 0), (hz("G5"), .09), (hz("C6"), .18)], "three-note rising chime"),
            ("notify-03", [(hz("D6"), 0), (hz("A6"), .13)], "high glassy ping-ping")]
    for name, notes, desc in seqs:
        x = np.zeros(n_of(1.0))
        for f, at in notes: put(x, bell(f, .8, .22), at, .8)
        add(name, fx.reverb(x, .6, .5, .15, tail=.15), desc, ("notify",))
    x = np.zeros(n_of(1.0))
    for k, m in enumerate([72, 76, 79, 84]): put(x, bell(hz(m), .7, .3, .7), k * .06, .7)
    add("success-01", fx.reverb(x, .8, .4, .2, tail=.2), "rising C major arpeggio of bells", ("success",))
    x = np.zeros(n_of(1.2)); put(x, bell(hz("F5"), .9, .35), 0); put(x, bell(hz("C6"), 1.0, .4), .1); put(x, bell(hz("A6"), 1.0, .4), .1, .4)
    add("success-02", fx.reverb(x, 1.0, .4, .25, tail=.3), "two-note lift into a sparkle", ("success",))
    d = .4
    t = t_axis(d)
    b = tone(curve([(0, 330), (1, 220)], d, log=True), d, .14, .003, triangle) + .4 * tone(curve([(0, 335), (1, 224)], d, log=True), d, .14, .003, triangle)
    add("error-01", fx.saturate(b, 2.2) * np.where(t % .13 < .1, 1, .35), "low double-buzz, a polite no", ("error",))
    x = np.zeros(n_of(.6)); put(x, strike(180, "wood", .3, .7, 4), 0); put(x, tone(hz("Eb3"), .3, .09, .004, triangle), .0, .7); put(x, tone(hz("A3"), .35, .1, .004, triangle), .12, .7)
    add("error-02", x, "wooden thunk and a falling tritone", ("error",))
    return S


def key():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x, 1, 6), d, t)
    for i in range(1, 9):
        r = rng("key", i)
        f = float(np.geomspace(200, 900, 8)[[3, 0, 6, 2, 5, 1, 7, 4][i - 1]])
        c = float(np.geomspace(1800, 6500, 8)[[5, 2, 7, 0, 4, 6, 1, 3][i - 1]])
        d = .1
        x = np.zeros(n_of(d))
        put(x, strike(r.uniform(1100, 3600), "plastic", d, r.uniform(.4, 1), i, r.uniform(.5, 1.4)), 0, r.uniform(.3, .7))
        put(x, tone(f, d, r.uniform(.01, .05), .001), r.uniform(0, .008), r.uniform(.5, 1))
        put(x, contact(.01, c, i), 0, .6)
        put(x, contact(.006, c * .7, 40 + i), .06, .15 + .1 * r.random())
        add(f"key-0{i}", x, f"keyboard key, thock {int(f)} Hz", ("key", "typing"))
    d = .22
    x = np.zeros(n_of(d)); put(x, tone(140, d, .05, .001), 0, .9); put(x, strike(1100, "plastic", d, .6, 50, 1.2), 0, .4); put(x, contact(.012, 2000, 51), 0, .7); put(x, contact(.01, 2500, 52), .09, .2)
    add("space-01", x, "wide space bar, deeper with a rattle", ("key", "space"))
    x = np.zeros(n_of(.16)); put(x, tone(210, .16, .04, .001), 0, .8); put(x, strike(1800, "plastic", .16, .9, 53), 0, .45); put(x, contact(.01, 3500, 54), 0, .7); put(x, contact(.008, 3000, 55), .1, .25)
    add("enter-01", x, "enter key, firm thock", ("key", "enter"))
    for i, (f, g) in enumerate([(2800, 6000), (2100, 4500)], 1):
        x = np.zeros(n_of(.14)); put(x, tone(f, .04, .007), 0, .6); put(x, contact(.006, g, 60 + i)); put(x, tone(f * .8, .04, .006), .085, .3); put(x, contact(.005, g * .8, 70 + i), .085, .5)
        add(f"mouse-0{i}", x, "mouse button press and release", ("mouse", "click"))
    return S


def pop():
    S = {}
    def add(n, x, d, t): S[n] = Sound(finish(x, 1, 8), d, t)
    for i, f in enumerate([400, 650, 950], 1):
        d = .2
        g = curve([(0, f), (.25, f * 1.9), (1, f * 2.1)], d, log=True)
        add(f"bubble-0{i}", tone(g, d, .045, .003) * .9, "water bubble, " + ("big", "medium", "small")[i - 1], ("bubble",))
    for i, (f, c) in enumerate([(700, 1500), (500, 900), (900, 2200)], 1):
        d = .12
        x = tone(curve([(0, f), (1, f * .35)], d, log=True), d, .02, .001) + lowpass(noise.white(d, 80 + i), c) * exp_decay(d, .006, .001) * .6
        add(f"pop-0{i}", x, "lip pop, " + ("round", "deep", "bright")[i - 1], ("pop",))
    for i, f in enumerate([520, 330], 1):
        d = .35
        x = np.zeros(n_of(d)); put(x, tone(curve([(0, f), (.2, f * 3), (1, f * 3.2)], .16, log=True), .16, .05, .002)); put(x, tone(curve([(0, f * 1.5), (.3, f * 3.6), (1, f * 3.8)], .14, log=True), .14, .04, .002), .08, .5)
        add(f"plop-0{i}", fx.reverb(x, .3, .6, .12, tail=.08), "water drop plop", ("plop",))
    add("blip-01", tone(curve([(0, 600), (1, 1100)], .12, log=True), .12, .03, .002), "soft rising blip", ("blip",))
    return S


REGISTRY = {
    "ui": ("interface clicks, taps, toggles, hovers, notifications, success and error", (0.5, 1.5), ui),
    "key": ("keyboard keys (each different), space, enter and mouse clicks", (0.5, 1.5), key),
    "pop": ("bubbles, lip pops, water plops", (1.0, 1.5), pop),
}
