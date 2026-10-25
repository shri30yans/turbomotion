// A stylised glowing hand (vector artwork): forearm, palm, four fanned fingers and a thumb, pointing along +x from the wrist.

import { shape } from "../layers";
import type { Layer } from "../layers";
import type { Color, LayerOpts, Vec2 } from "../types";
import { capsulePath } from "./capsule";
import { THEME } from "./util";

export interface HandOpts extends LayerOpts {
  id: string;
  /** Where the wrist sits. */
  pos: Vec2;
  /** Degrees; 0 points right. */
  rotation?: number;
  fill?: Color;
  /** Outline colour. */
  rim?: Color;
  /** Glow colour as `[r, g, b]`. */
  glow?: [number, number, number];
  /** Mirror it (a left hand). */
  flip?: boolean;
}

/** `hand({ id: "h", pos: [400, 600], rotation: -20, fill: "#f3f0e8", rim: "#3D6BFF" })`. */
export function hand(o: HandOpts): Layer {
  const { flip, fill = THEME.paper, rim = THEME.cobalt, glow = [61, 107, 255], rotation = 0, fx, ...layer } = o;
  const s = flip ? -1 : 1;
  const fingers: Array<[number, number, number]> = [[-16, 112, 24], [-5, 128, 25], [6, 120, 24], [18, 98, 22]];
  const paths = [
    capsulePath(-190, 0, 200, 70, 0),
    capsulePath(10, 0, 120, 92, 0),
    ...fingers.map(([deg, len, w]) => capsulePath(110, deg * 1.1 * s, len, w, deg * s)),
    capsulePath(30, -34 * s, 92, 28, -52 * s),
  ];
  const glowFx = [{ effect: "blur", radius: 1.1 }, { effect: "glow", radius: 20, strength: 1.1, threshold: 0, color: glow }];
  return shape(paths.map((d) => ({ d, fill, stroke: { color: rim, width: 4 } })), { ...layer, rotation, fx: fx ?? glowFx });
}
