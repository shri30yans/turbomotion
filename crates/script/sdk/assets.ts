// Artwork from code: SVG through the engine's importer (`svg`, `svgFile`) and pixel art from rows of characters.
import { Layer } from "./layers";
import { filmRoot } from "./measure";
import type { Color, LayerOpts } from "./types";

declare const __tm_svg: (markup: string, id: string, fit: number) => string;
declare const __tm_read: (root: string, name: string) => string;

export interface SvgOpts extends Omit<LayerOpts, "scale"> {
  /** Pixel width of the artwork in the composition; its natural size (viewBox width) when left out. */
  width?: number;
  /** Multiplies the fitted size (a number, or `[sx, sy]`). */
  scale?: number | [number, number];
  /** Recolour every filled / stroked item. */
  fill?: Color;
  stroke?: Color;
  /** Path data only: the coordinate box the path is drawn in (24 x 24). */
  viewBox?: [number, number, number, number];
  /** Path data only: stroke width in viewBox units. */
  strokeWidth?: number;
}

let counter = 0;
const natural = (markup: string): number => {
  const box = /viewBox\s*=\s*"\s*[-\d.]+[ ,]+[-\d.]+[ ,]+([\d.]+)/.exec(markup);
  const width = /<svg[^>]*\swidth\s*=\s*"([\d.]+)/.exec(markup);
  return Number(box?.[1] ?? width?.[1] ?? 100);
};
const hex = (c: Color): string => (typeof c === "string" ? c : `rgb(${c.slice(0, 3).join(",")})`);

/**
 * SVG markup (`"<svg ...>"`) or bare path data (`"M0 0 L10 0 ..."`) as a shape layer; its `anchor` is the artwork's centre.
 * @example
 * import { comp, svg } from "turbomotion";
 * const film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: "#1a1d29" });
 * film.add(svg("<svg viewBox='0 0 10 10'><path d='M5 0 L10 10 L0 10 Z' fill='#ffd166'/></svg>", { id: "triangle", width: 400, pos: [960, 540] }));
 * export default film;
 */
export function svg(source: string, o: SvgOpts = {}): Layer {
  if (typeof source !== "string" || !source.trim()) throw new Error('svg(source, opts): give SVG markup like "<svg viewBox=...>" or path data like "M0 0 L10 10"');
  if (!source.trim().startsWith("<") && !/^[Mm]\s*-?[\d.]/.test(source.trim())) throw new Error(`svg(): "${source.slice(0, 24)}" is neither SVG markup (starts with "<") nor path data (starts with "M" and a number)`);
  const { width, scale, fill, stroke, viewBox = [0, 0, 24, 24], strokeWidth = 2, ...layer } = o;
  const markup = source.trim().startsWith("<") ? source
    : `<svg viewBox="${viewBox.join(" ")}"><path d="${source}" fill="${stroke && !fill ? "none" : hex(fill ?? "#ffffff")}"${stroke ? ` stroke="${hex(stroke)}" stroke-width="${strokeWidth}"` : ""}/></svg>`;
  const id = layer.id ?? `svg${++counter}`;
  const imported = JSON.parse(__tm_svg(markup, id, width ?? natural(markup))) as { layer: Record<string, any>; warnings: string[] };
  for (const w of imported.warnings) console.warn(`svg(): ${w}`);
  const { type: _type, ...props } = imported.layer;
  if (scale !== undefined) props.scale = Array.isArray(scale) ? scale.map((s) => s * props.scale) : props.scale * scale;
  if (fill !== undefined || stroke !== undefined) {
    props.items = props.items.map((item: Record<string, any>) => ({
      ...item, ...(fill !== undefined && item.fill !== undefined ? { fill } : {}),
      ...(stroke !== undefined && item.stroke !== undefined ? { stroke: { ...item.stroke, color: stroke } } : {}),
    }));
  }
  return new Layer("shape", { ...props, ...layer });
}

/** An `.svg` file next to the film (or under its `root`), imported like `svg()`. The file is watched by `sdk watch`. */
export function svgFile(path: string, o: SvgOpts = {}): Layer {
  if (typeof path !== "string" || !path.endsWith(".svg")) throw new Error('svgFile(path, opts): path must name an .svg file relative to the film, like "icons/play.svg"');
  return svg(__tm_read(filmRoot(), path), { id: `svg-${path.replace(/[^a-z0-9]+/gi, "-")}-${++counter}`, ...o });
}

export interface PixelOpts extends LayerOpts {
  /** Screen pixels per sprite pixel (8). Use whole numbers and whole positions for crisp edges. */
  px?: number;
  /** Overlap between neighbouring runs, hiding hairline seams at fractional positions (0.3). */
  bleed?: number;
}

/**
 * Pixel art from rows of characters: `pixelSprite(["..aa..", ".aaaa."], { a: "#ff5a1f" }, { px: 6 })`. `.` and space are clear; centred on its position.
 * @example
 * import { comp, pixelSprite } from "turbomotion";
 * const film = comp({ width: 1920, height: 1080, fps: 30, dur: 3, bg: "#1a1d29" });
 * film.add(pixelSprite(
 *   ["..aa..", ".aaaa.", "aabbaa", "aaaaaa", ".a..a."],
 *   { a: "#ff5a1f", b: "#ffffff" },
 *   { id: "invader", px: 40, pos: [960, 540] },
 * ));
 * export default film;
 */
export function pixelSprite(rows: string[], palette: Record<string, Color>, o: PixelOpts = {}): Layer {
  const { px = 8, bleed = 0.3, ...layer } = o;
  if (!Array.isArray(rows) || !rows.length || rows.some((r) => typeof r !== "string")) throw new Error("pixelSprite(rows, palette): rows is a list of equal-length strings");
  if (!(px > 0)) throw new Error(`pixelSprite(): px must be above 0, got ${px}`);
  const cols = rows[0].length;
  rows.forEach((r, y) => { if (r.length !== cols) throw new Error(`pixelSprite(): row ${y} has ${r.length} characters but row 0 has ${cols}`); });
  const items: Array<Record<string, unknown>> = [];
  rows.forEach((row, y) => {
    for (let x = 0; x < cols;) {
      const ch = row[x];
      if (ch === "." || ch === " ") { x++; continue; }
      if (!(ch in palette)) throw new Error(`pixelSprite(): "${ch}" at row ${y}, column ${x} is not in the palette (${Object.keys(palette).join(" ")}); use "." for clear`);
      let end = x;
      while (end < cols && row[end] === ch) end++;
      items.push({ rect: [(x - cols / 2) * px, (y - rows.length / 2) * px, (end - x) * px + bleed, px + bleed], fill: palette[ch] });
      x = end;
    }
  });
  if (!items.length) throw new Error("pixelSprite(): every pixel is clear");
  return new Layer("shape", { items, ...layer });
}
