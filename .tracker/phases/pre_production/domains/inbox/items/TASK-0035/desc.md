## Problem Description
In both `src/tui/app.rs` (`sync_push_selected`) and `src/cli.rs` (`Command::Push`), once the first active adapter pushes a feature and sets `feature.pushed = true`, subsequent active adapters skip that feature. Support pushing unpushed features across all configured active adapters cleanly (recording pushed state after the adapter loop or per-adapter tracking).

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
