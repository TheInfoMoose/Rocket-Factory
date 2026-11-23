## Problem Description
**Audit Finding:** `BUG-18` (Low) — `docs/codebase-audit-report.md`

**Location:** `src/tui/app.rs:1473, 1485-1672`.

`E` edits only the first outgoing connection and `D` deletes only the first outgoing or fallback incoming connection. Mode guards across action/modal keys (`a`, `q`, `r`/`S`/`p`, `e`/`E`/`d`/`D`/`c`/`v`) are inconsistent when in `MoveCard`/`Connect`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
