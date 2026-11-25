## Problem Description
Regression from STEP 19A (src/tui/app/sync.rs:66-122,172-175; src/adapter.rs:805-821). run_sync_job now calls sync_push_project_in_memory (no disk write) and returns SyncOutcome::Error without updates. If feature 1 pushes and feature 2 fails, feature 1's pushed=true/external_id is lost in both memory and disk, so retrying push duplicates external items.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
