## Problem Description
Minor Phase 5 findings: (1) compute_label_pos in src/tui/canvas.rs:305-346 places L-bend corner midpoint labels at (mx+1, my), potentially overwriting horizontal segment cells; (2) first_unoccupied_grid_pos in src/tui/app.rs:391-406 falls back to (0,0) when grid is full; (3) render_modal in src/tui/ui.rs:389 u16 multiply (area.width * 60) overflows if width > 1092; (4) 'v' key in src/tui/app.rs:641-650 does not clear connect_source_id/mode; (5) footer in src/tui/ui.rs:258-264 omits [v] editor hint; (6) suspend_for_editor in src/tui/terminal.rs:71-76 fails if $EDITOR includes arguments (e.g. 'code --wait').

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
