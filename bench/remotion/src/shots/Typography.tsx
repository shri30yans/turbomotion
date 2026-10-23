import React from 'react';
import {AbsoluteFill, Img, staticFile, useCurrentFrame, useVideoConfig} from 'remotion';
import {C, F, Film, Shot, cueAt, easeCam, lerp, motion, triple, useGsap} from '../core';
import {DepthImage} from '../ui/DepthImage';
import {Missing, Tag, useCamera} from './Photo';

const Paper: React.FC<{dark?: boolean}> = ({dark}) => {
  const frame = useCurrentFrame();
  return <AbsoluteFill style={{background: dark ? C.night : C.paper}}>
    <AbsoluteFill style={{backgroundImage: `url(${staticFile('_engine/paper.jpg')})`, backgroundSize: '1920px 1080px',
      opacity: dark ? 0.08 : 0.55, mixBlendMode: dark ? 'screen' : 'multiply', transform: `scale(${1.04 + frame * 0.00004})`}} />
    <AbsoluteFill style={{background: dark ? 'radial-gradient(ellipse at 50% 40%, rgba(255,240,210,.06), rgba(0,0,0,.5))'
      : 'radial-gradient(ellipse at 45% 40%, rgba(255,250,240,.5), rgba(120,90,50,.18))'}} />
  </AbsoluteFill>;
};

/** Big number that counts up on its cue, over a dimmed photograph. */
export const StatShot: React.FC<{shot: Shot; film: Film}> = ({shot, film}) => {
  const {fps} = useVideoConfig();
  const frame = useCurrentFrame();
  const p = shot.params;
  const asset = film.assets[shot.args[0]];
  const ar = asset ? asset.width / asset.height : 16 / 9;
  const cam = useCamera(shot, ar, film.width, film.height, false);
  // The count runs from the cut and lands exactly on the cue word, so short shots still finish.
  const at = 0.12;
  const land = Math.max(0.7, Math.min(cueAt(shot, 0, 0.9) + 0.1, shot.duration - 0.35));
  const value = Number(p.value);
  const decimals = (p.value.split('.')[1] || '').length;
  const k = Math.max(0, Math.min(1, (frame / fps - at) / (land - at)));
  const eased = 1 - Math.pow(1 - k, 3);
  const shown = (value * eased).toLocaleString('en-US', {minimumFractionDigits: decimals, maximumFractionDigits: decimals});
  const scope = useGsap(ctx => {
    const m = motion(ctx);
    m.fadeUp('.stat-num', at, 30, 0.6);
    m.fadeUp('.stat-label', land - 0.1, 16);
    m.draw('.stat-rule', land - 0.3, 0.7);
  });
  return <AbsoluteFill ref={scope} style={{background: C.night}}>
    {asset ? <DepthImage src={asset.file} width={film.width} height={film.height} rect={cam.rect} shift={[0, 0]} dolly={0}
      focus={cam.focus} grade="warm" dim={0.58} vignette={0.5} /> : <Missing id={shot.args[0]} />}
    <AbsoluteFill style={{justifyContent: 'center', paddingLeft: 150}}>
      <div className="stat-num" style={{opacity: 0, display: 'inline-flex', alignSelf: 'flex-start', alignItems: 'baseline', gap: 28, color: C.white}}>
        {p.prefix && <span style={{font: `300 150px ${F.display}`, color: C.ochre}}>{p.prefix}</span>}
        <span style={{font: `600 300px/0.9 ${F.display}`, letterSpacing: -10, fontVariantNumeric: 'tabular-nums',
          fontVariationSettings: '"opsz" 144, "SOFT" 30'}}>{shown}</span>
        <span style={{font: `600 84px ${F.sans}`, letterSpacing: -2, color: C.ochre}}>{p.unit}</span>
      </div>
      <svg width={900} height={14} style={{marginTop: 26}}><path className="stat-rule" d="M4 7 H896" stroke={C.ochre} strokeWidth={4} /></svg>
      <div className="stat-label" style={{opacity: 0, marginTop: 22, alignSelf: 'flex-start', font: `500 28px ${F.mono}`, letterSpacing: 3, color: 'rgba(255,248,236,.85)', textTransform: 'uppercase'}}>{p.label}</div>
    </AbsoluteFill>
  </AbsoluteFill>;
};

/** Kinetic typography card. text="line one|line two"; second line is the accent. */
export const CardShot: React.FC<{shot: Shot; film: Film}> = ({shot, film}) => {
  const p = shot.params;
  const lines = (p.text || '').split('|');
  const dark = p.tone === 'dark';
  const img = p.image ? film.assets[p.image] : undefined;
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const [ix, iy] = (p.focus || '0.5,0.5').split(',').map(Number);
  const scope = useGsap(ctx => {
    const m = motion(ctx);
    if (p.kicker) m.chars('.kicker', 0.15, 0.018);
    // Line 0 lands immediately; line i lands on cue i-1 (the beat that earns it).
    lines.forEach((_, i) => {
      const at = i === 0 ? 0.25 : (shot.cueTimes[i - 1] !== undefined ? shot.cueTimes[i - 1] - 0.1 : 0.25 + 0.85 * i);
      m.lines(`.line-${i}`, Math.max(0.1, at), 0.07, 0.8);
    });
    m.draw('.card-rule', 0.2, 1.0);
    if (ctx.q('.card-photo').length) ctx.tl.fromTo(ctx.q('.card-photo'), {clipPath: 'inset(100% 0% 0% 0%)'}, {clipPath: 'inset(0% 0% 0% 0%)', duration: 1.0, ease: 'power3.inOut', immediateRender: true}, 0.05);
  });
  return <AbsoluteFill ref={scope}>
    <Paper dark={dark} />
    {img && <div className="card-photo" style={{position: 'absolute', left: 1010, top: 110, width: 838, height: 860, overflow: 'hidden', borderRadius: 4,
      boxShadow: '0 30px 80px rgba(60,40,20,.35)'}}>
      <Img src={staticFile(img.file)} style={{width: '100%', height: '100%', objectFit: 'cover', objectPosition: `${ix * 100}% ${iy * 100}%`,
        transform: `scale(${Number(p.zoom || 1.06) + 0.05 * Math.min(1, frame / fps / shot.duration)})`, transformOrigin: `${ix * 100}% ${iy * 100}%`, filter: 'contrast(1.04) saturate(1.05) sepia(.08)'}} />
    </div>}
    <AbsoluteFill style={{justifyContent: 'center', padding: img ? '0 980px 0 150px' : '0 170px'}}>
      {p.kicker && <div className="kicker" style={{font: `500 26px ${F.mono}`, letterSpacing: 4, color: C.terracotta, marginBottom: 40}}>{p.kicker}</div>}
      {lines.map((l, i) => <div key={i} className={`line-${i}`} style={{
        font: `${i === 0 ? 500 : 400} ${img ? 92 : lines.length > 2 ? 104 : 128}px/1.04 ${F.display}`, letterSpacing: img ? -2.5 : -3.5,
        color: i === 0 ? (dark ? C.white : C.ink) : C.terracotta, fontStyle: i === 0 ? 'normal' : 'italic',
        fontVariationSettings: '"opsz" 144, "SOFT" 40', maxWidth: 1580}}>{l}</div>)}
      <svg width={260} height={10} style={{marginTop: 56}}><path className="card-rule" d="M2 5 H258" stroke={C.ochre} strokeWidth={6} strokeLinecap="round" /></svg>
    </AbsoluteFill>
  </AbsoluteFill>;
};

/** Historical artwork: shown whole on paper, with provenance, then the camera explores it. */
export const PaintingShot: React.FC<{shot: Shot; film: Film}> = ({shot, film}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const p = shot.params;
  const asset = film.assets[shot.args[0]];
  const scope = useGsap(ctx => {
    const m = motion(ctx);
    ctx.tl.fromTo(ctx.q('.frame'), {opacity: 0, y: 30, scale: 0.97}, {opacity: 1, y: 0, scale: 1, duration: 0.9, ease: 'settle', immediateRender: true}, 0.05);
    if (p.badge) m.fadeUp('.badge', 0.5, -10);
    if (p.note) m.fadeUp('.note', 0.8, 12);
  });
  if (!asset) return <Missing id={shot.args[0]} />;
  const t = Math.min(1, frame / fps / shot.duration);
  const [fx, fy, fz] = triple(p.from, [0.5, 0.5, 1]);
  const [tx, ty, tz] = triple(p.to, [0.5, 0.5, 1.15]);
  const k = easeCam(t);
  const z = lerp(fz, tz, k), cx = lerp(fx, tx, k), cy = lerp(fy, ty, k);
  const ar = asset.width / asset.height;
  const boxH = 880, boxW = Math.min(1600, boxH * ar);
  const h = boxW / ar;
  return <AbsoluteFill ref={scope}>
    <Paper dark />
    <AbsoluteFill style={{alignItems: 'center', justifyContent: 'center'}}>
      <div style={{transform: `scale(${z})`, transformOrigin: `${cx * 100}% ${cy * 100}%`}}>
        <div className="frame" style={{opacity: 0, width: boxW, height: h, padding: 0, boxShadow: '0 30px 90px rgba(0,0,0,.6)', outline: `1px solid rgba(224,169,59,.35)`, outlineOffset: 14}}>
          <Img src={staticFile(asset.file)} style={{width: boxW, height: h, display: 'block', filter: 'contrast(1.03) saturate(1.04)'}} />
        </div>
      </div>
    </AbsoluteFill>
    {p.tag && <Tag text={p.tag} />}
    {p.badge && <div className="badge" style={{position: 'absolute', right: 72, top: 64, opacity: 0, font: `500 19px ${F.mono}`, letterSpacing: 1.6,
      color: C.white, background: 'rgba(255,248,236,.12)', padding: '9px 16px', borderRadius: 4, border: '1px solid rgba(255,248,236,.3)'}}>{p.badge}</div>}
    {p.note && <div className="note" style={{position: 'absolute', right: 72, bottom: 40, opacity: 0, maxWidth: 1100, textAlign: 'right',
      font: `500 27px ${F.sans}`, color: 'rgba(255,248,236,.9)', textShadow: '0 2px 14px rgba(0,0,0,.8)'}}>{p.note}</div>}
  </AbsoluteFill>;
};
