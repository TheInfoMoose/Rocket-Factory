## Problem Description
**Audit Finding:** `PERF-04` (Low) — `docs/codebase-audit-report.md`

**Location:** `src/tui/terminal.rs:221-255`.

`run_event_loop` calls `terminal.draw(...)` every 100 ms `event::poll` timeout even when idle, and processes only 1 event per frame draw.

**Fix:** Track a `dirty` flag and drain pending events before redrawing.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
