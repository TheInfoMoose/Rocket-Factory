## Problem Description
**Audit Finding:** `PERF-01` (Medium) — `docs/codebase-audit-report.md`

**Locations:** `src/tui/terminal.rs:234-254`, `src/tui/app.rs:167-275`.

`process_pending_sync` and `poll_interval` auto-sync run `sync_pull`/`sync_push_selected` synchronously on the UI loop, freezing rendering and keyboard input (including `Ctrl+C`) until external commands exit.

**Fix:** Run syncs on a background thread via `mpsc` channel.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
