## Problem Description
Pre-existing bug observed in src/tui/app/sync.rs:142-143. apply_sync_outcome replaces comp.features from the stale worker snapshot, discarding foreground feature additions/edits made while sync was in flight.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
