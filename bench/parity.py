"""Visual parity: native stills against frozen frames from the Remotion reference.

    python3 bench/parity.py               # compare every shot of the corpus
    python3 bench/parity.py --shots s001,s002
    python3 bench/parity.py --regenerate  # redraw the goldens with Remotion (needs `npm install` in bench/remotion)

The goldens (bench/golden/<shot>.jpg) are Remotion's stills of bench/corpus/film.json, each at the
shot's default still frame. Exit status 1 when any shot falls below MIN_SSIM.
"""
import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get('TURBOMOTION', ROOT / 'target/release/turbomotion'))
FILM = ROOT / 'bench/corpus/film.json'
PUBLIC = ROOT / 'bench/corpus/public'
ASSETS = ROOT / 'assets'
GOLDEN = ROOT / 'bench/golden'
MIN_SSIM = 0.93


def run(*cmd):
    return subprocess.run([str(c) for c in cmd], check=True, capture_output=True, text=True)


def regenerate(shots):
    with tempfile.TemporaryDirectory() as tmp:
        run('node', ROOT / 'bench/remotion/render.mjs', 'stills', FILM, f'--public={PUBLIC}', f'--assets={ASSETS}', f'--dir={tmp}', *([f'--shots={",".join(shots)}'] if shots else []))
        GOLDEN.mkdir(exist_ok=True)
        for png in sorted(Path(tmp).glob('*.png')):
            run('ffmpeg', '-v', 'error', '-y', '-i', png, '-q:v', '2', GOLDEN / f'{png.stem}.jpg')
            print('golden', png.stem)


def compare(shots):
    film = json.loads(FILM.read_text())
    only = shots or [s['id'] for s in film['shots']]
    kinds = {s['id']: s['type'] for s in film['shots']}
    failures = []
    with tempfile.TemporaryDirectory() as tmp:
        run(BINARY, 'stills', FILM, '--public', PUBLIC, '--assets', ASSETS, '--dir', tmp, '--shots', ','.join(only))
        for shot in only:
            out = run('ffmpeg', '-i', Path(tmp) / f'{shot}.png', '-i', GOLDEN / f'{shot}.jpg', '-lavfi', '[0][1]ssim;[0][1]psnr', '-f', 'null', '-').stderr
            ssim = float(re.search(r' All:([\d.]+)', out).group(1))
            psnr = re.search(r'average:([\d.]+|inf)', out).group(1)
            flag = '' if ssim >= MIN_SSIM else f'  <-- below {MIN_SSIM}'
            print(f'{shot} {kinds[shot]:9} SSIM {ssim:.3f}  PSNR {psnr} dB{flag}')
            if ssim < MIN_SSIM:
                failures.append(shot)
    print(f'{len(only) - len(failures)}/{len(only)} shots at or above SSIM {MIN_SSIM}')
    sys.exit(1 if failures else 0)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--shots', type=lambda s: s.split(','))
    parser.add_argument('--regenerate', action='store_true')
    args = parser.parse_args()
    regenerate(args.shots) if args.regenerate else compare(args.shots)


main()
