// A frosted-glass panel: a soft shadow, a translucent body with a sheen, and a bright rim. When the engine offers a
// `backdrop` effect, `blur` asks the body to blur whatever is behind it; without one the panel is translucent only.

import { shape } from "../layers";
import type { Layer } from "../layers";
import type { Color, LayerOpts, Vec2 } from "../types";
import { alpha, Part, shadow } from "./util";

export interface GlassPanelOpts extends LayerOpts {
  id: string;
  /** Centre of the panel. */
  pos: Vec2;
  size: Vec2;
  radius?: number;
  /** Colour of the glass (white). */
  tint?: Color;
  /** Opacity of the glass body, 0 to 1 (0.16). */
  frost?: number;
  rim?: Color;
  /** Radius in px of the engine's `backdrop` blur behind the body; leave out when the engine has none. */
  blur?: number;
  /** A drop shadow under the panel. */
  shadow?: boolean;
}

export interface GlassPanelPart extends Part {
  body: Layer;
}

/** `glassPanel({ id: "card", pos: [960, 540], size: [640, 360], radius: 28, tint: "#cfe1ff" })`. */
export function glassPanel(o: GlassPanelOpts): GlassPanelPart {
  const { id, pos, size: [w, h], radius = 24, tint = "#ffffff", frost = 0.16, rim = "#ffffff", blur, shadow: shade = true, ...layer } = o;
  const tintHex = typeof tint === "string" ? tint : "#ffffff";
  const rect = [-w / 2, -h / 2, w, h];
  const body = shape([
    { rect, radius, fill: { gradient: { type: "linear", from: [0, -h / 2], to: [0, h / 2], stops: [[0, alpha(tintHex, frost * 1.8)], [0.5, alpha(tintHex, frost)], [1, alpha(tintHex, frost * 0.6)]] } }, stroke: { color: alpha(typeof rim === "string" ? rim : "#ffffff", 0.45), width: 1.5 } },
    { rect: [-w / 2 + radius * 0.6, -h / 2 + 1.5, w - radius * 1.2, 2], radius: 1, fill: { color: "#ffffff", opacity: 0.55 } },
  ], { ...layer, id, pos, ...(blur ? { fx: [{ effect: "backdrop", blur }] } : {}) });
  const under = shade ? [shape([{ rect, radius, fill: "#000000" }], { id: `${id}-shadow`, pos, opacity: 0.35, fx: [shadow(48, 28, 0.5)] })] : [];
  return { layers: [...under, body], body, end: 0 };
}
