## Problem Description
When placement expands the grid, the component is saved before the config. A config failure returns without updating memory and without undoing the component write, so disk and memory differ. Related closed item: BUG-0004.

Class: new-feature defect, STEP 12A
Location (at 93c8752): src/tui/app.rs:570-582, src/tui/app.rs:1012-1032
Suggested fix: Roll back the component write on config failure and add failure-path tests for both operations.
Found during: Phase 12 sample review, 1956932..93c8752 (launch commit inferred). Evidence: code reading only, not run.

## Reproduction Steps
1. Make project config saving fail while component saves succeed.
2. Create or move a card outside the saved grid but inside the visible grid.
3. Reload. The component change is present although the operation reported failure.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
