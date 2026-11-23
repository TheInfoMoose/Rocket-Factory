## Problem Description
ensure_visible only scrolls far enough to show the selection. It never pulls scroll back when zooming out increases capacity.

Class: new-feature defect, STEP 12A
Location (at 93c8752): src/tui/canvas.rs:179-193
Suggested fix: Clamp scroll against occupied extent and new capacity after zoom. Test card visibility after the zoom keys.
Found during: Phase 12 sample review, 1956932..93c8752 (launch commit inferred). Evidence: code reading only, not run.

## Reproduction Steps
1. At 140x40 place cards in columns 0-3 of one row and select column 3.
2. Press + + - -. scroll_col stays 2, so columns 0-1 are hidden although all four fit.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
