## Problem Description
**Audit Finding:** `BUG-14` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/adapter.rs:258-312`.

`push_unpushed_features` marks `feature.pushed = true` only after all features and adapters succeed; a failure on feature #2 or adapter #2 leaves feature #1 `pushed = false`, causing duplicate external pushes on retry. It also discards `external_id`s from secondary adapters.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
