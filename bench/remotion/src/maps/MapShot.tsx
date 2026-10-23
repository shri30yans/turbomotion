import React from 'react';
import {AbsoluteFill, staticFile, useCurrentFrame, useVideoConfig} from 'remotion';
import {C, F, Film, Shot, cueAt} from '../core';
import geo from '../../../../assets/maps/geo.json';
import {MAPS, MapSpec, PLACES} from './views';

type G = {countries: Record<string, number[][][]>; states: Record<string, number[][][]>; rivers: Record<string, number[][][]>};
const GEO = geo as unknown as G;

const ease = (x: number) => (x <= 0 ? 0 : x >= 1 ? 1 : 1 - Math.pow(1 - x, 3));
const smooth = (x: number) => (x <= 0 ? 0 : x >= 1 ? 1 : x * x * (3 - 2 * x));

/** Resolve a spec time: number = seconds, "cN" = cue N, "cN+0.4" = cue N plus offset. */
function when(shot: Shot, v: number | string | undefined, d = 0.3) {
  if (v === undefined) return d;
  if (typeof v === 'number') return v;
  const m = v.match(/^(?:c(\d+)|@([a-z][a-z0-9_]*))([+-][\d.]+)?$/);
  if (!m) return d;
  return cueAt(shot, m[1] !== undefined ? Number(m[1]) : m[2]) + (m[3] ? Number(m[3]) : 0);
}

export const MapShot: React.FC<{shot: Shot; film: Film}> = ({shot, film}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const t = frame / fps;
  const W = film.width, H = film.height;
  const spec: MapSpec | undefined = MAPS[shot.args[0]];
  if (!spec) return <AbsoluteFill style={{background: '#300', color: '#fff', font: `40px ${F.mono}`}}>UNKNOWN MAP {shot.args[0]}</AbsoluteFill>;
  const step = Number(shot.params.step || (shot.params.rings ? 2 : 1));
  // A step's elements animate on their cues; elements from earlier steps are already present.
  const at = (el: {step?: number; at?: number | string}) => ((el.step ?? 1) < step ? -10 : (el.step ?? 1) > step ? 1e9 : when(shot, el.at));

  // Camera: interpolate between keyed views.
  const views = (spec.views.filter(v => (v.step ?? step) === step).length ? spec.views.filter(v => (v.step ?? step) === step) : spec.views);
  let center = views[0].center, span = views[0].span;
  for (let i = 1; i < views.length; i++) {
    const a = views[i - 1], b = views[i];
    const t0 = when(shot, b.at, 1), dur = b.dur ?? 2.2;
    const k = smooth((t - t0) / dur);
    if (k > 0) {
      center = [a.center[0] + (b.center[0] - a.center[0]) * k, a.center[1] + (b.center[1] - a.center[1]) * k];
      span = Math.exp(Math.log(a.span) + (Math.log(b.span) - Math.log(a.span)) * k);
    }
  }
  const drift = 1 + 0.02 * Math.min(1, t / shot.duration);
  const cos = Math.cos((center[1] * Math.PI) / 180);
  const s = W / (span / drift);
  const P = (lon: number, lat: number): [number, number] => [W / 2 + (lon - center[0]) * cos * s, H / 2 - (lat - center[1]) * s];
  const ring = (r: number[][]) => 'M' + r.map(([x, y]) => P(x, y).map(v => v.toFixed(1)).join(' ')).join('L') + 'Z';
  const line = (r: number[][]) => 'M' + r.map(([x, y]) => P(x, y).map(v => v.toFixed(1)).join(' ')).join('L');
  const kmPx = s / 111.3;

  const landPaths = Object.entries(GEO.countries).map(([name, rs]) =>
    <path key={name} d={rs.map(ring).join(' ')} fill={spec.home?.includes(name) ? '#EFE3C8' : '#E2D5B9'} stroke="#A99475" strokeWidth={1.6} strokeLinejoin="round" />);
  const statePaths = Object.entries(GEO.states).map(([name, rs]) => {
    const hl = spec.areas?.find(a => a.state === name);
    const k = hl ? ease((t - at(hl)) / 0.8) : 0;
    return <path key={name} d={rs.map(ring).join(' ')} fill={hl ? `rgba(224,169,59,${0.32 * k})` : 'none'}
      stroke={hl && k > 0 ? C.ochre : '#C2B08F'} strokeWidth={hl && k > 0 ? 2.5 : 1} strokeDasharray={hl ? undefined : '5 6'} />;
  });
  const rivers = Object.entries(GEO.rivers).filter(([n]) => ['Ganges', 'Indus', 'Sutlej', 'Chenab', 'Narmada', 'Brahmaputra'].includes(n))
    .map(([n, rs]) => <path key={n} d={rs.map(line).join(' ')} fill="none" stroke="#86A9B4" strokeWidth={2.4} strokeLinecap="round" opacity={0.85} />);

  return <AbsoluteFill style={{background: '#CFDAD2', overflow: 'hidden'}}>
    <svg width={W} height={H} style={{position: 'absolute', inset: 0}}>
      <defs><filter id="soft" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="14" /></filter></defs>
      {landPaths}{statePaths}{rivers}
      {spec.schematic?.map((el, i) => {
        const k = ease((t - at(el)) / 0.9);
        if (k <= 0) return null;
        if (el.kind === 'hills') return <g key={i} opacity={k} filter="url(#soft)">{el.pts.map((pt, j) => {
          const [x, y] = P(pt[0], pt[1]); const r = (pt[2] || 1.2) * kmPx;
          return <g key={j}><ellipse cx={x} cy={y} rx={r * 1.2} ry={r * 0.75} fill="#C9B488" opacity={0.42} />
            <ellipse cx={x - r * 0.15} cy={y - r * 0.12} rx={r * 0.6} ry={r * 0.36} fill="#B39A6C" opacity={0.3} /></g>;
        })}</g>;
        if (el.kind === 'lake') return <path key={i} d={ring(el.pts)} fill="#8DB3C0" stroke="#5F8D9D" strokeWidth={2} opacity={k} />;
        if (el.kind === 'grid') {
          const [a, b] = [P(el.pts[0][0], el.pts[0][1]), P(el.pts[1][0], el.pts[1][1])];
          const x0 = Math.min(a[0], b[0]), x1 = Math.max(a[0], b[0]), y0 = Math.min(a[1], b[1]), y1 = Math.max(a[1], b[1]);
          const n = 3, cells = [];
          for (let gx = 0; gx < n; gx++) for (let gy = 0; gy < n; gy++) {
            const kk = ease((t - at(el) - (gx + gy) * 0.12) / 0.5);
            cells.push(<rect key={`${gx}${gy}`} x={x0 + (x1 - x0) * gx / n + 4} y={y0 + (y1 - y0) * gy / n + 4} width={(x1 - x0) / n - 8} height={(y1 - y0) / n - 8}
              fill={gx === 1 && gy === 1 ? 'rgba(180,72,47,.55)' : 'rgba(224,169,59,.45)'} stroke={C.terracotta} strokeWidth={2} opacity={kk} />);
          }
          return <g key={i}>{cells}<rect x={x0} y={y0} width={x1 - x0} height={y1 - y0} fill="none" stroke={C.ink} strokeWidth={4} opacity={k} /></g>;
        }
        return null;
      })}
      {spec.rings?.map((r, i) => {
        const k = ease((t - at(r)) / 1.0);
        if (k <= 0) return null;
        const [x, y] = P(r.center[0], r.center[1]);
        return <g key={i} opacity={k}>
          <circle cx={x} cy={y} r={r.km * kmPx * (0.85 + 0.15 * k)} fill="rgba(180,72,47,.10)" stroke={C.terracotta} strokeWidth={3} strokeDasharray="14 10" />
          {r.label && <text x={x} y={y - r.km * kmPx - 18} textAnchor="middle" style={{font: `500 24px ${F.mono}`, letterSpacing: 3, fill: C.terracotta}}>{r.label}</text>}
        </g>;
      })}
      {spec.routes?.map((r, i) => {
        const pts = r.via.map(id => PLACES[id]);
        const t0 = at(r), k = ease((t - t0) / (r.dur ?? 1.6));
        if (k <= 0) return null;
        // Great-circle-free curved route: quadratic between successive points, sampled.
        const samples: [number, number][] = [];
        for (let j = 0; j < pts.length - 1; j++) {
          const a = P(pts[j][0], pts[j][1]), b = P(pts[j + 1][0], pts[j + 1][1]);
          const mx = (a[0] + b[0]) / 2, my = (a[1] + b[1]) / 2, dx = b[0] - a[0], dy = b[1] - a[1];
          const bend = r.bend ?? 0.18;
          const cx = mx - dy * bend, cy = my + dx * bend;
          for (let q = 0; q <= 40; q++) {
            const u = q / 40;
            samples.push([(1 - u) * (1 - u) * a[0] + 2 * (1 - u) * u * cx + u * u * b[0], (1 - u) * (1 - u) * a[1] + 2 * (1 - u) * u * cy + u * u * b[1]]);
          }
        }
        const n = Math.max(2, Math.round(samples.length * k));
        const vis = samples.slice(0, n);
        const head = vis[vis.length - 1], prev = vis[Math.max(0, vis.length - 4)];
        const ang = Math.atan2(head[1] - prev[1], head[0] - prev[0]) * 180 / Math.PI;
        const col = r.color === 'teal' ? C.teal : C.terracotta;
        return <g key={i}>
          <path d={'M' + vis.map(p => p.map(v => v.toFixed(1)).join(' ')).join('L')} fill="none" stroke={col} strokeWidth={6}
            strokeDasharray={r.dashed ? '16 12' : undefined} strokeLinecap="round" />
          <path d="M-16 -11 L4 0 L-16 11 Z" fill={col} transform={`translate(${head[0]} ${head[1]}) rotate(${ang})`} />
          {r.label && k > 0.6 && (() => {
            const m = vis[Math.floor(vis.length / 2)];
            return <text x={m[0] + (r.labelDx ?? 0)} y={m[1] - 18 + (r.labelDy ?? 0)} textAnchor="middle" opacity={ease((k - 0.6) / 0.4)}
              style={{font: `600 28px ${F.sans}`, fill: col, paintOrder: 'stroke', stroke: '#F1E7D2', strokeWidth: 7}}>{r.label}</text>;
          })()}
        </g>;
      })}
      {spec.pins?.map((pin, i) => {
        const k = ease((t - at(pin)) / 0.5);
        if (k <= 0) return null;
        const [lon, lat] = PLACES[pin.place];
        const [x, y] = P(lon, lat);
        const big = pin.style === 'home';
        const pulse = big ? ((t - at(pin)) % 2) / 2 : 0;
        const dx = pin.dx ?? 22, dy = pin.dy ?? -18, anchor = dx < 0 ? 'end' : 'start';
        return <g key={i} opacity={k}>
          {big && <circle cx={x} cy={y} r={14 + 40 * pulse} fill="none" stroke={C.terracotta} strokeWidth={3} opacity={1 - pulse} />}
          <circle cx={x} cy={y} r={(big ? 13 : 9) * (0.6 + 0.4 * k)} fill={big ? C.terracotta : C.ink} stroke="#F7EEDC" strokeWidth={4} />
          <text x={x + dx} y={y + dy} textAnchor={anchor} style={{font: `${big ? 700 : 600} ${big ? 44 : 34}px ${F.sans}`, letterSpacing: -0.5, fill: C.ink,
            paintOrder: 'stroke', stroke: '#F4EBD9', strokeWidth: 9, strokeLinejoin: 'round'}}>{pin.label ?? pin.place}</text>
          {pin.sub && <text x={x + dx} y={y + dy + 34} textAnchor={anchor} style={{font: `500 22px ${F.mono}`, letterSpacing: 1.5, fill: C.terracotta,
            paintOrder: 'stroke', stroke: '#F4EBD9', strokeWidth: 7}}>{pin.sub}</text>}
        </g>;
      })}
      {spec.labels?.map((l, i) => {
        const k = ease((t - at(l)) / 0.8);
        const [x, y] = P(l.at_[0], l.at_[1]);
        return <text key={i} x={x} y={y} textAnchor="middle" opacity={k * (l.faint ? 0.55 : 0.9)} style={{font: `500 ${l.size ?? 24}px ${F.mono}`, letterSpacing: 6, fill: '#7A6A52'}}>{l.text}</text>;
      })}
    </svg>
    <AbsoluteFill style={{backgroundImage: `url(${staticFile('_engine/paper.jpg')})`, backgroundSize: 'cover', mixBlendMode: 'multiply', opacity: 0.45}} />
    <AbsoluteFill style={{boxShadow: 'inset 0 0 220px rgba(70,45,20,.35)'}} />
    {spec.title && <div style={{position: 'absolute', left: 72, top: 60, font: `500 21px ${F.mono}`, letterSpacing: 2.2, color: C.ink, display: 'flex', gap: 14, alignItems: 'center'}}>
      <span style={{width: 12, height: 12, background: C.terracotta}} />{spec.title}</div>}
    {(shot.params.note || spec.note) && <div style={{position: 'absolute', left: 72, bottom: 58, font: `500 26px ${F.sans}`, color: C.inkSoft,
      opacity: ease((t - 0.8) / 0.6), maxWidth: 1100}}>{shot.params.note || spec.note}</div>}
    <div style={{position: 'absolute', right: 72, bottom: 60, font: `500 17px ${F.mono}`, letterSpacing: 2, color: '#8C7A60'}}>{spec.legend ?? 'MODERN BORDERS SHOWN FOR ORIENTATION'}</div>
  </AbsoluteFill>;
};
