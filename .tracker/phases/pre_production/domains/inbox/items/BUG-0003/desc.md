## Problem Description
App::viewport (src/tui/app.rs:58) is initialized to (0, 0) and never mutated; ensure_visible (src/tui/canvas.rs:72-86) is never called in production code. On a standard 120x35 terminal, only 2 card columns fit in the canvas, leaving cards at col >= 2 or offscreen rows permanently culled and unreachable.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
