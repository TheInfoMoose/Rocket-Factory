## Problem Description
**Audit Finding:** `ARCH-02` (Medium) — `docs/codebase-audit-report.md`

**Locations:** `src/cli.rs:133-218`, `src/tui/app.rs:167-275`, `src/adapter.rs`.

`cli::execute_command` and `App::sync_pull`/`App::sync_push_selected` re-implement adapter sync loops with divergent partial-write and empty-adapter semantics, and both discard `PushResponse` (`external_id`, `message`).

**Recommendation:** Consolidate sync orchestration into `src/adapter.rs` with staged persistence and surface `PushResponse` metadata.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
