## Problem Description
In src/tui/app.rs, move_selected_card (lines 304-312), confirm_connect (lines 360-383), UpdateComponent (lines 465-506), and UpdateConnection (lines 508-545) mutate self.project in memory before calling save_component / save_project_config without rolling back on Err (unlike CreateComponent). Failed disk writes leave UI state desynced from disk.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
