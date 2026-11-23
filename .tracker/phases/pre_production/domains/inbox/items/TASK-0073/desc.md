## Problem Description
**Audit Finding:** `ARCH-05` (Low) — `docs/codebase-audit-report.md`

**Locations:** `src/lib.rs:1-12`, `src/tui/mod.rs:1-9`, `src/tui/canvas.rs:19-20, 204-206`, `src/tui/terminal.rs:19-79`, `src/tui/app.rs:55-81`.

Over-exposed `pub` modules and internal items (`TuiTerminal`, `install_panic_hook`, `SyncRequest`, etc.), plus unused public exports (`CELL_STRIDE_X`, `CELL_STRIDE_Y`, `visible_grid_capacity`).

**Recommendation:** Narrow internal items to `pub(crate)` and remove dead exports.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
