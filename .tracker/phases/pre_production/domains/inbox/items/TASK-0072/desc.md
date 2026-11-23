## Problem Description
**Audit Finding:** `ARCH-04` (Low) — `docs/codebase-audit-report.md`

**Location:** `src/error.rs:4-46`.

`RfError::NotFound` is reused for missing CLI flags, `RfError::PathEscape` for theme-name syntax errors, and `RfError::JsonSerialize(#[from] serde_json::Error)` mislabels JSON deserialization failures.

**Recommendation:** Add `InvalidArgument(String)` / `Validation(String)` and rename `JsonSerialize` to `Json`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
