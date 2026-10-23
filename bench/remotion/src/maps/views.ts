/**
 * Map definitions. Times: number = seconds into the shot, "cN" = the shot's Nth cue word,
 * "cN+0.5" = offset. `step` lets several consecutive shots build one map: elements from
 * earlier steps are already on screen, the current step's elements animate on their cues.
 * Film-specific maps live here for now; add a project-level maps file when a second film needs it.
 */
export type When = number | string;
export type MapSpec = {
  title?: string; legend?: string; note?: string; home?: string[];
  views: {center: [number, number]; span: number; at?: When; dur?: number; step?: number}[];
  pins?: {place: string; label?: string; sub?: string; at?: When; step?: number; style?: 'home'; dx?: number; dy?: number}[];
  routes?: {via: string[]; at?: When; dur?: number; step?: number; label?: string; dashed?: boolean; color?: 'teal' | 'terracotta'; bend?: number; labelDx?: number; labelDy?: number}[];
  rings?: {center: [number, number]; km: number; label?: string; at?: When; step?: number}[];
  areas?: {state: string; at?: When; step?: number}[];
  labels?: {text: string; at_: [number, number]; at?: When; step?: number; size?: number; faint?: boolean}[];
  schematic?: {kind: 'hills' | 'lake' | 'grid'; pts: number[][]; at?: When; step?: number}[];
};

import data from '../../../../assets/maps/views.json';

/** [lon, lat] */
export const PLACES = data.places as unknown as Record<string, [number, number]>;
export const MAPS = data.maps as unknown as Record<string, MapSpec>;
