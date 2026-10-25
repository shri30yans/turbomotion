"""Quality checks on rendered sounds: clipping, DC, edge clicks, silence, and whether siblings really differ."""
from __future__ import annotations

import numpy as np

from .core import SR, mono, to_db
from .master import envelope, peak_time


def features(x: np.ndarray) -> dict:
    """Numbers that describe a sound: duration, peak time, spectral centroid (Hz), crest, a 48-point envelope."""
    m = mono(x)
    spec = np.abs(np.fft.rfft(m * np.hanning(len(m)))) ** 2
    freq = np.fft.rfftfreq(len(m), 1 / SR)
    env = envelope(x, 10.0)
    idx = np.linspace(0, len(env) - 1, 48)
    curve = np.interp(idx, np.arange(len(env)), env)
    rms = np.sqrt(np.mean(m**2))
    bands = np.array([spec[(freq >= lo) & (freq < lo * 1.5)].sum() for lo in np.geomspace(60, 18000, 14)]) + 1e-12
    return {
        "bands": 10 * np.log10(bands / bands.sum()),
        "dur": len(m) / SR,
        "peak": peak_time(x),
        "centroid": float((spec * freq).sum() / max(spec.sum(), 1e-12)),
        "crest_db": float(to_db(np.abs(m).max() / max(rms, 1e-9))),
        "rms_db": float(to_db(rms)),
        "env": curve / max(curve.max(), 1e-12),
    }


def check_file(x: np.ndarray) -> list[str]:
    """Problems with one rendered sound (empty list is good)."""
    bad = []
    if not np.isfinite(x).all():
        bad.append("non-finite samples")
    pk = np.abs(x).max()
    if pk >= 0.999:
        bad.append(f"clipping (peak {pk:.3f})")
    if pk < 0.05:
        bad.append(f"almost silent (peak {pk:.3f})")
    if abs(mono(x).mean()) > 0.004:
        bad.append(f"DC offset {mono(x).mean():.4f}")
    edge = max(abs(x[0]).max(), abs(x[-1]).max())
    if edge > 0.01:
        bad.append(f"click at an edge ({edge:.3f})")
    return bad


def sibling_problems(feats: dict[str, dict]) -> list[str]:
    """Two variants are 'the same' when centroid and duration are within 4 % and their envelopes correlate above 0.97."""
    bad, names = [], list(feats)
    for i, a in enumerate(names):
        for b in names[i + 1 :]:
            fa, fb = feats[a], feats[b]
            near_c = abs(np.log(fa["centroid"] / fb["centroid"])) < 0.04
            near_d = abs(np.log(fa["dur"] / fb["dur"])) < 0.04
            corr = float(np.corrcoef(fa["env"], fb["env"])[0, 1]) if fa["env"].std() > 0 and fb["env"].std() > 0 else 1.0
            spec_gap = float(np.abs(fa["bands"] - fb["bands"]).mean())
            if near_c and near_d and corr > 0.97 and spec_gap < 3.0:
                bad.append(f"{a} and {b} are too alike (centroid {fa['centroid']:.0f}/{fb['centroid']:.0f} Hz, env corr {corr:.2f}, band gap {spec_gap:.1f} dB)")
    return bad
