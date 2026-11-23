## Problem Description
**Audit Finding:** `PERF-02` (Medium) — `docs/codebase-audit-report.md`

**Locations:** `src/tui/ui.rs:75, 205, 229, 272`, `src/tui/canvas.rs:573, 582, 628-665, 690`, `src/tui/app.rs:285-287, 325-346`, `src/model.rs:137-142`.

`visible_component_indices()` is called 5 times per frame with $O(N^2)$ scans, `Project::sub_components()` allocates a `Vec` per row/card, `component_depth()` allocates a `HashSet` per row, and `render_canvas` scans `segment_cells` linearly per label cell.

**Fix:** Cache/compute frame indices and child counts once per frame; use `HashSet<(u16, u16)>` or 2D occupancy grid.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
