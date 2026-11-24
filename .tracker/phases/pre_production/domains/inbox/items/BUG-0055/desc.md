## Problem Description
New defect from STEP 17C (src/audit.rs:193-205 vs src/tui/app/actions.rs:191-266, 331-385). audit_project now flags parent_id without a matching EdgeKind::Contains connection as HierarchyDrift, but TUI sub-component creation (a / e -> Add Sub-Component) only sets parent_id and never adds a Contains edge. (Related: BUG-0029 covers DeleteConnection/confirm_connect leaving parent_id.)

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
