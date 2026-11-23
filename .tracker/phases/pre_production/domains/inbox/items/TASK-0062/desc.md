## Problem Description
select_component_by_id sets selected_index instead of navigating. The zoom test checks titles after zooming, with no scrolled selection or card checks. The grid-expansion test never expands the grid or asserts its size; the STEP 12D spec omitted that assertion as well.

Class: test defects, STEP 12D
Location (at 93c8752): tests/e2e.rs:726, tests/e2e.rs:1072-1100, tests/e2e.rs:1121-1161
Suggested fix: Navigate with keys, assert cards after zooming from a scrolled selection, and expand then reload the grid.
Found during: Phase 12 sample review, 1956932..93c8752 (launch commit inferred). Evidence: code reading only, not run.

## Reproduction Steps
1. Read the listed tests against the STEP 12D test names and contract.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
