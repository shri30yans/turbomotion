# Briefs: measuring the agent loop

These are written requests for an agent working with `turbomotion comp`. A run is scored on the same machinery it
uses: how many `check` iterations until it passes, whether the assertions in `comp.assert.json` (written by the
reviewer, not the agent) pass, and a checklist read from the final `sheet`. If an agent needs more than three
iterations for a first draft to be valid, the catalogue, the error messages or the defaults are wrong, not the agent.

| Brief | Tests |
| --- | --- |
| `01-line-reveal.md` | masks, text animators, beat times |
| `02-photo-parallax.md` | the image pipeline and `treat`-style finishing |
| `03-new-effect.md` | writing a shader plugin from a description |
| `04-lower-third-block.md` | reuse of blocks instead of rebuilding |

Record per run: iterations to a passing `check`, iterations to a passing assertion set, the first error message the
agent hit, and whether it found the answer in the catalogue or had to read source.
