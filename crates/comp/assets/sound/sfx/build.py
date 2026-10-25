"""Render the whole kit: `python3 -m sfx.build` from assets/sound/. Writes kit/<family>/<name>.wav and kit/index.json."""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

from . import fam_air, fam_drums, fam_glitch, fam_hit, fam_mech, fam_pitch, fam_rise, fam_ui
from .checks import check_file, features, sibling_problems
from .core import SR, write_wav
from .master import peak_time
from .fam_pitch import PITCHED

KIT = Path(__file__).resolve().parent.parent / "kit"
MODULES = [fam_air, fam_rise, fam_hit, fam_ui, fam_glitch, fam_mech, fam_pitch, fam_drums]

# The 15 original cue names. A name that is also a family is the family; the others pool several variants.
ALIASES = {
    "card-in": ["swish/flick", "paper/flip"], "stamp": ["impact/punch", "impact/wood"], "counter": ["ui/notify", "shimmer/glint"],
    "loss": ["downer/drop", "impact/sub"], "reveal": ["shimmer/sparkle", "shimmer/shimmer", "swell/cymbal"],
    "click": ["ui/click", "key/mouse"], "tick": ["mech/tick"], "boom": ["impact/sub", "impact/cine"], "type": ["key/key"],
    # curated pools for transitions: each is a handful of sounds that suit one kind of move
    "fast-pass": ["whoosh/short", "whoosh/bright", "swish/flick"],
    "soft-pass": ["whoosh/airy", "whoosh/long", "paper/cloth"],
    "heavy-pass": ["whoosh/dark", "whoosh/long", "downer/noise"],
    "wipe-pass": ["swish/whip", "whoosh/bright", "paper/flip"],
    "pull-back": ["whoosh/reverse", "downer/noise", "sweep/noise-down"],
}


# A bare `kit:<family>` draws only from these variants/styles (everything else is still reachable by name).
DEFAULT_POOLS = {"riser": ["quick", "noise-03", "pulse-02"]}


def is_mono(x: np.ndarray) -> bool:
    return x.ndim == 1 or np.abs(x[:, 0] - x[:, 1]).max() < 0.01 * max(np.abs(x).max(), 1e-9)


def render_all(only: set[str] | None = None, write: bool = True):
    index = {"version": 1, "sample_rate": SR, "families": {}, "aliases": ALIASES}
    problems, rendered = [], {}
    for mod in MODULES:
        for fam, (desc, (jit_p, jit_g), fn) in mod.REGISTRY.items():
            if only and fam not in only:
                continue
            sounds = fn()
            feats, entries = {}, {}
            for name, s in sounds.items():
                x = s.x if s.x.ndim == 1 or not is_mono(s.x) else s.x[:, 0]
                problems += [f"{fam}/{name}: {p}" for p in check_file(x)]
                f = features(x)
                feats[name] = f
                entries[name] = {"dur": round(f["dur"], 3), "peak": round(peak_time(x), 3), "desc": s.desc, "tags": list(s.tags),
                                 "channels": 1 if x.ndim == 1 else 2, "rms_db": round(f["rms_db"], 1), "centroid": round(f["centroid"])}
                if write:
                    (KIT / fam).mkdir(parents=True, exist_ok=True)
                    write_wav(KIT / fam / f"{name}.wav", x)
                rendered[(fam, name)] = x
            if fam not in PITCHED:
                problems += [f"{fam}: {p}" for p in sibling_problems(feats)]
            entry = {"desc": desc, "jitter": {"pitch": jit_p, "gain": jit_g}, "variants": entries}
            if fam in DEFAULT_POOLS:
                entry["default"] = DEFAULT_POOLS[fam]
            if fam in PITCHED:
                entry["pitched"] = {"lo": int(PITCHED[fam][0]), "hi": int(PITCHED[fam][1])}
            index["families"][fam] = entry
    return index, problems, rendered


def main(argv: list[str]) -> int:
    only = set(argv) or None
    index, problems, _ = render_all(only)
    if only is None:
        (KIT / "index.json").write_text(json.dumps(index, indent=1) + "\n")
    n = sum(len(f["variants"]) for f in index["families"].values())
    print(f"{n} sounds in {len(index['families'])} families -> {KIT}")
    for p in problems:
        print("PROBLEM", p)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
