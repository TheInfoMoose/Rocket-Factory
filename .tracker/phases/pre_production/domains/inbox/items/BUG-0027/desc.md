## Problem Description
**Audit Finding:** `BUG-04` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/tui/app.rs:1088-1212`.

Adding a feature in `UpdateComponent` transitions a component from `Gap` to `Planned`, removing it from `visible_component_indices()` under `ListFilter::GapsOnly`. Because `clamp_selection()` is omitted, editing the last visible item leaves `selected_index` out of bounds.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
