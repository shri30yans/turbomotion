import { shot, text } from "turbomotion";
import type { __TYPE__ } from "./types";

export type { __TYPE__ } from "./types";

/** The __NAME__ scene. Replace the label with the scene's layers; keep one responsibility per file as the scene grows. */
export function __FUNCTION__({ id, dur }: __TYPE__) {
  const label = text("__NAME__", { id: `${id}-label`, pos: [960, 540], align: "center", size: 96, color: "#ffffff" });
  return shot({ id, dur, bg: "#000000", layers: [label] });
}
