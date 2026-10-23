"""Print the benchmark results (bench/results/*.json) as the markdown tables used in the README.

    python3 bench/report.py bench/results/m5.json
"""
import json
import sys
from pathlib import Path


def table(header, rows):
    line = lambda cells: '| ' + ' | '.join(str(c) for c in cells) + ' |'
    return '\n'.join([line(header), line(['---'] + [':---:'] * (len(header) - 1)), *map(line, rows)])


def main():
    r = json.loads(Path(sys.argv[1]).read_text())
    m = r['machine']
    print(f'Machine: {m["chip"]}, {m["memory_gib"]} GiB, {m["os"]}; {m["chrome"]}, Remotion {m["remotion"]}, {m["ffmpeg"].split(" Copyright")[0]}, Node {m["node"]} ({m["date"]})\n')

    if 'types' in r:
        rows = [[k, f'{v["native"]["fps"]:.0f}', f'{v["remotion"]["fps"]:.1f}', f'**{v["native"]["fps"] / v["remotion"]["fps"]:.1f}x**',
                 f'{r["quality"]["versus_remotion"][k]["ssim"]:.3f}' if 'quality' in r else ''] for k, v in r['types'].items()]
        print(table(['shot type', 'native fps', 'Remotion fps', 'speed-up', 'SSIM vs Remotion'], rows), '\n')

    if 'sweep' in r:
        rows = [['native', f'{k} worker(s)', f'{v["fps"]:.1f}'] for k, v in r['sweep']['native'].items()]
        rows += [['Remotion', f'{k.split("x")[0]} browser(s) x {k.split("x")[1]} tab(s)', f'{v["fps"]:.1f}'] for k, v in r['sweep']['remotion'].items()]
        print(table(['renderer', 'parallelism', 'fps'], rows), '\n')

    if 'full' in r:
        rows = []
        for encoder, f in r['full'].items():
            n, x = f['native'], f['remotion']
            rows.append([encoder, f'{x["wall_s"]:.0f} s ({x["fps"]:.0f} fps)', f'{n["wall_s"]:.0f} s ({n["fps"]:.0f} fps)', f'**{x["wall_s"] / n["wall_s"]:.1f}x**',
                         f'{x["cpu_s"]:.0f} / {n["cpu_s"]:.0f}', f'{x["peak_rss_mib"]:.0f} / {n["peak_rss_mib"]:.0f}'])
        print(table(['encoder', 'Remotion wall', 'native wall', 'speed-up', 'CPU-seconds (R / N)', 'peak RAM MiB (R / N)'], rows), '\n')

    if 'quality' in r:
        print('deterministic:', r['quality']['deterministic'])


main()
