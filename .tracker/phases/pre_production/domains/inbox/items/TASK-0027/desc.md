## Problem Description
STEP 5C added 31 lines of editor orchestration in src/tui/terminal.rs:125-153 (auto-save missing .md, suspend_for_editor, reload_component_markdown, error handling) with 0 unit tests in terminal.rs. Extract a testable helper so the full editor flow and error paths are unit-tested.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
