import React from 'react';
import {AbsoluteFill, useCurrentFrame, useVideoConfig} from 'remotion';
import {C, F, Film, Shot, useGsap} from '../core';
import {Board, halo, label, mono, stepReveal} from './kit';

type P = {shot: Shot; film: Film};
const Svg: React.FC<{children: React.ReactNode}> = ({children}) =>
  <svg width={1920} height={1080} style={{position: 'absolute', inset: 0, overflow: 'visible'}}>{children}</svg>;

const Ox: React.FC<{x: number; y: number; t: number; flip?: boolean}> = ({x, y, t, flip}) => {
  const walk = Math.sin(t * 3) * 10;
  return <g transform={`translate(${x + walk} ${y}) scale(${flip ? -1 : 1} 1)`}>
    <ellipse cx={0} cy={-34} rx={44} ry={24} fill="#7B6048" stroke={C.ink} strokeWidth={3} />
    <path d="M38 -46 q22 -6 26 10 q-6 14 -22 10 Z" fill="#7B6048" stroke={C.ink} strokeWidth={3} />
    <path d="M52 -52 q6 -12 14 -10" fill="none" stroke={C.ink} strokeWidth={3} />
    {[-26, -10, 14, 30].map((lx, i) => <path key={i} d={`M${lx} -14 L${lx + Math.sin(t * 6 + i * 1.7) * 5} 0`} stroke={C.ink} strokeWidth={5} strokeLinecap="round" />)}
  </g>;
};

/* ---------------------------------------------------------------- the three-stage water lift */
export const WaterLift: React.FC<P> = ({shot}) => {
  const step = Number(shot.params.step || 0);
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const t = frame / fps;
  const scope = useGsap(ctx => stepReveal(ctx, shot, step));
  const lakeY = 900, levels = [850, 700, 550, 400, 250], towersX = [960, 1130, 1300, 1470];
  const hill = 'M520 1080 L520 905 C640 880 820 872 905 860 L985 860 L1010 718 L1150 712 L1180 566 L1320 560 L1350 414 L1490 408 L1520 262 L1880 236 L1920 236 L1920 1080 Z';
  const flow = -t * 70;
  const bag = (x: number, y0: number, y1: number, ph = 0) => {
    const k = (Math.sin(t * 1.6 + ph) + 1) / 2;
    return <g><path d={`M${x} ${y1 - 40} V${y0 - (y0 - y1) * k}`} stroke={C.ink} strokeWidth={2} />
      <path d={`M${x - 12} ${y0 - (y0 - y1) * k} q12 26 24 0 Z`} fill="#8A5A36" stroke={C.ink} strokeWidth={2} /></g>;
  };
  const wheelA = t * 1.4;
  const pots = Array.from({length: 14}, (_, i) => {
    const u = ((i / 14 + t * 0.12) % 1);
    // loop: down the left side, round the bottom, up the right side, over the wheel
    const L = 140, top = 214, x0 = 1615, x1 = 1665;
    if (u < 0.4) return [x0, top + (u / 0.4) * L];
    if (u < 0.5) { const a = Math.PI * ((u - 0.4) / 0.1); return [1640 - Math.cos(a) * 25, top + L + Math.sin(a) * 18]; }
    if (u < 0.9) return [x1, top + L - ((u - 0.5) / 0.4) * L];
    const a = Math.PI * ((u - 0.9) / 0.1); return [1640 + Math.cos(a) * 25, top - Math.sin(a) * 25];
  });
  return <Board tag="HOW WATER CLIMBED TO THE PALACE" legend="BASED ON THE UNESCO NOMINATION DOSSIER (2013) · SCHEMATIC, VERTICAL SCALE APPROX.">
    <AbsoluteFill ref={scope}>
      <Svg>
        <path d={hill} fill="#D8C49C" stroke="#9C8460" strokeWidth={4} />
        <path d={`M40 ${lakeY} H520 V1080 H40 Z`} fill="#7FA8B6" />
        <path d={`M40 ${lakeY} H520`} stroke="#4F7F92" strokeWidth={4} strokeDasharray="30 18" strokeDashoffset={flow * 0.3} />
        <text x={250} y={980} textAnchor="middle" style={{...label(36, '#F4EBD9', 700)}}>Maota Lake</text>
        <g>
          <path d="M1700 236 V110 H1880 V236" fill="#EAD8B0" stroke={C.ink} strokeWidth={5} />
          <path d="M1690 110 H1890 M1710 110 V86 H1735 V110 M1760 110 V86 H1785 V110 M1810 110 V86 H1835 V110 M1858 110 V86 H1880 V110" fill="none" stroke={C.ink} strokeWidth={5} />
          <text x={1790} y={70} textAnchor="middle" style={label(32)}>Palace</text>
        </g>
        {/* step 0: the problem */}
        <g data-s="0:0" data-a="fade">
          {step === 0 && <>
            <path d="M300 860 C 700 600, 1300 300, 1690 170" fill="none" stroke={C.terracotta} strokeWidth={6} strokeDasharray="18 14" />
            <text x={900} y={520} textAnchor="middle" style={{...label(110, C.terracotta, 800), ...halo()}}>?</text>
          </>}
        </g>
        <g data-s="0:1" data-a="pop">{step === 0 && [[580, 820, '1'], [1200, 620, '2'], [1600, 300, '3']].map(([x, y, n]) =>
          <g key={n as string}><circle cx={x as number} cy={y as number} r={40} fill={C.ochre} stroke={C.ink} strokeWidth={4} />
            <text x={x as number} y={(y as number) + 15} textAnchor="middle" style={label(44, C.ink, 800)}>{n}</text></g>)}</g>
        {/* stage 1: animals, pulleys, leather bags into two terrace tanks */}
        <g data-s="1:0-0.4" data-a="up">
          <path d="M540 905 V780 H640 V905" fill="none" stroke={C.ink} strokeWidth={6} />
          <circle cx={560} cy={790} r={14} fill="#fff" stroke={C.ink} strokeWidth={4} />
          {bag(560, 900, 800)}
          <Ox x={700} y={845} t={t} />
          <rect x={600} y={812} width={34} height={30} fill="#9CC3CC" stroke={C.ink} strokeWidth={3} />
          <rect x={640} y={812} width={34} height={30} fill="#9CC3CC" stroke={C.ink} strokeWidth={3} />
          <text x={600} y={690} textAnchor="middle" style={{...label(28, C.ink, 700), ...halo()}}>STAGE 1</text>
          <text x={600} y={740} textAnchor="middle" style={{...mono(18), ...halo()}}>PULLEYS · LEATHER BAGS · TWO TANKS</text>
        </g>
        <g data-s="2:0" data-a="fade">
          <path d="M675 835 C760 845 860 852 945 852" fill="none" stroke="#9C6B45" strokeWidth={12} />
          <path d="M675 835 C760 845 860 852 945 852" fill="none" stroke="#9CC3CC" strokeWidth={4} strokeDasharray="14 12" strokeDashoffset={flow} />
          <text x={800} y={905} textAnchor="middle" style={{...label(28, '#7A4E33', 700), ...halo('#D8C49C')}}>clay pipeline ≈ 125 m</text>
        </g>
        {/* stage 2: four towers, each with pulley, animals, tank */}
        {towersX.map((x, i) => <g key={i} data-s={`3:${i === 0 ? '0' : '0+' + (i * 0.35).toFixed(2)}`} data-a="up">
          <path d={`M${x - 30} ${levels[i]} V${levels[i + 1] - 30} H${x + 140} V${levels[i + 1] + 10}`} fill="none" stroke={C.ink} strokeWidth={6} />
          <rect x={x - 26} y={levels[i] - 26} width={52} height={26} fill="#9CC3CC" stroke={C.ink} strokeWidth={3} />
          <circle cx={x} cy={levels[i + 1] - 30} r={13} fill="#fff" stroke={C.ink} strokeWidth={4} />
          <Ox x={x + 85} y={levels[i + 1] - 6} t={t + i} />
        </g>)}
        <text data-s="3:1" x={1060} y={330} textAnchor="middle" style={{...label(28, C.ink, 700), ...halo()}}>STAGE 2 · FOUR LIFTING TOWERS</text>
        {towersX.map((x, i) => <g key={i} data-s={`4:0${i ? '+' + (i * 0.3).toFixed(1) : ''}`} data-a="fade">
          {bag(x, levels[i] - 6, levels[i + 1] - 20, i)}
          <path d={`M${x - 60} ${levels[i] - 20} V${levels[i + 1] + 20}`} stroke={C.water} strokeWidth={6} markerEnd="url(#up)" />
          <text x={x - 70} y={(levels[i] + levels[i + 1]) / 2} textAnchor="end" style={{...label(24, C.water, 700), ...halo('#D8C49C')}}>10–13 m</text>
        </g>)}
        <defs><marker id="up" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10 Z" fill={C.water} /></marker></defs>
        {/* stage 3: Persian wheel */}
        <g data-s="5:0" data-a="up">
          <rect x={1590} y={340} width={100} height={40} fill="#9CC3CC" stroke={C.ink} strokeWidth={3} />
          <g transform={`rotate(${(wheelA * 180) / Math.PI} 1640 214)`}><circle cx={1640} cy={214} r={36} fill="none" stroke={C.ink} strokeWidth={6} />
            {[0, 1, 2, 3].map(k => <line key={k} x1={1640} y1={214} x2={1640 + Math.cos(k * Math.PI / 2) * 36} y2={214 + Math.sin(k * Math.PI / 2) * 36} stroke={C.ink} strokeWidth={4} />)}</g>
          {pots.map(([x, y], i) => <circle key={i} cx={x} cy={y} r={8} fill="#A0623A" stroke={C.ink} strokeWidth={2} />)}
          <text x={1570} y={455} textAnchor="end" style={{...label(28, C.ink, 700), ...halo()}}>STAGE 3 · PERSIAN WHEEL</text>
        </g>
        <g data-s="5:1" data-a="fade">
          <path d="M1680 190 H1720" stroke={C.water} strokeWidth={10} />
          <path d="M1700 190 C1740 190 1760 200 1790 236" stroke="#9CC3CC" strokeWidth={5} strokeDasharray="12 10" strokeDashoffset={flow} fill="none" />
          <text x={1600} y={500} textAnchor="end" style={{...mono(18), ...halo()}}>POTS DIP, FILL, RISE AND TIP</text>
        </g>
        {/* step 6: scale, drawn in the clear sky above the lake */}
        <g data-s="6:payoff-0.3" data-a="fade">
          <path d="M255 850 V250 M235 850 H275 M235 250 H275" stroke={C.terracotta} strokeWidth={5} />
          <text x={285} y={520} style={{...label(64, C.terracotta, 800), ...halo()}}>≈ 45 m</text>
          <text x={287} y={562} style={{...mono(20, C.terracotta), ...halo()}}>THE FOUR TOWERS</text>
        </g>
        <g data-s="6:payoff+0.6" data-a="up">
          <rect x={120} y={250} width={90} height={600} fill="#E9DCC1" stroke={C.ink} strokeWidth={4} />
          {Array.from({length: 14}, (_, k) => <line key={k} x1={120} x2={210} y1={250 + (k + 1) * 40} y2={250 + (k + 1) * 40} stroke={C.ink} strokeWidth={2} />)}
          <text x={165} y={232} textAnchor="middle" style={{...mono(18, C.ink), ...halo()}}>15 STOREYS</text>
        </g>
      </Svg>
    </AbsoluteFill>
  </Board>;
};

/* ---------------------------------------------------------------- palace plan: the walking route */
const COURTS = [
  {id: 'jaleb', name: 'Jaleb Chowk', sub: 'FIRST COURTYARD', x: 130, y: 290, w: 470, h: 400, step: 1},
  {id: 'diwan', name: 'Diwan-i-Aam', sub: 'SECOND COURTYARD', x: 640, y: 250, w: 360, h: 440, step: 2},
  {id: 'garden', name: 'Garden court', sub: 'THIRD COURTYARD', x: 1040, y: 250, w: 410, h: 440, step: 4},
  {id: 'mansingh', name: 'Man Singh Mahal', sub: 'ZENANA · OLDEST', x: 1490, y: 220, w: 330, h: 490, step: 5},
];
// One route segment per step (index = step - 1).
const ROUTE = [
  'M330 1000 C360 880 440 760 470 690 L380 500',
  'M380 500 L615 640 L800 500',
  'M800 500 L1020 500',
  'M1020 500 L1240 500',
  'M1240 500 L1470 500 L1655 470',
];
const HERE = [[0, 0], [380, 500], [800, 500], [1020, 500], [1240, 500], [1655, 470]];
export const PalacePlan: React.FC<P> = ({shot}) => {
  const step = Number(shot.params.step || 0);
  const overview = step === 0 || step === 6 || shot.params.highlight === 'all';
  const scope = useGsap(ctx => {
    stepReveal(ctx, shot, 1);
    ctx.q('.seg').forEach(el => {
      const idx = Number(el.getAttribute('data-i'));
      if (step === 6) ctx.tl.fromTo(el, {drawSVG: '0%'}, {drawSVG: '100%', duration: 0.55, ease: 'none', immediateRender: true}, 0.3 + idx * 0.5);
      else if (step === 0) ctx.tl.set(el, {drawSVG: '0%'}, 0);
      else if (idx < step - 1) ctx.tl.set(el, {drawSVG: '100%'}, 0);
      else if (idx === step - 1) ctx.tl.fromTo(el, {drawSVG: '0%'}, {drawSVG: '100%', duration: 1.3, ease: 'power2.inOut', immediateRender: true}, 0.3);
      else ctx.tl.set(el, {drawSVG: '0%'}, 0);
    });
    if (step === 0) ctx.q('.court').forEach((el, i) => ctx.tl.fromTo(el, {opacity: 0, y: 14}, {opacity: 1, y: 0, duration: 0.6, ease: 'settle', immediateRender: true}, 0.2 + i * 0.25));
    if (ctx.q('.mark').length) {
      ctx.tl.fromTo(ctx.q('.mark'), {opacity: 0}, {opacity: 1, duration: 0.4, immediateRender: true}, 0.6);
      ctx.tl.fromTo(ctx.q('.mark-ring'), {attr: {r: 120}}, {attr: {r: 46}, duration: 0.6, ease: 'back.out(1.6)', immediateRender: true}, 0.6);
    }
    if (ctx.q('.here').length) ctx.tl.fromTo(ctx.q('.here'), {scale: 0, opacity: 0}, {scale: 1, opacity: 1, duration: 0.5, ease: 'back.out(2)', transformOrigin: '50% 50%', immediateRender: true}, 1.5);
  });
  const here = HERE[Math.min(5, step)];
  const box = (x: number, y: number, w: number, h: number, name: string, fill = '#D9D3C6') => <g>
    <rect x={x} y={y} width={w} height={h} fill={fill} stroke={C.ink} strokeWidth={3} />
    <text x={x + w / 2} y={y + h / 2 + 6} textAnchor="middle" style={mono(17, C.ink)}>{name}</text></g>;
  return <Board tag="AMBER PALACE · THE ROUTE" legend="SCHEMATIC · NOT TO SCALE · AFTER THE UNESCO NOMINATION PLAN">
    <AbsoluteFill ref={scope}>
      <Svg>
        <path d="M0 860 C300 840 500 900 800 880 C1100 860 1400 900 1920 870 V1080 H0 Z" fill="#9EC0C9" />
        <text x={1150} y={985} textAnchor="middle" style={{...mono(26, '#3F6E7E')}}>MAOTA LAKE</text>
        <rect x={560} y={915} width={170} height={74} fill="#9DB386" stroke="#5F7A4A" strokeWidth={3} />
        <text x={645} y={1022} textAnchor="middle" style={mono(16, '#3F6E7E')}>KESAR KYARI</text>
        <g transform="translate(1840 110)"><path d="M0 -36 L12 0 L0 -8 L-12 0 Z" fill={C.ink} /><text x={0} y={30} textAnchor="middle" style={mono(18, C.ink)}>N</text></g>
        <text x={1800} y={170} textAnchor="end" style={mono(17)}>JAIGARH FORT · 152 m HIGHER ↗</text>
        {COURTS.map(c => {
          const on = overview || c.step === step || (c.id === 'diwan' && step === 3);
          return <g key={c.id} className="court">
            <rect x={c.x} y={c.y} width={c.w} height={c.h} rx={6} fill={on ? 'rgba(224,169,59,.42)' : '#EADFC8'} stroke={on ? C.ink : '#B4A07C'} strokeWidth={on ? 5 : 3} />
            <text x={c.x + c.w / 2} y={c.y + 52} textAnchor="middle" style={{...label(on ? 38 : 30, on ? C.ink : C.inkSoft, on ? 700 : 600)}}>{c.name}</text>
            <text x={c.x + c.w / 2} y={c.y + 84} textAnchor="middle" style={mono(16, on ? C.terracotta : '#A08E70')}>{c.sub}</text>
          </g>;
        })}
        {box(680, 600, 230, 70, 'DIWAN-I-AAM HALL', '#C9AE84')}
        {box(1075, 610, 340, 66, 'SHEESH MAHAL')}
        {box(1075, 330, 340, 56, 'SUKH NIWAS')}
        <path d="M1150 470 H1340 M1245 420 V560 M1150 520 H1340" stroke="#5F8D9D" strokeWidth={7} />
        <rect x={1615} y={430} width={86} height={86} fill="#D9C39A" stroke={C.ink} strokeWidth={3} />
        <text x={1658} y={548} textAnchor="middle" style={mono(15, C.ink)}>BARADARI</text>
        <rect x={998} y={455} width={44} height={90} fill={C.terracotta} />
        <text x={1020} y={440} textAnchor="middle" style={{...mono(17, C.terracotta), ...halo('#EADFC8')}}>GANESH POL</text>
        <rect x={446} y={676} width={50} height={30} fill={C.terracotta} />
        <text x={471} y={738} textAnchor="middle" style={{...mono(17, C.terracotta), ...halo()}}>SURAJ POL</text>
        <rect x={604} y={612} width={30} height={30} fill="#8A6A44" />
        <text x={560} y={668} textAnchor="end" style={{...mono(14, C.inkSoft)}}>SHILA DEVI</text>
        {shot.params.mark === 'shila' && <g className="mark" style={{opacity: 0}}>
          <circle className="mark-ring" cx={619} cy={627} r={46} fill="none" stroke={C.terracotta} strokeWidth={6} />
          <path d="M590 660 L380 790" stroke={C.terracotta} strokeWidth={3} />
          <text x={370} y={815} textAnchor="end" style={{...label(30, C.terracotta, 800), ...halo()}}>Singh Pol → Shila Devi</text></g>}
        {ROUTE.map((d, i) => <path key={i} className="seg" data-i={i} d={d} fill="none" stroke={C.teal} strokeWidth={10} strokeLinecap="round" strokeLinejoin="round" />)}
        {!overview && <g className="here"><circle cx={here[0]} cy={here[1]} r={24} fill={C.teal} stroke="#fff" strokeWidth={5} />
          <text x={here[0]} y={here[1] - 40} textAnchor="middle" style={{...label(26, C.teal, 800), ...halo('#EADFC8')}}>YOU ARE HERE</text></g>}
      </Svg>
    </AbsoluteFill>
  </Board>;
};
