## Problem Description
**Audit Finding:** `TEST-04` (Low) — `docs/codebase-audit-report.md`

Untested error and variant branches across `cli.rs`, `adapter.rs`, `audit.rs`, `export.rs`, `model.rs`, `theme.rs`, `error.rs`, and `tui/style.rs` (see section 6.1 of `docs/codebase-audit-report.md`).

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
