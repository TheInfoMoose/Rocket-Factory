## Problem Description
**Audit Finding:** `BUG-13` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/adapter.rs:117-144`.

`reconcile_status` assigns `component.external_tasks = entry.tasks.clone()`, clobbering tasks pulled from earlier active adapters (and duplicate `component_id` entries inside one response overwrite each other while inflating `components_updated`). Auto-linking also relies on unattributed `task.title == feature.title`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
