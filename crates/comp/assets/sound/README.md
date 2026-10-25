# Sound kit

About 230 procedurally synthesised sounds in 26 families, written as `kit/<family>/<name>.wav` (48 kHz, 16-bit, mono
where stereo adds nothing) with an index in `kit/index.json` (compiled into the engine). No samples, no licences.
Use them as `"src": "kit:<cue>"` on an `audio` layer, as a timeline `hits` cue, or with the SDK's `hit`, `melody` and
`drumPattern`.

| Cue | Meaning |
| --- | --- |
| `kit:whoosh` | any whoosh; a different one each use, with a touch of pitch and gain |
| `kit:whoosh/airy` | one of the airy whooshes (`airy-01`, `airy-02`) |
| `kit:whoosh/airy-01` | exactly that file, unchanged |
| `kit:pluck/C4`, `kit:bass/F#2` | a pitched voice at a note (flats work: `Bb3`) |
| `kit:boom`, `kit:click`, ... | the original 15 names: pools of fitting variants |
| `kit:fast-pass`, `kit:soft-pass`, ... | curated pools the transitions use |

An unknown cue is a plan error that lists the families, pools and variants.

## Families

whoosh, swish, sweep, riser, downer, swell (air and builds); impact, stinger (hits; stingers are chords in C minor
`cmin-NN` and F major `fmaj-NN`); ui, key, pop (interface); glitch, shimmer; paper, mech (physical); pluck, bell,
marimba, stab, bass (pitched, two octaves); kick, snare, clap, hat, rim, tom (drums). `kit/index.json` lists every
variant with its duration, peak time, description and tags; `kit/sheets/<family>.png` shows spectrograms and
`kit/audition.wav` plays three variants of each family (`kit/audition.json` gives the start of each).

## How variants are chosen

A cue that names a family (or a pool) walks a seeded shuffle of its variants in time order within each composition,
so neighbouring hits differ. The pick also gets a small deterministic pitch (+/- a fraction of a semitone) and gain
(+/- 1 dB) variation; tonal voices get none. `kit:<family>/<name>` and notes are exact. Pitched voices are rendered at
every third semitone; other notes play the nearest root at the rate that tunes them (at most one semitone away).
Transitions choose their variant when the timeline is expanded, so the film file shows the exact file, and they are
placed so that variant's loudest moment lands on the transition's midpoint.

## Regenerating

From this folder (needs numpy, scipy, Pillow):

```
python3 -m sfx.build       # writes kit/<family>/*.wav and kit/index.json, and checks every file
python3 -m sfx.audition    # kit/audition.wav and the spectrogram sheets
```

`build` fails on clipping, DC offset, clicks at a file's edges, silence, or two siblings that are the same sound
(spectral centroid, duration, envelope and band balance). The library is `sfx/` (oscillators, noise colours, swept
biquads, envelopes, modal synthesis for struck objects, Karplus-Strong strings, granular clouds, saturation and
bitcrush, a Schroeder reverb with stereo width, mastering) with one `fam_*.py` per group of families.

`make_kit.py` and `motion_cues.py` and the flat `*.wav` files here are the first, single-variant kit. They are kept for
reference and no longer used.
