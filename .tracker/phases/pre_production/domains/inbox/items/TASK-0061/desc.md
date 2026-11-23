## Problem Description
app.rs:1962 expects column 3 without the render that always precedes input (column 2 at 140 wide). app.rs:3281 injects capacity. ui.rs:1091 sets zoom directly, avoiding the Compact default. canvas.rs:904 pins old render tests to Normal, leaving Compact L-bends untested. ui.rs:927 and 1002 inject Help without pressing ? (old).

Class: test defects, new in Phase 12 unless marked old
Location (at 93c8752): src/tui/app.rs:1962, src/tui/app.rs:3281, src/tui/ui.rs:927, src/tui/ui.rs:1002, src/tui/ui.rs:1091, src/tui/canvas.rs:904
Suggested fix: Render before input, use keys for zoom and Help, and add Compact L-bend coverage.
Found during: Phase 12 sample review, 1956932..93c8752 (launch commit inferred). Evidence: code reading only, not run.

## Reproduction Steps
1. Read the listed tests against the draw-then-handle_key loop in src/tui/terminal.rs:221-233.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
