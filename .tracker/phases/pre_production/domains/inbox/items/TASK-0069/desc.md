## Problem Description
**Audit Finding:** `ARCH-01` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/tui/app.rs` (`3,517` lines), `src/storage.rs:70-105`, `src/theme.rs:105-156`, `src/tui/terminal.rs:134-154`.

`src/tui/app.rs` concentrates UI state, adapter sync, tree visibility, grid placement/collision, modal persistence/rollback, and key routing. Grid expansion checks are duplicated across 4 sites despite `ensure_grid_contains`, and path-containment logic is duplicated across 4 locations.

**Recommendation:** Split `src/tui/app.rs` into focused submodules (`state`, `grid`, `actions`, `sync`), reuse `ensure_grid_contains`, and expose a single hardened `ensure_contained` helper from `src/storage.rs`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
