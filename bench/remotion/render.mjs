// The Remotion reference renderer for the benchmark and parity suites. It draws the same
// film.json as the native renderer, with the same output spec.
//
//   node render.mjs video  <film.json> --public=DIR --assets=DIR --out=FILE [options]
//   node render.mjs stills <film.json> --public=DIR --assets=DIR --dir=DIR [--shots=s001,s002] [--at=SECONDS]
//
// video options:
//   --from=0 --to=<end>      frame range, end exclusive (as in turbomotion)
//   --browsers=1             parallel Chrome instances, each rendering one slice of the range
//   --concurrency=1          Remotion tabs per browser (renderMedia `concurrency`)
//   --codec=x264|hardware    libx264 (--crf, --preset) or VideoToolbox (--bitrate)
//   --crf=17 --preset=faster --bitrate=20M
// Prints one JSON line of timings, in seconds.
import {bundle} from '@remotion/bundler';
import {openBrowser, renderMedia, renderStill, selectComposition} from '@remotion/renderer';
import {execFileSync} from 'node:child_process';
import {cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {serveBundle} from './serve-bundle.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const [mode, filmPath] = process.argv.slice(2);
const flag = (name, fallback) => process.argv.find(a => a.startsWith(`--${name}=`))?.split('=').slice(1).join('=') ?? fallback;
const film = JSON.parse(readFileSync(filmPath, 'utf8'));
const pub = resolve(flag('public'));
const assets = resolve(flag('assets'));
const seconds = t0 => (performance.now() - t0) / 1000;

// The bundler serves one `public/` directory: the project's, plus the shared fonts, grain and paper.
const staging = mkdtempSync(join(tmpdir(), 'remotion-public-'));
for (const entry of ['photos', 'depth', 'illustrations']) if (existsSync(join(pub, entry))) symlinkSync(join(pub, entry), join(staging, entry));
mkdirSync(join(staging, '_engine'));
for (const entry of ['fonts', 'grain', 'paper.jpg']) cpSync(join(assets, entry), join(staging, '_engine', entry), {recursive: true});

// The installed Google Chrome with ANGLE (Metal) is used because Remotion's bundled headless shell
// does not start on every Mac; WebGL (the photo depth effect) needs it either way.
const CHROME = process.env.REMOTION_CHROME || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const browserOptions = existsSync(CHROME) ? {browserExecutable: CHROME, chromeMode: 'chrome-for-testing'} : {};
const chromiumOptions = {gl: 'angle'};
const inputProps = {film};
const timings = {};

let t = performance.now();
const bundled = await bundle({entryPoint: join(HERE, 'src/index.tsx'), publicDir: staging});
const server = await serveBundle(bundled);
timings.bundle = seconds(t);

const slices = (from, to, n) => Array.from({length: n}, (_, i) => [from + Math.floor(((to - from) * i) / n), from + Math.floor(((to - from) * (i + 1)) / n) - 1]);

try {
  t = performance.now();
  const browsers = await Promise.all(Array.from({length: Number(flag('browsers', 1))}, () => openBrowser('chrome', {...browserOptions, chromiumOptions})));
  const compositions = await Promise.all(browsers.map(b => selectComposition({serveUrl: server.url, id: 'Film', inputProps, puppeteerInstance: b, ...browserOptions, chromiumOptions})));
  timings.browser = seconds(t);

  if (mode === 'stills') {
    const only = flag('shots')?.split(',');
    const fixed = flag('at');
    const dir = resolve(flag('dir')); mkdirSync(dir, {recursive: true});
    for (const s of film.shots.filter(s => !only || only.includes(s.id))) {
      // Default: settled, 1.3 s after the last cue (the same frame the native `stills` picks).
      const lastCue = s.cueTimes.length ? Math.max(...s.cueTimes) + 1.3 : s.duration * 0.72;
      const at = Math.min(s.duration - 0.1, fixed !== undefined ? Number(fixed) : Math.max(lastCue, s.duration * 0.45));
      await renderStill({serveUrl: server.url, composition: compositions[0], inputProps, puppeteerInstance: browsers[0], frame: s.startFrame + Math.floor(at * film.fps),
        output: join(dir, `${s.id}.png`), imageFormat: 'png', ...browserOptions, chromiumOptions});
    }
  } else if (mode === 'video') {
    const out = resolve(flag('out'));
    const from = Number(flag('from', 0)), to = Number(flag('to', film.durationInFrames));
    const hardware = flag('codec', 'x264') === 'hardware';
    const codecOptions = hardware
      ? {hardwareAcceleration: 'required', videoBitrate: flag('bitrate', '20M')}
      : {crf: Number(flag('crf', 17)), x264Preset: flag('preset', 'faster')};
    const parts = slices(from, to, browsers.length).map(([a, b], i) => ({a, b, file: out.replace(/\.mp4$/, `.part${i}.mp4`)}));
    t = performance.now();
    await Promise.all(parts.map((part, i) => renderMedia({
      serveUrl: server.url, composition: compositions[i], inputProps, puppeteerInstance: browsers[i], frameRange: [part.a, part.b],
      codec: 'h264', pixelFormat: 'yuv420p', imageFormat: 'jpeg', jpegQuality: 95, muted: true, concurrency: Number(flag('concurrency', 1)),
      ...codecOptions, ...browserOptions, chromiumOptions, outputLocation: part.file,
    })));
    timings.render = seconds(t);
    t = performance.now();
    if (parts.length === 1) execFileSync('mv', [parts[0].file, out]);
    else {
      const list = out.replace(/\.mp4$/, '.concat.txt');
      writeFileSync(list, parts.map(p => `file '${p.file}'`).join('\n') + '\n');
      execFileSync('ffmpeg', ['-v', 'error', '-y', '-f', 'concat', '-safe', '0', '-i', list, '-c', 'copy', out]);
      rmSync(list); parts.forEach(p => rmSync(p.file));
    }
    timings.concat = seconds(t);
    timings.frames = to - from;
  } else throw new Error('mode must be video or stills');
  await Promise.all(browsers.map(b => b.close({silent: true})));
} finally {
  await server.close();
  rmSync(staging, {recursive: true, force: true});
}
console.log(JSON.stringify(timings));
// Like Remotion's own CLI, leave explicitly: lingering browser helpers can keep Node alive for minutes.
process.exit(0);
