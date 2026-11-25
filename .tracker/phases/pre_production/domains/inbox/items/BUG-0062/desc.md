## Problem Description
Test defect from STEP 19C/19D/19E (tests/e2e.rs:56-89; src/cli.rs:353-387; src/tui/terminal.rs:1045,1205). On Windows, cmd /C args containing quotes get escaped as \" by std::process::Command, breaking JSON output and findstr, and more is interactive. New adapter unit tests in src/cli.rs and src/tui/terminal.rs also hardcode sh -c without cfg(unix) or cross-platform helpers. Will fail Phase 20 Windows CI matrix (TASK-0085).

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
