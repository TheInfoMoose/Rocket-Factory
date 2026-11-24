## Problem Description
In src/tui/terminal.rs:421-428, step_event_loop drains ctx.sync_rx at the start of the step and sets dirty = true, then immediately calls next_event(Duration::from_millis(100)) before returning dirty to run_event_loop, delaying idle sync-outcome redraws by up to 100ms.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
