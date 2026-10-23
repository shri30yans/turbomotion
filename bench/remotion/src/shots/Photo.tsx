import React from 'react';
import {AbsoluteFill, useCurrentFrame, useVideoConfig} from 'remotion';
import {C, F, Film, Shot, camera, cueAt, easeCam, lerp, motion, num, triple, useGsap} from '../core';
import {DepthImage} from '../ui/DepthImage';

/** Parse "x,y,Label;x,y,Label" pins (image-normalised coordinates). */
const parsePins = (v?: string) => (v ? v.split(';').map(p => {
  const [x, y, ...rest] = p.split(',');
  return {x: Number(x), y: Number(y), label: rest.join(',').trim()};
}) : []);

const badgeColor = (b: string) => (/LEGEND|STORY/.test(b) ? C.terracotta : /RECONSTRUCTION|ILLUSTRAT/.test(b) ? C.ochre : 'rgba(20,14,10,.62)');

export function useCamera(shot: Shot, ar: number, W: number, H: number, hasDepth: boolean) {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const t = Math.min(1, frame / fps / Math.max(0.5, shot.duration));
  const [fx, fy, fz] = triple(shot.params.from, [0.5, 0.5, 1.12]);
  const [tx, ty, tz] = triple(shot.params.to, [0.5, 0.5, 1.0]);
  const k = easeCam(t);
  const margin = hasDepth ? 1.09 : 1.0; // room for parallax displacement at the frame edges
  const cx = lerp(fx, tx, k), cy = lerp(fy, ty, k), z = lerp(fz, tz, k) * margin;
  const rect = camera(ar, W, H, cx, cy, z);
  // Parallax: lateral drift follows the pan; dolly follows the zoom direction.
  const dir: [number, number] = [tx - fx, ty - fy];
  const len = Math.hypot(dir[0], dir[1]);
  const lateral = len > 0.02 ? [dir[0] / len, dir[1] / len] : [1, 0];
  const amp = hasDepth ? 0.016 : 0;
  const drift = (k - 0.5) * 2;
  const shift: [number, number] = [lateral[0] * amp * drift, lateral[1] * amp * drift * 0.6];
  const dolly = hasDepth ? Math.sign(tz - fz || 0.0001) * 0.07 * k : 0;
  return {rect, shift, dolly, focus: [cx, cy] as [number, number], t, k};
}

export const PhotoShot: React.FC<{shot: Shot; film: Film; grade?: 'warm' | 'archival' | 'painting'}> = ({shot, film, grade}) => {
  const {width: W, height: H} = film;
  const id = shot.args[0];
  const asset = film.assets[id];
  const p = shot.params;
  const g = (p.grade as 'warm' | 'archival' | 'painting') || grade || (asset?.kind === 'illustration' || asset?.kind === 'generated' ? 'painting' : 'warm');
  const useDepth = p.depth === '1' && !!asset?.depth;
  const ar = asset ? asset.width / asset.height : 16 / 9;
  const cam = useCamera(shot, ar, W, H, useDepth);
  const pins = parsePins(p.pins);
  const scope = useGsap(ctx => {
    const {tl} = ctx;
    const m = motion(ctx);
    if (p.tag) m.fadeUp('.tag', 0.25, 12);
    if (p.badge) m.fadeUp('.badge', 0.4, -10);
    if (p.hero) m.chars('.hero', cueAt(shot, 0, 0.6) - 0.15, 0.04);
    const titleAt = cueAt(shot, 0, 0.6);
    if (p.title) m.lines('.title', titleAt - 0.1, 0.08, 0.8);
    if (p.note) m.fadeUp('.note', p.title ? titleAt + 0.45 : Math.max(0.6, titleAt), 14);
    if (p.title || p.note || p.hero) m.show('.scrim', 0, 0.8);
    pins.forEach((_, i) => {
      const at = cueAt(shot, i + (p.pinsFrom ? Number(p.pinsFrom) : (shot.cues > pins.length ? shot.cues - pins.length : 0)));
      m.pop(`.pin-${i} .dot`, at);
      m.draw(`.pin-${i} .lead`, at + 0.08, 0.45);
      m.fadeUp(`.pin-${i} .plabel`, at + 0.3, 10, 0.45);
      tl.fromTo(ctx.q(`.pin-${i} .ring`), {scale: 0.4, opacity: 0.9}, {scale: 2.6, opacity: 0, duration: 1.2, ease: 'power2.out', transformOrigin: '50% 50%', immediateRender: true}, at);
    });
  });
  if (!asset) return <Missing id={id} />;
  return <AbsoluteFill ref={scope} style={{background: C.night, overflow: 'hidden'}}>
    <DepthImage src={asset.file} depth={useDepth ? asset.depth : undefined} width={W} height={H} rect={cam.rect}
      shift={cam.shift} dolly={cam.dolly} focus={cam.focus} grade={g} dim={num(p.dim, 0)} />
    <div className="scrim" style={{position: 'absolute', inset: 0, opacity: 0,
      background: 'linear-gradient(0deg, rgba(14,9,6,.78) 0%, rgba(14,9,6,.35) 32%, rgba(14,9,6,0) 55%)'}} />
    <svg width={W} height={H} style={{position: 'absolute', inset: 0, overflow: 'visible'}}>
      {pins.map((pin, i) => {
        const x = cam.rect.x + pin.x * cam.rect.w, y = cam.rect.y + pin.y * cam.rect.h;
        const right = x < W * 0.62;
        // Alternate labels above/below so neighbouring pins never collide.
        const lx = x + (right ? 120 : -120), ly = i % 2 === 0 ? y - 90 : y + 100;
        return <g key={i} className={`pin-${i}`}>
          <circle className="ring" cx={x} cy={y} r={14} fill="none" stroke={C.white} strokeWidth={3} style={{opacity: 0}} />
          <path className="lead" d={`M${x} ${y} L${lx} ${ly} L${lx + (right ? 40 : -40)} ${ly}`} fill="none" stroke={C.white} strokeWidth={2.5} />
          <circle className="dot" cx={x} cy={y} r={9} fill={C.ochre} stroke={C.night} strokeWidth={3} style={{opacity: 0}} />
          <foreignObject x={right ? lx + 48 : lx - 48 - 560} y={ly - 30} width={560} height={70}>
            <div className="plabel" style={{opacity: 0, display: 'flex', justifyContent: right ? 'flex-start' : 'flex-end'}}>
              <span style={{font: `600 30px ${F.sans}`, color: C.night, background: C.white, padding: '8px 16px', borderRadius: 6,
                boxShadow: '0 6px 24px rgba(0,0,0,.35)', letterSpacing: '-0.3px', whiteSpace: 'nowrap'}}>{pin.label}</span>
            </div>
          </foreignObject>
        </g>;
      })}
    </svg>
    {p.tag && <Tag text={p.tag} />}
    {p.badge && <div className="badge" style={{position: 'absolute', right: 72, top: 64, opacity: 0, font: `500 19px ${F.mono}`, letterSpacing: 1.6,
      color: C.white, background: badgeColor(p.badge), padding: '9px 16px', borderRadius: 4, border: '1px solid rgba(255,248,236,.35)',
      backdropFilter: 'blur(6px)'}}>{p.badge}</div>}
    {p.hero && <div className="hero" style={{position: 'absolute', left: 110, bottom: 150, font: `600 220px/0.95 ${F.display}`,
      letterSpacing: -6, color: C.white, textShadow: '0 10px 50px rgba(0,0,0,.45)', fontVariationSettings: '"SOFT" 50, "opsz" 144'}}>{p.hero}</div>}
    {(p.title || p.note) && <div style={{position: 'absolute', left: 110, bottom: 104, width: 1250}}>
      {p.title && <div className="title" style={{font: `500 84px/1.04 ${F.display}`, letterSpacing: -2, color: C.white,
        textShadow: '0 6px 30px rgba(0,0,0,.4)', fontVariationSettings: '"opsz" 120'}}>{p.title}</div>}
      {p.note && <div className="note" style={{opacity: 0, marginTop: 22, display: 'flex', gap: 18, alignItems: 'stretch', maxWidth: 1100}}>
        <span style={{width: 5, background: C.ochre, borderRadius: 2}} />
        <span style={{font: `500 32px/1.35 ${F.sans}`, color: 'rgba(255,248,236,.92)', letterSpacing: '-0.3px'}}>{p.note}</span>
      </div>}
    </div>}
  </AbsoluteFill>;
};

export const Tag: React.FC<{text: string; dark?: boolean}> = ({text, dark}) =>
  <div className="tag" style={{position: 'absolute', left: 72, top: 64, opacity: 0, display: 'flex', alignItems: 'center', gap: 14,
    font: `500 21px ${F.mono}`, letterSpacing: 2.2, color: dark ? C.ink : C.white,
    textShadow: dark ? 'none' : '0 2px 12px rgba(0,0,0,.55)'}}>
    <span style={{width: 12, height: 12, background: C.terracotta, display: 'inline-block'}} />{text}
  </div>;

export const Missing: React.FC<{id: string}> = ({id}) =>
  <AbsoluteFill style={{background: '#3a0d0d', color: 'white', font: `40px ${F.mono}`, alignItems: 'center', justifyContent: 'center'}}>
    MISSING ASSET: {id}
  </AbsoluteFill>;
