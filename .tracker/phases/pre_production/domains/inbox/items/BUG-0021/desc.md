## Problem Description
**Audit Finding:** `SEC-04` (Low) — `docs/codebase-audit-report.md`

**Location:** `src/tui/terminal.rs:68-85`.

`parse_editor_command` uses `raw.split_ascii_whitespace()`, breaking quoted `$EDITOR` paths or arguments containing spaces.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
