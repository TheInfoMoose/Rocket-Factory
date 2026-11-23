## Problem Description
At 120 columns the list went from 29 to 22 columns (inner 27 to 20). Nested labels that fit at 1956932 now lose their (complete/total) suffix. For example, depth-2 worker_pool needs 26 columns.

Class: regression, STEP 12A
Location (at 93c8752): src/tui/ui.rs:18-32, src/tui/ui.rs:173-176
Suggested fix: Truncate the name ahead of the count, or widen the list. Assert inside the list rect.
Found during: Phase 12 sample review, 1956932..93c8752 (launch commit inferred). Evidence: code reading only, not run.

## Reproduction Steps
1. Open a project with depth-2 component worker_pool at 120x35.
2. Expand both ancestors. The name fits, but (0/0) is clipped.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
