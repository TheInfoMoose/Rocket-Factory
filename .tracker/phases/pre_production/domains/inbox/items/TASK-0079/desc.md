## Problem Description
**Audit Finding:** `TEST-02` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/tui/terminal.rs:38-66, 79-104, 191-257`.

Terminal raw-mode setup/teardown, panic hook, `$EDITOR` suspend/resume, startup pull error banner, `KeyEventKind::Press` filtering, and `poll_interval` auto-sync have zero automated tests.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
