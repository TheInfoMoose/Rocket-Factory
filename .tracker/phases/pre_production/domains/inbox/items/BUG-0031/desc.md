## Problem Description
**Audit Finding:** `BUG-08` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/tui/canvas.rs:587-667, 430-528`.

Edges overwrite shared corridor cells and highlight styles without junction glyphs; label placement checks `segment_cells` and `visible_cards` but not previously placed labels; and `compute_label_pos` lacks canvas occupancy info, causing `render_canvas` to drop labels whenever `(lx, ly)` hits another edge or card.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
