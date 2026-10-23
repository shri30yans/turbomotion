import React, {useLayoutEffect, useRef, useState} from 'react';
import {cancelRender, continueRender, delayRender, staticFile} from 'remotion';

/**
 * A photograph rendered through WebGL so a depth map can give it real parallax
 * (2.5D camera): near pixels shift and scale more than far ones as the camera moves.
 * Colour grade and vignette are applied in the same pass. Without a depth map the
 * same shader produces a plain, graded camera move.
 */
const VERT = `attribute vec2 p; varying vec2 v; void main(){ v = p*0.5+0.5; gl_Position = vec4(p,0.,1.); }`;
const FRAG = `precision highp float;
uniform sampler2D img; uniform sampler2D dep; uniform vec2 res; uniform vec4 rect;
uniform vec2 shift; uniform float dolly; uniform vec2 focus; uniform float hasDepth; uniform float grade; uniform float vig; uniform float dim;
varying vec2 v;
float depthAt(vec2 uv){ return hasDepth > .5 ? texture2D(dep, clamp(uv, .001, .999)).r : .5; }
vec2 warp(vec2 uv, float d){ float k = d - .5; vec2 u = uv - shift*k; return focus + (u - focus)/(1. + dolly*k); }
void main(){
  vec2 px = vec2(v.x*res.x, (1.-v.y)*res.y);
  vec2 uv = (px - rect.xy)/rect.zw;
  float d = depthAt(uv);
  vec2 s = warp(uv, d);
  d = depthAt(s);            // one refinement step reduces edge tearing
  s = warp(uv, d);
  vec3 c = texture2D(img, clamp(s, .0005, .9995)).rgb;
  float l = dot(c, vec3(.299,.587,.114));
  if (grade < .5) {          // warm documentary grade
    c = mix(vec3(l), c, 1.06);
    c = c*c*(3.-2.*c)*.25 + c*.75;
    c *= vec3(1.035, 1.0, .955);
    c = c*.97 + .018;
  } else if (grade < 1.5) {  // archival: sepia, lifted blacks
    vec3 sep = vec3(l*1.07+.04, l*.96+.025, l*.80);
    c = mix(sep, sep*sep*(3.-2.*sep), .5)*.92 + .045;
  } else {                   // painting / illustration: neutral, light protection
    c = c*.985 + .01;
  }
  vec2 q = v - .5;
  c *= 1. - vig*smoothstep(.35, .95, length(q*vec2(1.,.82)));
  c *= 1. - dim;
  gl_FragColor = vec4(c, 1.);
}`;

type Props = {
  src: string; depth?: string; width: number; height: number;
  rect: {x: number; y: number; w: number; h: number};
  shift: [number, number]; dolly: number; focus: [number, number];
  grade: 'warm' | 'archival' | 'painting'; vignette?: number; dim?: number; style?: React.CSSProperties;
};

const cache = new Map<string, Promise<HTMLImageElement>>();
function load(url: string) {
  if (!cache.has(url)) cache.set(url, new Promise((res, rej) => {
    const im = new Image();
    im.crossOrigin = 'anonymous';
    im.onload = () => im.decode().then(() => res(im), () => res(im));
    im.onerror = () => rej(new Error('image failed: ' + url));
    im.src = url;
  }));
  return cache.get(url)!;
}

export const DepthImage: React.FC<Props> = ({src, depth, width, height, rect, shift, dolly, focus, grade, vignette = .32, dim = 0, style}) => {
  const canvas = useRef<HTMLCanvasElement>(null);
  const gl = useRef<{g: WebGLRenderingContext; prog: WebGLProgram; u: Record<string, WebGLUniformLocation | null>} | null>(null);
  const [ready, setReady] = useState(false);
  const [gate] = useState(() => delayRender('depth image ' + src));
  useLayoutEffect(() => {
    let dead = false;
    Promise.all([load(staticFile(src)), depth ? load(staticFile(depth)) : Promise.resolve(null)]).then(([im, dm]) => {
      if (dead) return;
      const c = canvas.current!;
      const g = c.getContext('webgl', {preserveDrawingBuffer: true, antialias: false, premultipliedAlpha: false})!;
      const sh = (type: number, srcCode: string) => {
        const s = g.createShader(type)!; g.shaderSource(s, srcCode); g.compileShader(s);
        if (!g.getShaderParameter(s, g.COMPILE_STATUS)) throw new Error(g.getShaderInfoLog(s) || 'shader');
        return s;
      };
      const prog = g.createProgram()!;
      g.attachShader(prog, sh(g.VERTEX_SHADER, VERT)); g.attachShader(prog, sh(g.FRAGMENT_SHADER, FRAG));
      g.linkProgram(prog); g.useProgram(prog);
      const buf = g.createBuffer(); g.bindBuffer(g.ARRAY_BUFFER, buf);
      g.bufferData(g.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), g.STATIC_DRAW);
      const loc = g.getAttribLocation(prog, 'p'); g.enableVertexAttribArray(loc); g.vertexAttribPointer(loc, 2, g.FLOAT, false, 0, 0);
      const tex = (unit: number, source: HTMLImageElement | null) => {
        const t = g.createTexture(); g.activeTexture(g.TEXTURE0 + unit); g.bindTexture(g.TEXTURE_2D, t);
        g.pixelStorei(g.UNPACK_FLIP_Y_WEBGL, false);
        if (source) g.texImage2D(g.TEXTURE_2D, 0, g.RGB, g.RGB, g.UNSIGNED_BYTE, source);
        else g.texImage2D(g.TEXTURE_2D, 0, g.RGB, 1, 1, 0, g.RGB, g.UNSIGNED_BYTE, new Uint8Array([128, 128, 128]));
        for (const [k, val] of [[g.TEXTURE_MIN_FILTER, g.LINEAR], [g.TEXTURE_MAG_FILTER, g.LINEAR], [g.TEXTURE_WRAP_S, g.CLAMP_TO_EDGE], [g.TEXTURE_WRAP_T, g.CLAMP_TO_EDGE]])
          g.texParameteri(g.TEXTURE_2D, k, val);
      };
      tex(0, im); tex(1, dm);
      const names = ['img', 'dep', 'res', 'rect', 'shift', 'dolly', 'focus', 'hasDepth', 'grade', 'vig', 'dim'];
      const u = Object.fromEntries(names.map(n => [n, g.getUniformLocation(prog, n)]));
      g.uniform1i(u.img, 0); g.uniform1i(u.dep, 1); g.uniform1f(u.hasDepth, dm ? 1 : 0);
      gl.current = {g, prog, u};
      setReady(true);
      continueRender(gate);
    }).catch(cancelRender);
    return () => { dead = true; };
  }, [src, depth, gate]);
  useLayoutEffect(() => {
    const s = gl.current;
    if (!s || !ready) return;
    const {g, u} = s;
    g.viewport(0, 0, width, height);
    g.uniform2f(u.res, width, height);
    g.uniform4f(u.rect, rect.x, rect.y, rect.w, rect.h);
    g.uniform2f(u.shift, shift[0], shift[1]);
    g.uniform1f(u.dolly, dolly);
    g.uniform2f(u.focus, focus[0], focus[1]);
    g.uniform1f(u.grade, grade === 'warm' ? 0 : grade === 'archival' ? 1 : 2);
    g.uniform1f(u.vig, vignette);
    g.uniform1f(u.dim, dim);
    g.drawArrays(g.TRIANGLE_STRIP, 0, 4);
  });
  return <canvas ref={canvas} width={width} height={height} style={{position: 'absolute', inset: 0, width, height, ...style}} />;
};
