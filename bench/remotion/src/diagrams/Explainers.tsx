import React from 'react';
import {AbsoluteFill, useCurrentFrame, useVideoConfig} from 'remotion';
import {C, F, Film, Shot, cueAt, useGsap} from '../core';
import {Board, Dim, halo, label, mono, stepReveal} from './kit';

type P = {shot: Shot; film: Film};
const stepOf = (shot: Shot) => Number(shot.params.step || 1);
const Svg: React.FC<{children: React.ReactNode}> = ({children}) =>
  <svg width={1920} height={1080} style={{position: 'absolute', inset: 0, overflow: 'visible'}}>{children}</svg>;

const Person: React.FC<{x: number; ground: number; h: number; color?: string}> = ({x, ground, h, color = C.ink}) => {
  const head = h * 0.13;
  return <g>
    <circle cx={x} cy={ground - h + head} r={head} fill={color} />
    <path d={`M${x - h * 0.13} ${ground - h + head * 2.3} Q${x} ${ground - h + head * 1.9} ${x + h * 0.13} ${ground - h + head * 2.3}
      L${x + h * 0.11} ${ground - h * 0.48} L${x + h * 0.08} ${ground} L${x + h * 0.02} ${ground} L${x} ${ground - h * 0.42}
      L${x - h * 0.02} ${ground} L${x - h * 0.08} ${ground} L${x - h * 0.11} ${ground - h * 0.48} Z`} fill={color} />
  </g>;
};

/* ---------------------------------------------------------------- Jaivana to scale */
export const JaivanaScale: React.FC<P> = ({shot}) => {
  const scope = useGsap(ctx => stepReveal(ctx, shot, 1));
  const m = 158, ground = 880, dx = -230;
  const bx0 = 470 + dx, bx1 = bx0 + 6.15 * m, by = 640, rb = 0.89 / 2 * m, rm = 0.70 / 2 * m;
  const wheel = (cx: number, r: number, ghost = false) => <g>
    <circle cx={cx} cy={ground - r} r={r} fill={ghost ? 'none' : '#4A3A2C'} stroke={ghost ? C.terracotta : C.ink} strokeWidth={ghost ? 4 : 6} strokeDasharray={ghost ? '14 10' : undefined} />
    {!ghost && Array.from({length: 12}, (_, i) => { const a = i * Math.PI / 6; return <line key={i} x1={cx} y1={ground - r} x2={cx + Math.cos(a) * r * 0.9} y2={ground - r + Math.sin(a) * r * 0.9} stroke="#8E7A62" strokeWidth={4} />; })}
    {!ghost && <circle cx={cx} cy={ground - r} r={r * 0.18} fill={C.ink} />}
  </g>;
  return <Board tag="JAIVANA · TO SCALE" legend="REPORTED DIMENSIONS · SILHOUETTE SIMPLIFIED">
    <AbsoluteFill ref={scope}>
      <Svg>
        <path d={`M60 ${ground} H1860`} stroke={C.ink} strokeWidth={4} />
        <g data-s="1:t0.15" data-a="up">
          <path d={`M${bx0 + 120} ${by + 40} L${bx1 - 140} ${by + 30} L${bx1 - 220} ${ground - 120} L${bx0 + 220} ${ground - 120} Z`} fill="#6B7D72" stroke={C.ink} strokeWidth={5} />
          {wheel(bx0 + 330, 0.685 * m)}{wheel(bx1 - 300, 0.685 * m)}
          <path d={`M${bx0} ${by - rb} L${bx1} ${by - rm} L${bx1} ${by + rm} L${bx0} ${by + rb} Z`} fill="#7A4E33" stroke={C.ink} strokeWidth={6} strokeLinejoin="round" />
          {[0.18, 0.5, 0.82].map(f => <path key={f} d={`M${bx0 + f * (bx1 - bx0)} ${by - rb + (rb - rm) * f - 6} V${by + rb - (rb - rm) * f + 6}`} stroke="#5A3622" strokeWidth={10} />)}
          <circle cx={bx0 - 18} cy={by} r={30} fill="#7A4E33" stroke={C.ink} strokeWidth={6} />
          <ellipse cx={bx1} cy={by} rx={14} ry={rm} fill="#2A1A12" stroke={C.ink} strokeWidth={5} />
        </g>
        <Dim x1={bx0} y1={by - rb - 60} x2={bx1} y2={by - rm - 60} text="6.15 m barrel" s="1:t0.9" />
        <g data-s="1:0" data-a="up">
          <Person x={1180} ground={ground} h={1.37 * m} color="#8E6F52" />
          <text x={1180} y={ground + 50} textAnchor="middle" style={label(28, C.inkSoft)}>child ≈ main wheel</text>
          <text x={1180} y={ground + 84} textAnchor="middle" style={mono(20, C.terracotta)}>≈ 1.37 m</text>
        </g>
        <g data-s="1:1-0.3" data-a="fade">{wheel(1620, 1.37 * m, true)}
          <text x={1620} y={ground - 2.74 * m - 24} textAnchor="middle" style={{...label(30, C.terracotta, 700), ...halo()}}>transport wheel ≈ 2.74 m</text></g>
        <g data-s="1:1" data-a="up"><Person x={1385} ground={ground} h={1.75 * m} />
          <text x={1385} y={ground + 50} textAnchor="middle" style={label(28, C.inkSoft)}>adult</text>
          <text x={1385} y={ground + 84} textAnchor="middle" style={mono(20, C.terracotta)}>≈ 1.75 m</text></g>
      </Svg>
    </AbsoluteFill>
  </Board>;
};

/* ---------------------------------------------------------------- family tree */
const Node: React.FC<{x: number; y: number; name: string; sub?: string; s: string; hl?: string; w?: number; id?: string}> = ({x, y, name, sub, s, hl, w = 380, id}) =>
  <g data-s={s} data-a="up" className={id}>
    <rect x={x - w / 2} y={y - 66} width={w} height={132} rx={10} fill="#FBF4E6" stroke={hl || C.line} strokeWidth={hl ? 5 : 2.5} />
    <text x={x} y={y + (sub ? -2 : 14)} textAnchor="middle" style={{font: `500 46px ${F.display}`, letterSpacing: -1, fill: C.ink}}>{name}</text>
    {sub && <text x={x} y={y + 40} textAnchor="middle" style={mono(19, C.terracotta)}>{sub}</text>}
  </g>;
export const Family: React.FC<P> = ({shot}) => {
  const step = stepOf(shot);
  const scope = useGsap(ctx => stepReveal(ctx, shot, step));
  const B = [960, 210], BD = [430, 490], MZ = [1080, 490], AK = [1600, 490], MS = [430, 790], JH = [1340, 790];
  return <Board tag="THE KACHHWAHA–MUGHAL FAMILY">
    <AbsoluteFill ref={scope}>
      <Svg>
        <path data-s="1:t0.3" data-a="draw" d={`M${B[0]} ${B[1] + 66} V330 H${BD[0]} V${BD[1] - 66} M${B[0]} 330 H${MZ[0]} V${MZ[1] - 66}`} fill="none" stroke={C.inkSoft} strokeWidth={4} />
        <path data-s="1:t0.8" data-a="draw" d={`M${MZ[0] + 190} ${MZ[1] - 10} H${AK[0] - 190} M${MZ[0] + 190} ${MZ[1] + 10} H${AK[0] - 190}`} stroke={C.terracotta} strokeWidth={4} />
        <text data-s="1:t1.0" x={(MZ[0] + AK[0]) / 2 + 5} y={MZ[1] + 52} textAnchor="middle" style={mono(17, C.terracotta)}>1562</text>
        <Node x={B[0]} y={B[1]} name="Raja Bharmal" sub="RULER OF AMBER" s="1:t0.1" />
        <Node x={BD[0]} y={BD[1]} name="Bhagwant Das" sub="SON" s="1:t0.3" id="bd" />
        <Node x={AK[0]} y={AK[1]} name="Akbar" sub="MUGHAL EMPEROR" s="1:t0.8" w={340} />
        <g data-s="1:t0.5" data-a="up">
          <rect x={MZ[0] - 200} y={MZ[1] - 66} width={400} height={132} rx={10} fill="#FBF4E6" stroke={C.ochre} strokeWidth={5} />
          <text x={MZ[0] + 120} y={MZ[1] - 84} textAnchor="start" style={mono(17, C.inkSoft)}>DAUGHTER</text>
        </g>
        <g data-hide="1:0-0.1"><g data-s="1:t0.6"><text x={MZ[0]} y={MZ[1] + 14} textAnchor="middle" style={{font: `italic 400 44px ${F.display}`, fill: C.inkSoft}}>“Jodha Bai”</text></g>
          <path data-s="1:0-1.6" data-a="draw" d={`M${MZ[0] - 130} ${MZ[1] + 2} L${MZ[0] + 130} ${MZ[1] - 2}`} stroke={C.terracotta} strokeWidth={7} strokeLinecap="round" /></g>
        <text data-s="1:0-1.4" x={MZ[0]} y={MZ[1] + 116} textAnchor="middle" style={mono(18, C.terracotta)}>NOT IN MUGHAL CHRONICLES</text>
        <g data-s="1:0" data-a="up"><rect x={MZ[0] - 196} y={MZ[1] - 62} width={392} height={124} rx={8} fill="#FBF4E6" />
          <text x={MZ[0]} y={MZ[1] - 2} textAnchor="middle" style={{font: `500 40px ${F.display}`, fill: C.ink}}>Mariam-uz-Zamani</text>
          <text x={MZ[0]} y={MZ[1] + 40} textAnchor="middle" style={mono(18, C.terracotta)}>HER TITLE</text></g>
        <path data-s="2:0-0.3" data-a="draw" d={`M${(MZ[0] + AK[0]) / 2} ${MZ[1] + 10} V${JH[1] - 66}`} stroke={C.inkSoft} strokeWidth={4} fill="none" />
        <Node x={JH[0]} y={JH[1]} name="Jahangir" sub="NEXT EMPEROR · 1605" s="2:0" />
        <rect data-s="2:1" data-a="fade" x={MZ[0] - 230} y={MZ[1] - 96} width={AK[0] - MZ[0] + 420} height={JH[1] - MZ[1] + 196} rx={20} fill="rgba(224,169,59,.14)" stroke={C.ochre} strokeWidth={3} strokeDasharray="12 10" />
        <path data-s="3:0" data-a="draw" d={`M${BD[0]} ${BD[1] + 66} V${MS[1] - 66}`} stroke={C.terracotta} strokeWidth={6} fill="none" />
        <Node x={MS[0]} y={MS[1]} name="Man Singh" sub="GRANDSON · GENERAL" s="3:1-0.2" hl={C.terracotta} />
      </Svg>
    </AbsoluteFill>
  </Board>;
};

/* ---------------------------------------------------------------- seeing through the jali */
export const JaliView: React.FC<P> = ({shot}) => {
  const scope = useGsap(ctx => stepReveal(ctx, shot, 1));
  const ground = 860, eye: [number, number] = [1330, 392];
  const crowd = [[240, 0], [330, 1], [420, 0], [520, 1], [610, 0], [700, 1], [800, 0], [890, 1]];
  return <Board tag="SUHAG MANDIR · THE SCREENED GALLERY" legend="SIMPLIFIED SECTION">
    <AbsoluteFill ref={scope}>
      <Svg>
        <path d={`M60 ${ground} H1180`} stroke={C.ink} strokeWidth={4} />
        <text x={560} y={ground + 56} textAnchor="middle" style={mono(22)}>DIWAN-I-AAM COURTYARD</text>
        <g data-s="1:t0.2" data-a="up">{crowd.map(([x, v], i) => <Person key={i} x={x} ground={ground} h={150 + v * 14} color="#6E5A47" />)}</g>
        <g data-s="1:t0.1" data-a="up">
          <path d={`M1180 ${ground} V520 H1700 V${ground}`} fill="#E6CFA2" stroke={C.ink} strokeWidth={5} />
          <path d={`M1360 ${ground} V700 Q1440 620 1520 700 V${ground}`} fill="#4A3A2C" />
          <path d={`M1220 520 V330 H1660 V520`} fill="#EEDDBA" stroke={C.ink} strokeWidth={5} />
          <path d="M1200 330 Q1440 230 1680 330" fill="#E0C48E" stroke={C.ink} strokeWidth={5} />
          <text x={1440} y={600} textAnchor="middle" style={label(30)}>Ganesh Pol</text>
          <text x={1440} y={300} textAnchor="middle" style={{...label(30), ...halo('#E0C48E')}}>Suhag Mandir</text>
        </g>
        <g data-s="1:t0.5" data-a="fade">
          {Array.from({length: 6}, (_, r) => Array.from({length: 2}, (_, c) => <circle key={`${r}${c}`} cx={1222 + c * 0} cy={350 + r * 28} r={9} fill="none" stroke={C.ink} strokeWidth={3} />))}
          <rect x={1212} y={338} width={20} height={176} fill="url(#lattice)" stroke={C.ink} strokeWidth={4} />
        </g>
        <defs><pattern id="lattice" width={12} height={12} patternUnits="userSpaceOnUse"><path d="M0 6 H12 M6 0 V12" stroke={C.ink} strokeWidth={2} /></pattern></defs>
        <g data-s="1:t0.6" data-a="up"><Person x={1330} ground={520} h={150} color={C.terracotta} /></g>
        {crowd.map(([x], i) => <path key={i} data-s={`1:0${i ? '+' + (i * 0.06).toFixed(2) : ''}`} data-a="draw" data-d="0.6"
          d={`M${eye[0]} ${eye[1]} L${x} ${ground - 140}`} stroke={C.ochre} strokeWidth={3} strokeDasharray="2 0" fill="none" />)}
        <text data-s="1:0+0.5" x={760} y={330} textAnchor="middle" style={{...label(34, C.teal, 700), ...halo()}}>From inside: a clear view down</text>
        {[[420, 0], [700, 1]].map(([x], i) => <g key={i}>
          <path data-s={`1:1${i ? '+0.15' : ''}`} data-a="draw" data-d="0.5" d={`M${x} ${ground - 160} L1190 ${430 + i * 40}`} stroke={C.terracotta} strokeWidth={4} strokeDasharray="10 9" fill="none" />
          <g data-s={`1:1+${0.45 + i * 0.15}`} data-a="pop"><path d={`M1178 ${418 + i * 40} l24 24 M1202 ${418 + i * 40} l-24 24`} stroke={C.terracotta} strokeWidth={6} /></g>
        </g>)}
        <text data-s="1:1+0.6" x={760} y={420} textAnchor="middle" style={{...label(34, C.terracotta, 700), ...halo()}}>From outside: only stone lattice</text>
      </Svg>
    </AbsoluteFill>
  </Board>;
};

/* ---------------------------------------------------------------- Ganesh Pol's bent entrance */
export const GaneshRoute: React.FC<P> = ({shot}) => {
  const step = stepOf(shot);
  const scope = useGsap(ctx => {
    stepReveal(ctx, shot, step);
    if (step >= 2) {
      const at = step === 2 ? cueAt(shot, 0) : 0;
      ctx.tl.fromTo(ctx.q('.walker'), {opacity: 0}, {opacity: 1, duration: 0.2, immediateRender: true}, at);
      ctx.tl.to(ctx.q('.walker'), {motionPath: {path: ctx.q('.route-full')[0] as SVGPathElement, align: ctx.q('.route-full')[0] as SVGPathElement, alignOrigin: [0.5, 0.5]},
        duration: step === 2 ? Math.max(1.5, cueAt(shot, 1) - at + 1.2) : 2.4, ease: 'power1.inOut'}, at);
    }
  });
  const W = 22;
  return <Board tag="GANESH POL · PLAN" legend="SCHEMATIC PLAN · THE PASSAGE TURNS TWICE (UNESCO NOMINATION, 2013)">
    <AbsoluteFill ref={scope}>
      <Svg>
        <rect x={120} y={240} width={560} height={600} fill="#EADCC0" stroke="none" />
        <rect x={1240} y={240} width={560} height={600} fill="#E3D8BC" stroke="none" />
        <text x={400} y={210} textAnchor="middle" style={mono(22)}>PUBLIC COURT</text>
        <text x={1520} y={210} textAnchor="middle" style={mono(22)}>PRIVATE PALACE</text>
        <g data-s="1:t0.1" data-a="fade" stroke={C.ink} strokeWidth={W} strokeLinecap="square" fill="none">
          <path d="M760 260 H1160 V600" /><path d="M1160 720 V820 H760 V500" /><path d="M760 400 V260" />
          <path d="M960 260 V610" />
        </g>
        <g data-s="1:t0.1" data-a="fade"><rect x={771} y={271} width={178} height={538} fill="#F6EBD3" /><rect x={971} y={271} width={178} height={538} fill="#F6EBD3" />
          <text x={860} y={300} textAnchor="middle" style={mono(18)}>HALL 1</text><text x={1060} y={300} textAnchor="middle" style={mono(18)}>HALL 2</text></g>
        <path data-s="1:0" data-a="draw" data-d="0.8" d="M420 450 H940" stroke={C.terracotta} strokeWidth={7} strokeDasharray="16 12" fill="none" />
        <g data-s="1:0+0.75" data-a="pop"><path d="M918 428 l44 44 M962 428 l-44 44" stroke={C.terracotta} strokeWidth={9} /></g>
        <text data-s="1:0+0.9" x={600} y={410} textAnchor="middle" style={{...label(34, C.terracotta, 700), ...halo('#EADCC0')}}>No straight line through</text>
        <path className="route-full" d="M420 450 H860 V700 H1060 V660 H1500" fill="none" stroke="none" />
        <path data-s="2:0" data-a="draw" data-d="1.0" d="M420 450 H860 V700" fill="none" stroke={C.teal} strokeWidth={9} strokeLinejoin="round" />
        <path data-s="2:1" data-a="draw" data-d="1.0" d="M860 700 H1060 V660 H1500" fill="none" stroke={C.teal} strokeWidth={9} strokeLinejoin="round" />
        <g data-s="2:0+0.6" data-a="pop"><circle cx={860} cy={450} r={30} fill={C.ochre} /><text x={860} y={461} textAnchor="middle" style={label(30, C.ink, 800)}>1</text></g>
        <g data-s="2:1" data-a="pop"><circle cx={860} cy={700} r={30} fill={C.ochre} /><text x={860} y={711} textAnchor="middle" style={label(30, C.ink, 800)}>2</text></g>
        <circle className="walker" cx={0} cy={0} r={16} fill={C.teal} stroke="#fff" strokeWidth={4} style={{opacity: step >= 2 ? undefined : 0}} />
        <g data-s="3:0" data-a="fade">{[[820, 430], [835, 470], [800, 455], [850, 680], [830, 720]].map(([x, y], i) =>
          <circle key={i} cx={x} cy={y} r={13} fill={C.terracotta} stroke="#fff" strokeWidth={3} />)}
          <text x={600} y={560} textAnchor="middle" style={{...label(32, C.terracotta, 700), ...halo('#EADCC0')}}>attackers bunch at the turns</text></g>
        <g data-s="3:1" data-a="fade">{[[900, 360], [900, 600], [1010, 760]].map(([x, y], i) =>
          <path key={i} d={`M${x} ${y - 18} L${x + 18} ${y + 14} L${x - 18} ${y + 14} Z`} fill={C.ink} />)}
          <text x={1500} y={560} textAnchor="middle" style={{...label(32, C.ink, 700), ...halo('#E3D8BC')}}>defenders waiting</text></g>
      </Svg>
    </AbsoluteFill>
  </Board>;
};

/* ---------------------------------------------------------------- mirror light */
export const MirrorLight: React.FC<P> = ({shot}) => {
  const scope = useGsap(ctx => stepReveal(ctx, shot, 1));
  const cx = 960, cy = 800, R = 560;
  const tiles = Array.from({length: 27}, (_, i) => { const a = Math.PI * (0.06 + 0.88 * i / 26); return {a, x: cx - Math.cos(a) * R, y: cy - Math.sin(a) * R * 0.95}; });
  return <Board dark tag="SHEESH MAHAL · ONE FLAME, MANY REFLECTIONS" legend="SIMPLIFIED: CONVEX GLASS PIECES SET IN PLASTER">
    <AbsoluteFill ref={scope}>
      <Svg>
        <defs><radialGradient id="glow"><stop offset="0" stopColor="#FFD98A" stopOpacity=".9" /><stop offset="1" stopColor="#FFD98A" stopOpacity="0" /></radialGradient></defs>
        <path d={`M${cx - R - 40} ${cy} A${R + 40} ${(R + 40) * 0.95} 0 0 1 ${cx + R + 40} ${cy}`} fill="none" stroke="#5C4A3A" strokeWidth={26} />
        {tiles.map((t, i) => <rect key={i} x={t.x - 17} y={t.y - 11} width={34} height={22} rx={4} fill="#9AA7AE" stroke="#D6E0E4" strokeWidth={2}
          transform={`rotate(${(t.a * 180) / Math.PI - 90} ${t.x} ${t.y})`} />)}
        <g data-s="1:0-0.2" data-a="pop"><circle cx={cx} cy={cy - 40} r={150} fill="url(#glow)" />
          <path d={`M${cx} ${cy - 92} C${cx + 26} ${cy - 54} ${cx + 18} ${cy - 22} ${cx} ${cy - 18} C${cx - 18} ${cy - 22} ${cx - 26} ${cy - 54} ${cx} ${cy - 92} Z`} fill="#FFC75A" />
          <rect x={cx - 34} y={cy - 18} width={68} height={22} rx={6} fill="#B07A3A" /></g>
        {tiles.map((t, i) => <path key={i} data-s={`1:0+${(0.25 + i * 0.03).toFixed(2)}`} data-a="draw" data-d="0.4" d={`M${cx} ${cy - 55} L${t.x} ${t.y}`} stroke="rgba(255,210,120,.45)" strokeWidth={2} fill="none" />)}
        {tiles.map((t, i) => <g key={i} data-s={`1:1+${(i * 0.035).toFixed(2)}`} data-a="pop">
          <circle cx={t.x} cy={t.y} r={26} fill="url(#glow)" /><circle cx={t.x} cy={t.y} r={6} fill="#FFE7AE" /></g>)}
        <text data-s="1:0+0.4" x={cx} y={cy + 120} textAnchor="middle" style={label(36, C.white, 600)}>one lamp</text>
        <text data-s="1:1+0.8" x={cx} y={170} textAnchor="middle" style={label(40, '#FFD98A', 700)}>every mirror holds its own flame</text>
      </Svg>
    </AbsoluteFill>
  </Board>;
};

/* ---------------------------------------------------------------- Sukh Niwas cooling */
export const Cooling: React.FC<P> = ({shot}) => {
  const step = stepOf(shot);
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const scope = useGsap(ctx => stepReveal(ctx, shot, step));
  const flow = -(frame / fps) * 60;
  const ground = 820;
  return <Board tag="SUKH NIWAS · HOW THE ROOM WAS COOLED" legend="SIMPLIFIED SECTION · NOT TO SCALE">
    <AbsoluteFill ref={scope}>
      <Svg>
        <path d={`M200 ${ground} H1560`} stroke={C.ink} strokeWidth={5} />
        <path d={`M200 ${ground} V280 H300 V${ground}`} fill="#E3D2B0" stroke={C.ink} strokeWidth={5} />
        <path d="M300 280 H1380 V330 H300" fill="#E3D2B0" stroke={C.ink} strokeWidth={5} />
        <path d={`M1380 330 V560 M1380 ${ground} V700`} stroke={C.ink} strokeWidth={8} />
        <text x={760} y={260} textAnchor="middle" style={mono(22)}>SUKH NIWAS</text>
        <text x={1690} y={ground - 20} textAnchor="middle" style={mono(22)}>GARDEN</text>
        <g data-s="1:0-0.2" data-a="up">
          <path d="M310 400 L470 760" stroke="#F4F1EA" strokeWidth={34} strokeLinecap="round" />
          <path d="M310 400 L470 760" stroke="#C9C3B6" strokeWidth={34} strokeDasharray="6 10" strokeLinecap="round" opacity={0.7} />
          <text x={560} y={430} style={{...label(30), ...halo()}}>pierced marble slab</text>
        </g>
        <g data-s="1:1" data-a="fade">
          <path d="M318 392 L476 758" stroke={C.water} strokeWidth={10} strokeDasharray="22 14" strokeDashoffset={flow} strokeLinecap="round" fill="none" />
          <text x={560} y={480} style={{...label(30, C.water, 700), ...halo()}}>water cascades over it</text>
        </g>
        {[0, 1, 2].map(i => <g key={i}>
          <path data-s={`2:0${i ? '+' + i * 0.2 : '-0.6'}`} data-a="draw" d={`M1340 ${470 + i * 70} C1000 ${470 + i * 70} 700 ${520 + i * 60} 430 ${560 + i * 50}`} fill="none" stroke={C.terracotta} strokeWidth={7} strokeLinecap="round" />
          <path data-s={`2:0+${0.8 + i * 0.2}`} data-a="draw" d={`M430 ${590 + i * 50} C620 ${640 + i * 40} 900 ${650 + i * 40} 1340 ${640 + i * 50}`} fill="none" stroke={C.teal} strokeWidth={7} strokeLinecap="round" />
        </g>)}
        <text data-s="2:0-0.4" x={1060} y={430} textAnchor="middle" style={{...label(30, C.terracotta, 700), ...halo()}}>hot air in</text>
        <text data-s="2:0+1.2" x={1060} y={790} textAnchor="middle" style={{...label(30, C.teal, 700), ...halo()}}>cooler air out</text>
        <g data-s="2:0+0.5" data-a="fade">{[0, 1, 2, 3].map(i => <path key={i} d={`M${380 + i * 26} ${700 - i * 70} q 12 -20 0 -40 q -12 -20 0 -40`} fill="none" stroke="#9CC3CC" strokeWidth={4} />)}
          <text x={300} y={880} style={{...label(26, C.inkSoft, 600)}}>evaporation absorbs heat</text></g>
        <path data-s="3:0" data-a="draw" d={`M470 ${ground - 8} H1720`} stroke={C.water} strokeWidth={14} strokeLinecap="round" />
        <path data-s="3:0+0.6" d={`M470 ${ground - 8} H1720`} stroke="#BFE0E8" strokeWidth={5} strokeDasharray="18 16" strokeDashoffset={flow * 1.5} fill="none" />
        <text data-s="3:0+0.3" x={900} y={ground + 50} textAnchor="middle" style={label(28, C.water, 700)}>marble channel through the floor</text>
        <g data-s="3:1" data-a="up">
          <path d={`M1690 ${ground - 8} V720`} stroke={C.water} strokeWidth={6} />
          {[-1, 0, 1].map(k => <path key={k} d={`M1690 720 q ${k * 40} -60 ${k * 80} 10`} fill="none" stroke="#7FB4C4" strokeWidth={5} />)}
          {[1600, 1780].map(x => <g key={x}><path d={`M${x} ${ground} V760`} stroke="#6B5B3E" strokeWidth={6} /><circle cx={x} cy={740} r={34} fill="#6E8F5E" /></g>)}
        </g>
      </Svg>
    </AbsoluteFill>
  </Board>;
};
