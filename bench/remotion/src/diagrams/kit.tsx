import React from 'react';
import {AbsoluteFill, staticFile, useCurrentFrame} from 'remotion';
import {C, Ctx, F, Shot, cueAt, motion} from '../core';

/**
 * Step/cue choreography for multi-shot diagrams.
 *   data-s="2:1"     element belongs to step 2 and enters on that shot's cue 1
 *   data-s="2:t1.4"  …enters 1.4 s into the shot
 *   data-s="2:1-0.8" …enters 0.8 s before cue 1 (also +)
 *   data-a="draw|pop|fade|up|lines"   entrance (default fade)
 * Elements from earlier steps are already present; later steps stay hidden.
 */
/** "3" | "3-0.3" | "t1.4" | "payoff" | "payoff+0.6": cue by position or by name, with an optional offset. */
export function whenCue(shot: Shot, c?: string) {
  if (c?.startsWith('t') && /^t[\d.]+$/.test(c)) return Math.max(0.05, Number(c.slice(1)));
  const m = (c || '0').match(/^(\d+|[a-z][a-z0-9_]*)([+-][\d.]+)?$/);
  if (!m) throw new Error(`${shot.id}: bad step time "${c}"`);
  return Math.max(0.05, cueAt(shot, /^\d+$/.test(m[1]) ? Number(m[1]) : m[1]) + (m[2] ? Number(m[2]) : 0));
}

export function stepReveal(ctx: Ctx, shot: Shot, step: number) {
  const m = motion(ctx);
  // data-hide="step:cue" fades an element out (e.g. a name replaced by a corrected one).
  ctx.q('[data-hide]').forEach(el => {
    const [s, c] = (el.getAttribute('data-hide') || '1:0').split(':');
    const st = Number(s);
    if (st < step) { ctx.tl.set(el, {opacity: 0}, 0); return; }
    if (st > step) return;
    const at = whenCue(shot, c);
    ctx.tl.to(el, {opacity: 0, duration: 0.3}, at);
  });
  ctx.q('[data-s]').forEach(el => {
    const [s, c] = (el.getAttribute('data-s') || '1:0').split(':');
    const st = Number(s);
    const anim = el.getAttribute('data-a') || 'fade';
    if (st > step) { ctx.tl.set(el, {opacity: 0}, 0); return; }
    if (st < step) return;
    const at = whenCue(shot, c);
    if (anim === 'draw') m.draw([el], at, Number(el.getAttribute('data-d') || 0.9));
    else if (anim === 'pop') m.pop([el], at);
    else if (anim === 'up') m.fadeUp([el], at, 16);
    else m.show([el], at, 0.5);
  });
}

export const Board: React.FC<{dark?: boolean; tag?: string; legend?: string; children: React.ReactNode}> = ({dark, tag, legend, children}) => {
  const frame = useCurrentFrame();
  return <AbsoluteFill style={{background: dark ? '#191310' : C.paper}}>
    <AbsoluteFill style={{backgroundImage: `url(${staticFile('_engine/paper.jpg')})`, backgroundSize: 'cover',
      mixBlendMode: dark ? 'screen' : 'multiply', opacity: dark ? 0.06 : 0.5, transform: `scale(${1.03 + frame * 0.00003})`}} />
    <AbsoluteFill style={{background: dark ? 'radial-gradient(ellipse at 50% 45%, rgba(224,169,59,.08), rgba(0,0,0,.55))'
      : 'radial-gradient(ellipse at 50% 42%, rgba(255,250,240,.45), rgba(120,90,50,.16))'}} />
    {children}
    {tag && <div style={{position: 'absolute', left: 72, top: 60, font: `500 21px ${F.mono}`, letterSpacing: 2.2, color: dark ? C.white : C.ink,
      display: 'flex', gap: 14, alignItems: 'center'}}><span style={{width: 12, height: 12, background: C.terracotta}} />{tag}</div>}
    {legend && <div style={{position: 'absolute', right: 72, bottom: 56, font: `500 17px ${F.mono}`, letterSpacing: 2, color: dark ? 'rgba(255,248,236,.55)' : '#8C7A60'}}>{legend}</div>}
  </AbsoluteFill>;
};

export const label = (size = 34, color: string = C.ink, weight = 600): React.CSSProperties =>
  ({font: `${weight} ${size}px ${F.sans}`, letterSpacing: -0.4, fill: color});
export const mono = (size = 22, color: string = C.inkSoft): React.CSSProperties =>
  ({font: `500 ${size}px ${F.mono}`, letterSpacing: 2, fill: color});
export const halo = (c = C.paper): React.CSSProperties => ({paintOrder: 'stroke', stroke: c, strokeWidth: 8, strokeLinejoin: 'round'});

/** Dimension line with end ticks and a centred label. */
export const Dim: React.FC<{x1: number; y1: number; x2: number; y2: number; text: string; s?: string; color?: string; off?: number}> = ({x1, y1, x2, y2, text, s, color = C.terracotta, off = -16}) => {
  const ang = Math.atan2(y2 - y1, x2 - x1);
  const nx = -Math.sin(ang) * 14, ny = Math.cos(ang) * 14;
  return <g data-s={s} data-a="fade">
    <path d={`M${x1} ${y1} L${x2} ${y2} M${x1 + nx} ${y1 + ny} L${x1 - nx} ${y1 - ny} M${x2 + nx} ${y2 + ny} L${x2 - nx} ${y2 - ny}`} stroke={color} strokeWidth={3} fill="none" />
    <text x={(x1 + x2) / 2} y={(y1 + y2) / 2 + off} textAnchor="middle" style={{...label(30, color, 700), ...halo()}}>{text}</text>
  </g>;
};
