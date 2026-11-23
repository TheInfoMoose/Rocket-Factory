## Problem Description
Help widened from 64 to 76 columns. Lines wrap below 80 columns, but popup height counts unwrapped lines, so bottom rows are clipped. It fit from 68 columns at 1956932. Related closed item: BUG-0007 (80-column case).

Class: regression, STEP 12B
Location (at 93c8752): src/tui/ui.rs:549, src/tui/ui.rs:698-713
Suggested fix: Use one column or shorter rows when narrow, and size height from wrapped rows. Test at 68 and 79 columns through the ? key.
Found during: Phase 12 sample review, 1956932..93c8752 (launch commit inferred). Evidence: code reading only, not run.

## Reproduction Steps
1. Open the TUI at 79x24.
2. Press ?. The Close hint is cut off; narrower widths lose more rows.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
