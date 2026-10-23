import React from 'react';
import {AbsoluteFill} from 'remotion';
import {C, F, Film, Shot, cueAt, motion, useGsap} from '../core';
import {Board, halo} from '../diagrams/kit';

/**
 * Comparison on one shared scale. items="One tower:12;Four towers:45", unit=m.
 * Every bar starts at the same baseline and shares the same px-per-unit, so length is the argument.
 * Bar i lands on cue i (the scene compiler guarantees the cue names); `ref` draws a reference line.
 */
export const CompareShot: React.FC<{shot: Shot; film: Film}> = ({shot}) => {
  const p = shot.params;
  const items = (p.items || '').split(';').map(s => { const [label, v] = s.split(':'); return {label: label.trim(), value: Number(v)}; });
  const unit = p.unit || '';
  const max = Math.max(...items.map(i => i.value));
  const left = 470, right = 1700, top = 330, rowH = Math.min(190, 560 / items.length);
  const px = (right - left) / (max * 1.0);
  const ref = p.ref ? {value: Number(p.ref.split(':')[1]), label: p.ref.split(':')[0]} : undefined;
  const scope = useGsap(ctx => {
    const m = motion(ctx);
    if (p.kicker) m.chars('.kicker', 0.15, 0.018);
    m.lines('.cmp-title', 0.25, 0.07, 0.8);
    items.forEach((_, i) => {
      const at = Math.max(0.3, cueAt(shot, i, 0.6 + 1.2 * i) - 0.1);
      m.fadeUp(`.row-${i} .lab`, at - 0.1, 10, 0.4);
      ctx.tl.fromTo(ctx.q(`.row-${i} .bar`), {scaleX: 0}, {scaleX: 1, duration: 1.0, ease: 'power3.out', transformOrigin: '0% 50%', immediateRender: true}, at);
      m.fadeUp(`.row-${i} .val`, at + 0.7, 8, 0.4);
    });
    if (ref) m.draw('.ref-line', Math.max(0.3, cueAt(shot, items.length, 0.6 + 1.2 * items.length) - 0.1), 0.8);
  });
  return <AbsoluteFill ref={scope}>
    <Board tag={p.tag} legend={p.legend || 'SAME SCALE · BARS START AT ZERO'}>
      <AbsoluteFill>
        {p.kicker && <div className="kicker" style={{position: 'absolute', left: 150, top: 150, font: `500 26px ${F.mono}`, letterSpacing: 4, color: C.terracotta}}>{p.kicker}</div>}
        <div className="cmp-title" style={{position: 'absolute', left: 150, top: 195, font: `500 84px/1.04 ${F.display}`, letterSpacing: -2, color: C.ink,
          fontVariationSettings: '"opsz" 144, "SOFT" 40'}}>{p.title}</div>
        <svg width={1920} height={1080} style={{position: 'absolute', inset: 0}}>
          <path d={`M${left} ${top - 30} V${top + items.length * rowH}`} stroke={C.ink} strokeWidth={4} />
          {items.map((it, i) => {
            const y = top + i * rowH, w = it.value * px, hot = i === items.length - 1;
            return <g key={i} className={`row-${i}`}>
              <text className="lab" x={left - 28} y={y + rowH * 0.42} textAnchor="end" style={{font: `600 38px ${F.sans}`, letterSpacing: -0.5, fill: C.ink, opacity: 0}}>{it.label}</text>
              <rect className="bar" x={left} y={y + 14} width={w} height={rowH - 56} rx={4} fill={hot ? C.terracotta : C.ochre} stroke={C.ink} strokeWidth={3} />
              <text className="val" x={left + w + 22} y={y + rowH * 0.42} style={{font: `700 56px ${F.display}`, fill: hot ? C.terracotta : C.ink, opacity: 0, ...halo()}}>
                {it.value}<tspan style={{font: `600 30px ${F.sans}`, fill: C.inkSoft}}> {unit}</tspan></text>
            </g>;
          })}
          {ref && <g>
            <path className="ref-line" d={`M${left + ref.value * px} ${top - 30} V${top + items.length * rowH}`} stroke={C.teal} strokeWidth={4} strokeDasharray="14 10" />
            <text x={left + ref.value * px + 14} y={top - 40} style={{font: `500 24px ${F.mono}`, letterSpacing: 2, fill: C.teal, ...halo()}}>{ref.label} · {ref.value} {unit}</text>
          </g>}
        </svg>
        {p.note && <div style={{position: 'absolute', left: 150, bottom: 70, font: `500 28px ${F.sans}`, color: C.inkSoft, maxWidth: 1200}}>{p.note}</div>}
      </AbsoluteFill>
    </Board>
  </AbsoluteFill>;
};
