## Problem Description
At 80 columns the E, v and r/S hints visible at 1956932 are gone. At 112-116 the theme hint is gone (the new footer needs 117 columns). The change does make help and quit fit at 80.

Class: plan-mandated regression, STEP 12B
Location (at 93c8752): src/tui/ui.rs:354-410
Suggested fix: Either accept this in the plan or shorten the labels.
Found during: Phase 12 sample review, 1956932..93c8752 (launch commit inferred). Evidence: code reading only, not run.

## Reproduction Steps
1. Compare the footer at 80 and 112 columns with 1956932.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
