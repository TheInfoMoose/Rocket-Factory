## Problem Description
Sub-component creation that expands the grid saves the component before the config and has no rollback. A failed config write leaves an orphan file that is absent from memory.

Class: old bug, present at 1956932
Location (at 93c8752): src/tui/app.rs:969-981, src/tui/app.rs:1012-1032
Suggested fix: Roll back the component write on config failure, sharing the fix with top-level create.
Found during: Phase 12 sample review, 1956932..93c8752 (launch commit inferred). Evidence: code reading only, not run.

## Reproduction Steps
1. Fill the saved grid and make project config saving fail.
2. Press a, submit a sub-component, then cancel after the error.
3. Reload. The sub-component exists.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
