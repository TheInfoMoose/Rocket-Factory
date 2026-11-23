## Problem Description
The narrow-canvas fallback requires relative column and row to both be 0, while visible capacity can report two rows. A selected card on row 1 is then neither scrolled to nor drawn.

Class: new-feature defect, STEP 12A
Location (at 93c8752): src/tui/canvas.rs:142-154
Suggested fix: Clamp width and height independently for the first visible column or row.
Found during: Phase 12 sample review, 1956932..93c8752 (launch commit inferred). Evidence: code reading only, not run.

## Reproduction Steps
1. At 60x24 select a card at column 0, row 1.
2. Zoom to Expanded with the viewport at 0,0. The selected card is not drawn.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
