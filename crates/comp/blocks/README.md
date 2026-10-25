# Built-in blocks

Blocks are parametrised sub-compositions: `{"type": "precomp", "block": "lower_third", "vars": {"name": "Ada"}}`.
The ones here ship inside the binary (compiled in from this folder) and are the same on every machine.
`turbomotion comp blocks` prints this list with each block's `best_for` and `breaks`.

More blocks come from folders that are named explicitly, lowest priority first: `TURBOMOTION_BLOCK_PATH`
(`:`-separated), the composition's `"block_path": ["../kit/blocks"]` (relative to the composition), and the
composition's own `blocks/`. A later folder replaces an earlier block of the same name. Nothing searches parent
folders.

Pick by **job**, not by browsing files.

| Job | Use when | Blocks |
| --- | --- | --- |
| **teach** | a term, a comparison, a checklist | `definition`, `term_card`, `step_chain`, `compare_two`, `claim_bench`, `inspect_board`, `scene_bug` |
| **card** | a number, a ticket, a stamp | `stat`, `title_card`, `panel`, `holo_slab`, `holo_ticket`, `security_card`, `stamp`, `ridgeline_flip` |
| **stage** | the room, a lower third, motion in space | `stage`, `lower_third`, `callout`, `connector`, `ticker`, `hud_reticle`, `deck_3d`, `card_orbit` |
| **mark** | one glyph or a small drawn icon | `icon` (`kind`: chart / lock / warn / bank / doc / arrow / punch / reaction), `reaction`, `punched_icon`, `icon_chart`, `icon_lock`, `icon_warn`, `icon_bank`, `icon_doc`, `icon_arrow` |
