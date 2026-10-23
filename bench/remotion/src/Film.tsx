import React, {useLayoutEffect} from 'react';
import {AbsoluteFill, Audio, Sequence, continueRender, delayRender, staticFile, useCurrentFrame, useVideoConfig} from 'remotion';
import {C, F, Film, Shot, useGsap, motion} from './core';
import {PhotoShot} from './shots/Photo';
import {CardShot, PaintingShot, StatShot} from './shots/Typography';
import {CompareShot} from './shots/Compare';
import {MapShot} from './maps/MapShot';
import {Cooling, Family, GaneshRoute, JaivanaScale, JaliView, MirrorLight} from './diagrams/Explainers';
import {PalacePlan, WaterLift} from './diagrams/Engineering';

const DIAGRAMS: Record<string, React.FC<{shot: Shot; film: Film}>> = {
  jaivana_scale: JaivanaScale, family: Family, jali_view: JaliView, ganesh_route: GaneshRoute,
  mirror_light: MirrorLight, cooling: Cooling, water_lift: WaterLift,
};

export const ShotView: React.FC<{shot: Shot; film: Film}> = ({shot, film}) => {
  switch (shot.type) {
    case 'photo': return <PhotoShot shot={shot} film={film} />;
    case 'illustration': return <PhotoShot shot={shot} film={film} grade="painting" />;
    case 'painting': return <PaintingShot shot={shot} film={film} />;
    case 'stat': return <StatShot shot={shot} film={film} />;
    case 'card': return <CardShot shot={shot} film={film} />;
    case 'compare': return <CompareShot shot={shot} film={film} />;
    case 'map': return <MapShot shot={shot} film={film} />;
    case 'plan': return <PalacePlan shot={shot} film={film} />;
    case 'diagram': {
      const D = DIAGRAMS[shot.args[0]];
      if (D) return <D shot={shot} film={film} />;
    }
  }
  return <AbsoluteFill style={{background: '#300', color: '#fff', font: `40px ${F.mono}`, padding: 80}}>UNKNOWN SHOT {shot.type} {shot.args.join(' ')}</AbsoluteFill>;
};

/** Chapter marker: number + title slides in over the first shot of each chapter. */
const ChapterMark: React.FC<{n: number; title: string}> = ({n, title}) => {
  const scope = useGsap(ctx => {
    const m = motion(ctx);
    m.draw('.cm-rule', 0.15, 0.6);
    m.chars('.cm-n', 0.2, 0.04);
    m.lines('.cm-t', 0.32, 0.05, 0.7);
    ctx.tl.to(ctx.q('.cm'), {opacity: 0, y: -10, duration: 0.5, ease: 'power2.in'}, 3.4);
  });
  return <AbsoluteFill ref={scope} style={{pointerEvents: 'none'}}>
    <div className="cm" style={{position: 'absolute', left: 72, bottom: 72, display: 'flex', alignItems: 'center', gap: 22,
      background: 'rgba(22,17,13,.72)', padding: '18px 30px 18px 22px', borderRadius: 6, backdropFilter: 'blur(8px)'}}>
      <span className="cm-n" style={{font: `600 44px ${F.display}`, color: C.ochre, fontVariantNumeric: 'tabular-nums'}}>{String(n).padStart(2, '0')}</span>
      <svg width={4} height={46}><path className="cm-rule" d="M2 0 V46" stroke={C.ochre} strokeWidth={3} /></svg>
      <span className="cm-t" style={{font: `600 34px ${F.sans}`, color: C.white, letterSpacing: -0.4}}>{title}</span>
    </div>
  </AbsoluteFill>;
};

/** Paper/film texture: one static tile at low opacity. Per-frame random grain was removed:
 * noise cannot be compressed (it pushed the encode to ~25 Mbps) and YouTube smears it anyway. */
const Texture: React.FC = () =>
  <AbsoluteFill style={{backgroundImage: `url(${staticFile('_engine/grain/0.png')})`, mixBlendMode: 'overlay', opacity: 0.1, pointerEvents: 'none'}} />;

/**
 * QA probe (only when inputProps.qa is set): after layout, report visible text boxes that
 * overlap each other or leave the title-safe area, and any missing-asset placeholders.
 */
const TEXT_SEL = '.tag,.title,.note,.badge,.hero,.plabel span,.stat-num,.stat-label,.kicker,.cm,[class^="line-"],svg text';
const QaProbe: React.FC<{film: Film}> = ({film}) => {
  const frame = useCurrentFrame();
  useLayoutEffect(() => {
    // Shots build their GSAP timelines after fonts load (async), so probe a moment later.
    const handle = delayRender('qa probe');
    const timer = setTimeout(() => { probe(); continueRender(handle); }, 500);
    return () => { clearTimeout(timer); continueRender(handle); };
  });
  const probe = () => {
    const vis = (el: Element) => {
      let o = 1;
      for (let e: Element | null = el; e; e = e.parentElement) o *= Number(getComputedStyle(e).opacity || 1);
      return o;
    };
    const boxes = [...document.querySelectorAll(TEXT_SEL)].map(el => ({el, r: el.getBoundingClientRect(), o: vis(el), t: (el.textContent || '').trim().slice(0, 40)}))
      .filter(b => b.o > 0.5 && b.r.width > 2 && b.r.height > 2 && b.t)
      // Remotion pre-mounts neighbouring sequences far off-screen; only judge what is in the frame.
      .filter(b => b.r.right > 0 && b.r.bottom > 0 && b.r.left < film.width && b.r.top < film.height);
    const issues: string[] = [];
    const m = 24;
    for (const b of boxes) if (b.r.left < m || b.r.top < m || b.r.right > film.width - m || b.r.bottom > film.height - m) issues.push(`outside-safe: "${b.t}" [${Math.round(b.r.left)},${Math.round(b.r.top)},${Math.round(b.r.right)},${Math.round(b.r.bottom)}] vp=${innerWidth}x${innerHeight}`);
    for (let i = 0; i < boxes.length; i++) for (let j = i + 1; j < boxes.length; j++) {
      const a = boxes[i].r, b = boxes[j].r;
      if (boxes[i].el.contains(boxes[j].el) || boxes[j].el.contains(boxes[i].el)) continue;
      const w = Math.min(a.right, b.right) - Math.max(a.left, b.left), h = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
      if (w > 4 && h > 4 && w * h > 0.12 * Math.min(a.width * a.height, b.width * b.height)) issues.push(`overlap: "${boxes[i].t}" × "${boxes[j].t}"`);
    }
    if (document.body.innerText.includes('MISSING ASSET') || document.body.innerText.includes('UNKNOWN')) issues.push('missing-asset');
    console.log('QA ' + JSON.stringify({frame, issues}));
  };
  return null;
};

export const FilmComp: React.FC<{film: Film; from?: number; qa?: boolean}> = ({film, qa}) => {
  const {fps} = useVideoConfig();
  return <AbsoluteFill style={{background: C.night}}>
    {film.shots.map(s => <Sequence key={s.id} from={s.startFrame} durationInFrames={s.durationInFrames} name={`${s.id} ${s.type} ${s.args[0] || ''}`}>
      <ShotView shot={s} film={film} />
    </Sequence>)}
    {film.chapters.map((c, i) => i > 0 && <Sequence key={c.id} from={c.startFrame} durationInFrames={Math.round(4.2 * fps)} name={`chapter ${c.title}`}>
      <ChapterMark n={i} title={c.title} />
    </Sequence>)}
    <Texture />
    {qa && <QaProbe film={film} />}
    {film.narration && <Audio src={staticFile(film.narration)} />}
    {film.music && <Audio src={staticFile(film.music)} />}
  </AbsoluteFill>;
};
