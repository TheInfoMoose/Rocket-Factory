## Problem Description
**Audit Finding:** `BUG-19` (Low) — `docs/codebase-audit-report.md`

**Location:** `src/tui/app.rs:847-866, 1396-1405`.

`resolve_visible_collisions_for` writes moved cards before `save_project_config`; `DeleteComponent` deletes the parent before saving detached children. Failures leave disk partially updated.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
