## Problem Description
`src/cli.rs` (`execute_command` `Command::Push`, lines 155–219) and `src/tui/app.rs` (`sync_push_selected`, lines 226–274) duplicate the multi-adapter unpushed-feature push loop (~45 lines). Extract a shared helper in `src/adapter.rs` so CLI and TUI share one implementation.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
