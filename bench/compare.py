"""Head-to-head benchmark: turbomotion against the Remotion implementation of the same shots.

    python3 bench/compare.py types sweep full quality --reps 3 --out bench/results/run.json

Both renderers draw bench/corpus/film.json (1920x1080, 30 fps, 7503 frames, every shot type) into
a silent H.264 mp4 with the same encoder settings. Each measurement is a whole process, start to
finish: wall time, CPU time of the process tree, and the peak resident memory of the tree.

  types    the first 150 frames of each shot type, one worker on each side (per-type cost)
  sweep    a 900-frame slice at several parallelism settings on each side; picks each side's best
  full     the whole corpus at each side's best parallelism, with both encoders
  quality  SSIM/PSNR of the native video against the Remotion video, and frame determinism

Runs alternate native/Remotion so that thermal drift hits both equally. See bench/README.md.
"""
import argparse
import json
import os
import platform
import re
import resource
import statistics
import subprocess
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get('TURBOMOTION', ROOT / 'target/release/turbomotion'))
FILM = ROOT / 'bench/corpus/film.json'
PUBLIC = ROOT / 'bench/corpus/public'
ASSETS = ROOT / 'crates/shots/assets'
REMOTION = ROOT / 'bench/remotion/render.mjs'
WORK = ROOT / 'bench/work'
COOLDOWN = 5  # seconds between runs

# (first frame, frames) of each shot type's first 150 frames, from the corpus's contiguous layout.
TYPE_SLICES = {'stat': 0, 'card': 180, 'photo': 675, 'painting': 885, 'compare': 1035, 'map': 1470, 'diagram': 2580, 'plan': 5985}
SLICE = 150
# A mixed slice (photo, painting, compare, map) for the parallelism sweep.
SWEEP_RANGE = (675, 1575)
NATIVE_WORKERS = [1, 2, 3, 4, 6]
REMOTION_SETTINGS = [(1, 1), (1, 5), (1, 10), (2, 4), (3, 3), (3, 4), (5, 2)]  # (browsers, concurrency)

ENCODERS = {
    'x264': {'native': ['--codec', 'software', '--crf', '17', '--preset', 'faster'], 'remotion': ['--codec=x264', '--crf=17', '--preset=faster']},
    'hardware': {'native': ['--codec', 'hardware', '--bitrate', '20M'], 'remotion': ['--codec=hardware', '--bitrate=20M']},
}


def process_tree_rss(root_pid):
    """Resident memory (MiB) of a process and all of its descendants."""
    table = subprocess.run(['ps', '-axo', 'pid=,ppid=,rss='], capture_output=True, text=True).stdout.split('\n')
    rows = [tuple(map(int, line.split())) for line in table if line.strip()]
    children = {}
    for pid, ppid, _ in rows:
        children.setdefault(ppid, []).append(pid)
    tree, queue = {root_pid}, [root_pid]
    while queue:
        for child in children.get(queue.pop(), []):
            if child not in tree:
                tree.add(child)
                queue.append(child)
    return sum(rss for pid, _, rss in rows if pid in tree) / 1024


def measure(cmd):
    """Run `cmd`; return wall seconds, CPU seconds of everything it spawned, peak tree RSS in MiB, stdout."""
    cpu_before = resource.getrusage(resource.RUSAGE_CHILDREN)
    peak, running = 0.0, threading.Event()
    started = time.monotonic()
    proc = subprocess.Popen([str(c) for c in cmd], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)

    def sample():
        nonlocal peak
        while not running.is_set():
            peak = max(peak, process_tree_rss(proc.pid))
            running.wait(0.25)

    sampler = threading.Thread(target=sample)
    sampler.start()
    out = proc.communicate()[0]
    wall = time.monotonic() - started
    running.set()
    sampler.join()
    if proc.returncode:
        raise RuntimeError(f'{cmd[0]} failed ({proc.returncode}):\n{out}')
    cpu_after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = (cpu_after.ru_utime - cpu_before.ru_utime) + (cpu_after.ru_stime - cpu_before.ru_stime)
    return {'wall': wall, 'cpu': cpu, 'peak_rss_mib': peak, 'output': out}


def native(out, frames, workers, encoder):
    first, last = frames
    cmd = [BINARY, 'render', FILM, '--public', PUBLIC, '--assets', ASSETS, '--out', out, '--from', first, '--to', last, '--workers', workers, *ENCODERS[encoder]['native']]
    result = measure(cmd)
    # The binary prints "<n> frames in <s>s = <fps> fps"; that is the render without process start.
    result['render'] = float(re.search(r'frames in ([\d.]+)s', result.pop('output')).group(1))
    return result


def remotion(out, frames, setting, encoder):
    first, last = frames
    browsers, concurrency = setting
    cmd = ['node', REMOTION, 'video', FILM, f'--public={PUBLIC}', f'--assets={ASSETS}', f'--out={out}', f'--from={first}', f'--to={last}',
           f'--browsers={browsers}', f'--concurrency={concurrency}', *ENCODERS[encoder]['remotion']]
    result = measure(cmd)
    timings = json.loads(result.pop('output').strip().splitlines()[-1])
    result['render'] = timings['render']
    result['setup'] = timings['bundle'] + timings['browser']
    return result


def summarise(runs, frames):
    """Median of each metric over repeated runs, with the spread of wall time."""
    walls = [r['wall'] for r in runs]
    renders = [r['render'] for r in runs]
    med = statistics.median
    return {
        'runs': len(runs), 'frames': frames,
        'wall_s': med(walls), 'wall_min_s': min(walls), 'wall_max_s': max(walls),
        'render_s': med(renders), 'fps': frames / med(renders),
        'cpu_s': med(r['cpu'] for r in runs), 'peak_rss_mib': med(r['peak_rss_mib'] for r in runs),
        **({'setup_s': med(r['setup'] for r in runs)} if 'setup' in runs[0] else {}),
    }


def pause():
    time.sleep(COOLDOWN)


def suite_types(reps, results):
    results['types'] = {}
    for name, first in TYPE_SLICES.items():
        frames = (first, first + SLICE)
        runs = {'native': [], 'remotion': []}
        for rep in range(reps):
            for side in ('native', 'remotion'):
                out = WORK / f'types-{name}-{side}-{rep}.mp4'
                runs[side].append(native(out, frames, 1, 'x264') if side == 'native' else remotion(out, frames, (1, 1), 'x264'))
                pause()
        results['types'][name] = {side: summarise(r, SLICE) for side, r in runs.items()}
        t = results['types'][name]
        print(f'types {name:9} native {t["native"]["fps"]:7.1f} fps   remotion {t["remotion"]["fps"]:6.1f} fps   x{t["native"]["fps"] / t["remotion"]["fps"]:.1f}', flush=True)


def suite_sweep(results):
    frames = SWEEP_RANGE
    n = frames[1] - frames[0]
    sweep = {'native': {}, 'remotion': {}}
    for workers in NATIVE_WORKERS:
        sweep['native'][f'{workers}'] = summarise([native(WORK / 'sweep-native.mp4', frames, workers, 'x264')], n)
        print(f'sweep native {workers} workers: {sweep["native"][str(workers)]["fps"]:.1f} fps', flush=True)
        pause()
    for browsers, concurrency in REMOTION_SETTINGS:
        key = f'{browsers}x{concurrency}'
        sweep['remotion'][key] = summarise([remotion(WORK / 'sweep-remotion.mp4', frames, (browsers, concurrency), 'x264')], n)
        print(f'sweep remotion {browsers} browsers x {concurrency} tabs: {sweep["remotion"][key]["fps"]:.1f} fps', flush=True)
        pause()
    results['sweep'] = sweep
    results['best'] = {
        'native': max(sweep['native'], key=lambda k: sweep['native'][k]['fps']),
        'remotion': max(sweep['remotion'], key=lambda k: sweep['remotion'][k]['fps']),
    }
    print('best:', results['best'], flush=True)


def best(results, side):
    key = results['best'][side]
    return int(key) if side == 'native' else tuple(map(int, key.split('x')))


def suite_full(reps, results):
    total = json.loads(FILM.read_text())['durationInFrames']
    results['full'] = {}
    for encoder in ENCODERS:
        runs = {'native': [], 'remotion': []}
        for rep in range(reps):
            runs['native'].append(native(WORK / f'full-native-{encoder}-{rep}.mp4', (0, total), best(results, 'native'), encoder))
            pause()
            if rep == 0:  # a Remotion pass of the whole corpus takes many minutes: once
                runs['remotion'].append(remotion(WORK / f'full-remotion-{encoder}.mp4', (0, total), best(results, 'remotion'), encoder))
                pause()
        results['full'][encoder] = {side: summarise(r, total) for side, r in runs.items()}
        f = results['full'][encoder]
        print(f'full {encoder:9} native {f["native"]["wall_s"]:7.1f}s   remotion {f["remotion"]["wall_s"]:7.1f}s   x{f["remotion"]["wall_s"] / f["native"]["wall_s"]:.1f}', flush=True)


def compare_videos(a, b):
    """SSIM and PSNR of video `a` against reference `b` (the Remotion render)."""
    log = subprocess.run(['ffmpeg', '-v', 'info', '-i', a, '-i', b, '-lavfi', '[0:v][1:v]ssim;[0:v][1:v]psnr', '-f', 'null', '-'], capture_output=True, text=True).stderr
    return {'ssim': float(re.search(r'SSIM.* All:([\d.]+)', log).group(1)), 'psnr_db': float(re.search(r'PSNR.* average:([\d.]+)', log).group(1))}


def frame_digest(video):
    """Digest of every decoded frame, to tell identical renders from similar ones."""
    return subprocess.run(['ffmpeg', '-v', 'error', '-i', video, '-f', 'framemd5', '-'], capture_output=True, text=True, check=True).stdout.split('#')[-1]


def suite_quality(results):
    quality = {'versus_remotion': {}, 'deterministic': {}}
    for name, first in TYPE_SLICES.items():
        quality['versus_remotion'][name] = compare_videos(WORK / f'types-{name}-native-0.mp4', WORK / f'types-{name}-remotion-0.mp4')
        q = quality['versus_remotion'][name]
        print(f'quality {name:9} SSIM {q["ssim"]:.4f}  PSNR {q["psnr_db"]:.1f} dB', flush=True)
    frames = (TYPE_SLICES['photo'], TYPE_SLICES['photo'] + SLICE)
    for side, run in (('native', lambda out: native(out, frames, 1, 'x264')), ('remotion', lambda out: remotion(out, frames, (1, 1), 'x264'))):
        run(WORK / f'det-{side}-a.mp4'), run(WORK / f'det-{side}-b.mp4')
        quality['deterministic'][side] = frame_digest(WORK / f'det-{side}-a.mp4') == frame_digest(WORK / f'det-{side}-b.mp4')
        print(f'determinism {side}: {quality["deterministic"][side]}', flush=True)
    results['quality'] = quality


def machine():
    chip = subprocess.run(['sysctl', '-n', 'machdep.cpu.brand_string'], capture_output=True, text=True).stdout.strip()
    memory = int(subprocess.run(['sysctl', '-n', 'hw.memsize'], capture_output=True, text=True).stdout) // 2 ** 30
    version = lambda *cmd: subprocess.run(cmd, capture_output=True, text=True).stdout.splitlines()[0]
    return {'chip': chip, 'memory_gib': memory, 'os': platform.platform(), 'ffmpeg': version('ffmpeg', '-version'), 'node': version('node', '-v'),
            'chrome': version('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '--version'),
            'remotion': json.loads((ROOT / 'bench/remotion/package.json').read_text())['dependencies']['remotion'],
            'date': time.strftime('%Y-%m-%d')}


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('suites', nargs='+', choices=['types', 'sweep', 'full', 'quality'])
    parser.add_argument('--reps', type=int, default=3)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--resume', action='store_true', help='keep the results already in --out and add to them')
    args = parser.parse_args()
    WORK.mkdir(exist_ok=True)
    results = json.loads(args.out.read_text()) if args.resume and args.out.exists() else {}
    results['machine'] = machine()
    results['spec'] = {'film': 'bench/corpus/film.json', 'size': '1920x1080', 'fps': 30, 'x264': 'crf 17, preset faster', 'hardware': 'VideoToolbox, 20 Mbit/s'}
    if 'types' in args.suites:
        suite_types(args.reps, results)
    if 'sweep' in args.suites:
        suite_sweep(results)
    if 'full' in args.suites:
        suite_full(args.reps, results)
    if 'quality' in args.suites:
        suite_quality(results)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(results, indent=1))


main()
