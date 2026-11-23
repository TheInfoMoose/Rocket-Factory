## Problem Description
**Audit Finding:** `BUG-17` (Low) — `docs/codebase-audit-report.md`

**Locations:** `src/tui/app.rs:619-620, 212, 267, 362, 1309`, `src/tui/ui.rs:416-421`.

`detail_scroll` increments up to `u16::MAX` without clamping to wrapped detail content height, and is not reset to `0` when `clamp_selection()` changes selection on collapse or sync.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
