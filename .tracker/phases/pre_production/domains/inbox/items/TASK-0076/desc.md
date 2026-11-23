## Problem Description
**Audit Finding:** `PERF-03` (Medium) — `docs/codebase-audit-report.md`

**Locations:** `src/tui/app.rs:178, 194, 241`, `src/adapter.rs:323`.

`sync_pull` and `sync_push_selected` deep-clone the entire `Project` (including all `markdown_body` strings) on every sync/poll tick, and `run_pull_status` re-serializes `Project` JSON per adapter. `updated_ids` also deduplicates via linear `Vec::contains`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
