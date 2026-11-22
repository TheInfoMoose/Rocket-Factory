## Problem Description
STEP 4E spec gap: MUTATION_SPEC item 1 places horizontal edge labels at `label_y = ly.saturating_sub(1)` (`y = 3`) and `label_x = lx.saturating_sub(1).max(inner.x)` (`x = 25`) and skips cells inside any visible card `Rect` (cards also render after edges and overwrite `x = 28..50, y = 2..7`), while MUTATION_SPEC item 2 tests adjacent cards at `(0, 0)` and `(1, 0)` (`GUTTER_X = 4`, leaving only `x = 25..28` = 3 cells at `y = 3`) with `label: Some("events".into())` (6 chars) and asserts `text.contains("events")`. Only `"eve"` fits in the gutter, so `assert!(text.contains("events"))` cannot pass without violating item 1 or weakening item 2. Location: `src/tui/canvas.rs:378-386`.
Lapse: L4 other
Kind: scope-lapse
Found during: STEP 4E (Neon)

## Reproduction Steps
n/a (spec gap)

## Fix Log & Agent Notes
