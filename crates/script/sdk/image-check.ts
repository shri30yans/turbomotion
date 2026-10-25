// `image()` options, checked where the author wrote them instead of by the engine's schema error at render time.
import type { ImageOpts } from "./types";

const FITS = ["cover", "contain", "fill", "none"];
const SAMPLING = ["nearest", "linear"];
const EXTENSIONS = /\.(png|jpe?g|webp|gif|bmp|tiff?|avif|heic)$/i;
const pair = (v: unknown): v is [number, number] => Array.isArray(v) && v.length === 2 && v.every((n) => typeof n === "number" && Number.isFinite(n));

/** Throws a message naming the option and the fix; returns `src` so it can be used inline. */
export function checkImage(src: string, o: ImageOpts): string {
  const fail = (m: string): never => { throw new Error(`image("${src}"): ${m}`); };
  if (typeof src !== "string" || !src) throw new Error('image(src, opts): src must be a file path like "photo.jpg"');
  if (!EXTENSIONS.test(src) && !/^[a-z]+:/.test(src)) fail('src must be an image file (png, jpg, webp, gif...); for vector artwork use svg() or svgFile()');
  if (o.fit !== undefined && !FITS.includes(o.fit)) fail(`fit must be one of ${FITS.join(", ")}, got ${JSON.stringify(o.fit)}`);
  if (o.sampling !== undefined && !SAMPLING.includes(o.sampling)) fail(`sampling must be "nearest" or "linear", got ${JSON.stringify(o.sampling)}`);
  if (o.size !== undefined && !(pair(o.size) && o.size[0] > 0 && o.size[1] > 0)) fail("size must be [width, height] in pixels, both above 0");
  if (o.focus !== undefined && !(pair(o.focus) && o.focus.every((n) => n >= 0 && n <= 1))) fail("focus must be [x, y] as fractions of the image, 0 to 1");
  for (const end of ["from", "to"] as const) {
    const f = o.camera?.[end];
    if (o.camera && !f) fail('camera needs { from: { zoom, center }, to: { zoom, center } }');
    if (f?.zoom !== undefined && !(f.zoom > 0)) fail(`camera.${end}.zoom must be above 0`);
    if (f?.center !== undefined && !pair(f.center)) fail(`camera.${end}.center must be [x, y] as fractions of the image`);
  }
  if (o.parallax && (typeof o.parallax.depth !== "string" || o.parallax.strength === undefined)) fail("parallax needs { depth: \"depth-map.png\", strength }");
  return src;
}
