## Problem Description
New defect from STEP 19B (src/tui/modal.rs:597-616; src/tui/ui.rs:1053-1055). When active_field == 1 (Label), '[' and ']' insert characters rather than cycling edges, but the footer hint still displays [[/]] Edge and omits PageUp/PageDown.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
