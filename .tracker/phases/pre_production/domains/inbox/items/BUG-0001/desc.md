## Problem Description
`src/tui/canvas.rs:378-386` — In `render_canvas`, `buf.set_string(lx, ly, clipped, edge_style)` writes the edge label starting at `segments[segments.len() / 2]` on the same row as a horizontal edge. For horizontally adjacent cards (`GUTTER_X = 4`), `segments` has 4 cells and `segments[2]` is 1 cell left of the `'▶'` arrowhead at `segments[3]`, so any label of length >= 2 (including `"events"` and all default `edge_kind_tag` values) overwrites `'▶'` — suggested fix: place horizontal edge labels above the edge (`ly.saturating_sub(1)`) or avoid overwriting the arrowhead cell (`segments.last()`).
Kind: bug
Found during: STEP 4C (Neon)

## Reproduction Steps
1. `cargo test --workspace tui::ui::tests::render_canvas_draws_component_cards_and_connectors`
2. Observe: `assertion failed: text.contains('▶')` at `src/tui/ui.rs:391`

## Fix Log & Agent Notes
