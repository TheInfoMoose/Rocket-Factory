## Problem Description
The target is min(position + 1, grid max). For a card already beyond the effective grid, that is smaller than its current position.

Class: old bug, present at 1956932
Location (at 93c8752): src/tui/app.rs:523, src/tui/app.rs:532
Suggested fix: Never let a forward move produce a smaller coordinate.
Found during: Phase 12 sample review, 1956932..93c8752 (launch commit inferred). Evidence: code reading only, not run.

## Reproduction Steps
1. Load a card at column 20 with an effective grid 8 columns wide.
2. Press m, then Right. The card jumps to column 7 if that cell is free.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
