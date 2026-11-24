## Problem Description
In src/tui/terminal.rs:321-336, spawn_sync_request clones app.project into project_snapshot and run_sync_job calls sync_pull_project/sync_push_project (src/adapter.rs:686), which writes the entire Component struct to disk via save_component. Foreground edits/moves/deletes/ changes saved while sync_in_flight is true are overwritten on disk (or deleted components recreated), while apply_sync_outcome only merges external_tasks and features in memory.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
