## Problem Description
**Audit Finding:** `BUG-06` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/tui/app.rs:651-666, 1252-1261, 1425-1456`.

`confirm_connect` matches `conn.from == source_id && conn.to == target_id` without checking `conn.kind`, deleting an existing non-`DependsOn` edge instead of adding `DependsOn`. Also neither `confirm_connect` nor `DeleteConnection` clears `to.parent_id` when deleting a `Contains` edge.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
