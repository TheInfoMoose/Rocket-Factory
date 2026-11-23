## Problem Description
**Audit Finding:** `TEST-03` (Medium) — `docs/codebase-audit-report.md`

**Location:** `src/storage.rs:25-284`.

Only 3 unit tests exist in `storage.rs`; `ensure_contained` has zero unit tests in `storage.rs`, and error paths (`AlreadyExists`, `NotFound`, `TomlParse`, missing `components/`/`.md`, symlink escapes) are untested.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
