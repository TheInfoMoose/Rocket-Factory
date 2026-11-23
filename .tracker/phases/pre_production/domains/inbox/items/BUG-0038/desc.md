## Problem Description
**Audit Finding:** `BUG-15` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/audit.rs:12-16, 72-135`.

`audit_project` checks only `UntrackedComponent`, `UnpushedFeatures`, and `DanglingConnection`. It misses `parent_id` cycles, dangling `parent_id`, duplicate component IDs, self-connections, out-of-bounds/overlapping `GridPos`, `0x0` `GridConfig`, and `Contains`/`parent_id` drift.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
