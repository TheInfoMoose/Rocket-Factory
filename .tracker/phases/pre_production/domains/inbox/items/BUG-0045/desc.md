## Problem Description
**Audit Finding:** `BUG-22` (Low) — `docs/codebase-audit-report.md`

**Location:** `src/adapter.rs:185-189, 221-230`.

`run_push_component` validates `component_id` for `TomlManifest` and `Json` but skips existence validation for `MarkdownRoadmap`. `run_command_with_stdin` decodes `stdout`/`stderr` with `from_utf8_lossy`, masking invalid UTF-8 bytes as `U+FFFD`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
