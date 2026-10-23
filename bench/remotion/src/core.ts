import {useLayoutEffect, useRef, useState} from 'react';
import {cancelRender, continueRender, delayRender, staticFile, useCurrentFrame, useVideoConfig} from 'remotion';
import {gsap} from 'gsap';
import {SplitText} from 'gsap/SplitText';
import {DrawSVGPlugin} from 'gsap/DrawSVGPlugin';
import {MotionPathPlugin} from 'gsap/MotionPathPlugin';
import {CustomEase} from 'gsap/CustomEase';

gsap.registerPlugin(SplitText, DrawSVGPlugin, MotionPathPlugin, CustomEase);
CustomEase.create('settle', '0.22,1,0.36,1');
CustomEase.create('camera', '0.45,0.05,0.35,1');

/* ------------------------------------------------------------------ design tokens */
export const C = {
  paper: '#F4EBD9', paperDeep: '#E9DCC1', ink: '#2B1E16', inkSoft: '#6E5A47',
  terracotta: '#B4482F', ochre: '#E0A93B', teal: '#1F6F68', sky: '#4F86A6', water: '#3E7FA0',
  night: '#16110D', white: '#FFF8EC', line: '#CDB995',
};
export const F = {display: 'FrauncesFilm, Georgia, serif', sans: 'ManropeFilm, Helvetica, sans-serif', mono: 'PlexFilm, Menlo, monospace'};

/* ------------------------------------------------------------------ film data types */
export type Shot = {
  id: string; chapter: number; type: string; args: string[]; params: Record<string, string>;
  text: string; cues: number; claims: string[]; startFrame: number; durationInFrames: number;
  duration: number; cueTimes: number[]; cueNames?: Record<string, number>; cueStatus?: string[]; beat?: string | null; words: {w: string; t: number}[];
};
export type Asset = {file: string; width: number; height: number; author?: string; license?: string; kind?: string; depth?: string};
export type Film = {
  title: string; fps: number; width: number; height: number; durationInFrames: number;
  chapters: {id: string; title: string; startFrame: number}[]; shots: Shot[]; assets: Record<string, Asset>;
  narration?: string;
  music?: string;
};

/** Cue time i for a shot, with sensible fallbacks when the script has fewer cues. */
export function cueAt(shot: Shot, ref: number | string, fallback?: number) {
  let i: number;
  if (typeof ref === 'string') {
    const n = shot.cueNames?.[ref];
    if (n === undefined) throw new Error(`${shot.id}: no cue named "${ref}" (has ${Object.keys(shot.cueNames || {}).join(', ') || 'none'})`);
    i = n;
  } else i = ref;
  if (shot.cueTimes[i] !== undefined) return Math.max(0.05, shot.cueTimes[i]);
  if (fallback !== undefined) return fallback;
  const last = shot.cueTimes.length ? shot.cueTimes[shot.cueTimes.length - 1] : 0.3;
  return Math.min(shot.duration - 0.6, last + 0.9 * (i - shot.cueTimes.length + 1));
}
export const num = (v: string | undefined, d: number) => (v === undefined || v === '' ? d : Number(v));
export const triple = (v: string | undefined, d: [number, number, number]): [number, number, number] => {
  if (!v) return d;
  const p = v.split(',').map(Number);
  return [p[0] ?? d[0], p[1] ?? d[1], p[2] ?? d[2]];
};

/* ------------------------------------------------------------------ fonts */
let fontsReady: Promise<void> | undefined;
export function loadFonts() {
  if (!fontsReady) {
    const faces = [
      new FontFace('FrauncesFilm', `url("${staticFile('_engine/fonts/Fraunces.ttf')}")`, {weight: '100 900'}),
      new FontFace('ManropeFilm', `url("${staticFile('_engine/fonts/manrope.ttf')}")`, {weight: '200 800'}),
      new FontFace('PlexFilm', `url("${staticFile('_engine/fonts/plexmono.ttf')}")`, {weight: '400'}),
    ];
    fontsReady = Promise.all(faces.map(f => f.load())).then(loaded => {
      loaded.forEach(f => (document.fonts as unknown as {add: (f: FontFace) => void}).add(f));
      return document.fonts.ready;
    }).then(() => undefined);
  }
  return fontsReady;
}

/* ------------------------------------------------------------------ deterministic GSAP */
export type Ctx = {
  tl: gsap.core.Timeline;
  q: (sel: string) => Element[];
  split: (target: gsap.DOMTarget, vars: SplitText.Vars) => SplitText;
};
/**
 * One paused GSAP timeline per mounted shot, seeked to the Remotion frame.
 * Fonts load and text is split before the first frame is released. The timeline is
 * fully resolved once (end then start) so arbitrary and reverse seeks are stable.
 */
export function useGsap(build: (ctx: Ctx) => void) {
  const scope = useRef<HTMLDivElement>(null);
  const tlRef = useRef<gsap.core.Timeline | null>(null);
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  const clock = useRef(frame / fps);
  clock.current = frame / fps;
  const [gate] = useState(() => delayRender('fonts + gsap timeline', {timeoutInMilliseconds: 60000}));
  const buildRef = useRef(build);
  buildRef.current = build;
  useLayoutEffect(() => {
    let dead = false;
    let ctx: gsap.Context | undefined;
    loadFonts().then(() => {
      if (dead) return;
      ctx = gsap.context(() => {
        const tl = gsap.timeline({paused: true, defaults: {ease: 'settle', duration: 0.7, force3D: false}});
        tlRef.current = tl;
        const sel = gsap.utils.selector(scope);
        buildRef.current({tl, q: (s: string) => sel(s) as Element[], split: (t, v) => SplitText.create(t, {autoSplit: false, ...v})});
        tl.totalTime(tl.duration(), true);
        tl.totalTime(0, true);
        tl.totalTime(clock.current, true);
      }, scope);
      continueRender(gate);
    }).catch(cancelRender);
    return () => { dead = true; ctx?.revert(); tlRef.current = null; };
  }, [gate]);
  useLayoutEffect(() => {
    const tl = tlRef.current;
    if (tl) { tl.totalTime(0, true); tl.totalTime(frame / fps, true); }
  }, [frame, fps]);
  return scope;
}

/** Reveal helpers shared by every shot type (one motion language for the film). */
/** Masks clip at the line box; pad them so descenders (g, y, p) and italic overhangs survive. */
function padMasks(masks: Element[] | undefined) {
  (masks || []).forEach(m => {
    const st = (m as HTMLElement).style;
    st.padding = '0.06em 0.14em 0.26em 0.14em';
    st.margin = '-0.06em -0.14em -0.26em -0.14em';
  });
}

export function motion({tl, q, split}: Ctx) {
  const lines = (sel: string, at: number, stagger = 0.08, duration = 0.75) => {
    const els = q(sel);
    if (!els.length) return;
    const s = split(els, {type: 'lines', mask: 'lines', linesClass: 'm-line'});
    padMasks(s.masks as Element[]);
    tl.fromTo(s.lines, {yPercent: 110}, {yPercent: 0, stagger, duration, ease: 'settle', immediateRender: true}, at);
  };
  const chars = (sel: string, at: number, stagger = 0.022) => {
    const els = q(sel);
    if (!els.length) return;
    const s = split(els, {type: 'lines,chars', mask: 'lines', charsClass: 'm-char'});
    padMasks(s.masks as Element[]);
    tl.fromTo(s.chars, {yPercent: 105}, {yPercent: 0, stagger, duration: 0.6, ease: 'settle', immediateRender: true}, at);
  };
  const fadeUp = (sel: string | Element[], at: number, y = 18, duration = 0.6) => {
    const els = typeof sel === 'string' ? q(sel) : sel;
    if (!els.length) return;
    tl.fromTo(els, {opacity: 0, y}, {opacity: 1, y: 0, duration, ease: 'settle', immediateRender: true}, at);
  };
  const pop = (sel: string | Element[], at: number) => {
    const els = typeof sel === 'string' ? q(sel) : sel;
    if (!els.length) return;
    tl.fromTo(els, {opacity: 0, scale: 0.6}, {opacity: 1, scale: 1, duration: 0.45, ease: 'back.out(2.2)', transformOrigin: '50% 50%', immediateRender: true}, at);
  };
  const draw = (sel: string | Element[], at: number, duration = 0.8, ease = 'power2.inOut') => {
    const els = typeof sel === 'string' ? q(sel) : sel;
    if (!els.length) return;
    tl.fromTo(els, {drawSVG: '0%'}, {drawSVG: '100%', duration, ease, immediateRender: true}, at);
  };
  const show = (sel: string | Element[], at: number, duration = 0.45) => {
    const els = typeof sel === 'string' ? q(sel) : sel;
    if (!els.length) return;
    tl.fromTo(els, {opacity: 0}, {opacity: 1, duration, ease: 'power1.out', immediateRender: true}, at);
  };
  return {lines, chars, fadeUp, pop, draw, show};
}

/* ------------------------------------------------------------------ camera */
/** Cover-fit an image of aspect `ar` in a W×H frame, zoomed by z, centred on (cx,cy). */
export function camera(ar: number, W: number, H: number, cx: number, cy: number, z: number) {
  const base = ar > W / H ? {w: H * ar, h: H} : {w: W, h: W / ar};
  const w = base.w * z, h = base.h * z;
  const x = Math.min(0, Math.max(W - w, W / 2 - cx * w));
  const y = Math.min(0, Math.max(H - h, H / 2 - cy * h));
  return {x, y, w, h};
}
export const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
export const easeCam = (t: number) => (t < 0 ? 0 : t > 1 ? 1 : t * t * (3 - 2 * t) * 0.35 + t * 0.65);
