## Problem Description
**Audit Finding:** `BUG-05` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/tui/app.rs:325-346`.

`visible_component_indices` initiates DFS only from components whose `parent_id` is `None` or missing from `existing_ids`. A cycle (`A -> B -> A`) hides all members from list and canvas while `gap_count()` still counts them.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
