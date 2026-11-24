## Problem Description
New defect from STEP 17C (src/audit.rs:207-245). Sub-components occupy the same global canvas grid as top-level components in src/tui/app/grid.rs, but audit_project skips grid bounds for components with parent_id (else-if branch) and keys overlap detection by (parent_id, position) instead of global position.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
