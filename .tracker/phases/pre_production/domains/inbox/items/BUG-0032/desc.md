## Problem Description
**Audit Finding:** `BUG-09` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/tui/terminal.rs:61-66, 87-89, 207-210`.

`run_tui` returns `loop_result.and(restore_result)`, discarding terminal-restore errors when `run_event_loop` fails; `restore_terminal` aborts on `disable_raw_mode()?` without leaving alternate screen or showing cursor; `suspend_for_editor` enables raw mode without rolling back if `EnterAlternateScreen` fails.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
