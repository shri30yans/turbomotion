"""Build examples/studio-effects/film.json: one two-second scene per effect in the catalogue, each showing the effect
on a title (with its own example parameters), plus a scene that animates a blur with keys.

    python3 examples/studio-effects/make.py          # writes film.json next to this file
    turbomotion studio stills examples/studio-effects --out /tmp/fx --times 1.2,3.2,...
"""
import json, subprocess
from pathlib import Path

HERE = Path(__file__).parent
BIN = HERE.parents[1] / 'target/release/turbomotion'
FONTS = {'dinc': ['Supplemental/DIN Condensed Bold.ttf', 0], 'avn_b': ['Avenir Next.ttc', 8], 'avn_m': ['Avenir Next.ttc', 5]}
SECONDS = 2.0

catalogue = json.loads(subprocess.run([str(BIN), 'studio', 'effects'], check=True, capture_output=True, text=True).stdout)


def scene(i, name, summary, fx):
    t0 = i * SECONDS
    return dict(
        id=name, t0=t0, t1=t0 + SECONDS, trans='cut', trd=0.5, grade={}, vig=0.5, bloom=0.3, ca=0.0, bg={'style': 'studio'}, cam=[], shakes=[],
        els=[
            dict(k='title', text='EFFECT', size=230, fill=[222, 176, 82], x=960, y=470, t0=t0, **{'in': 'none'}, fx=fx),
            dict(k='text', text=name.replace('_', ' ').upper(), font='avn_b', size=54, x=960, y=800, t0=t0, **{'in': 'none'}),
            dict(k='text', text=summary, font='avn_m', size=30, fill=[160, 175, 195], maxw=1500, x=960, y=880, t0=t0, **{'in': 'none'}),
        ])


scenes = [scene(i, e['effect'], e['summary'], [e['example']]) for i, e in enumerate(catalogue)]
n = len(scenes)
scenes.append(scene(n, 'animated_blur', 'Any number can be animated with [[seconds, value], ...] keys.', [{'effect': 'blur', 'radius': [[n * SECONDS + 0.2, 28], [n * SECONDS + 1.6, 0]]}]))
film = dict(fps=24, width=1920, height=1080, end=len(scenes) * SECONDS, fonts=FONTS, font_dir='/System/Library/Fonts/', scenes=scenes)
(HERE / 'film.json').write_text(json.dumps(film, indent=1))
print(len(scenes), 'scenes; look at t =', ', '.join(f'{i * SECONDS + 1.4:.1f}' for i in range(len(scenes))))
