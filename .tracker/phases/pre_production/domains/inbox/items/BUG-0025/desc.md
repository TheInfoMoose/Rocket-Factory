## Problem Description
**Audit Finding:** `BUG-02` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/tui/app.rs:801-811`.

When the current grid is full, `find_proximity_grid_pos` uses `extra_col = extra_col.saturating_add(1)` inside `loop`, stalling forever at `u16::MAX` if `cols == u16::MAX` or all columns on `anchor.row` up to `u16::MAX` are occupied.

**Fix:** Use `checked_add(1)`, scan fallback rows, or return `Option<GridPos>`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
