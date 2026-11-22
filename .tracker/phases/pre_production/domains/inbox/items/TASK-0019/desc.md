## Problem Description
STEP 4C spec gap: `MUTATION_SPEC` item 3 requires `render_canvas_draws_component_cards_and_connectors` to assert `text.contains('▶')` for a horizontal connection between `view_box` (col 0, row 0) and `core_bus` (col 1, row 0) with label `"events"`, but `src/tui/canvas.rs:378-386` overwrites `'▶'` with the edge label and `src/tui/canvas.rs` is outside `ALLOWED_FILES`. Location: `src/tui/canvas.rs:378-386`.
Lapse: L2 file outside ALLOWED_FILES
Kind: scope-lapse
Found during: STEP 4C (Neon)

## Reproduction Steps
n/a (spec gap)

## Fix Log & Agent Notes
