// Layout with the exact font shaping used by the composition renderer.
export interface MeasureOptions {
  font?: string;
  size?: number;
  weight?: number;
  italic?: boolean;
  tracking?: number;
  leading?: number;
  width?: number;
  align?: "left" | "center" | "right";
}
export interface CharBox { ch: string; x: number; w: number; line: number }
export interface WordBox { text: string; x: number; w: number; line: number }
export interface Measured {
  width: number; height: number; baseline: number; lines: number;
  chars: CharBox[]; words: WordBox[];
}
declare const __tm_measure: (request: string, fonts: string, root: string) => string;
let fonts: Record<string, string> = {};
let root = "";
/** Called by comp() so font roles and relative files agree with the film. */
export function configureMeasure(roles: Record<string, string> = {}, directory = ""): void {
  fonts = { ...roles };
  root = directory;
}
/** Pixel dimensions and character/word boxes, using the most recently constructed comp's fonts. */
export function measure(text: string, options: MeasureOptions = {}): Measured {
  if (typeof text !== "string") throw new Error("measure(text, options): text must be a string");
  return JSON.parse(__tm_measure(JSON.stringify({ ...options, text }), JSON.stringify(fonts), root));
}
/** The asset root the latest comp() declared (relative to the script's folder). */
export const filmRoot = (): string => root;
