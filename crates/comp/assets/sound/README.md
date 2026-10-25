# Sound kit

Five named cues, each with a distinct timbre. Use as `"src": "kit:card-in"` on an `audio` layer.

| Cue | Timbre | When |
| --- | --- | --- |
| `card-in` | soft whoosh and a muted wood tick | a holo slab or card arrives |
| `stamp` | heavy low impact with grit | the Wrong Trade banner hits |
| `counter` | bright metallic ping | a big number lands or ticks |
| `loss` | detuned low brass, beating | a loss is revealed |
| `reveal` | rising shimmer | a definition or masked line opens |

Regenerate with `python3 make_kit.py` in this folder. No samples, no licences.

## Motion cues

Transition and UI sounds from [motion_cues.py](motion_cues.py). A timeline transition picks one by default (see
`comp catalogue` → `timeline.transitions`); `"hits"` places any of them on an exact moment.

| Cue | Timbre | Default for |
| --- | --- | --- |
| `whoosh` | air past the lens | push, slide over, wipe, blur dissolve |
| `swish` | a fast whip | whip pan |
| `sweep` | a long pass | pan out, zoom back |
| `riser` | a build into a hit | zoom through, dip to white |
| `pop` | a soft bubble | flood |
| `click`, `tick`, `type` | UI and clock | cursor clicks, counters, typing |
| `glitch` | crushed digital bursts | glitch |
| `boom` | a sub drop | big reveals |
