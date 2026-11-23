## Problem Description
**Audit Finding:** `CI-04` (Low) — `docs/codebase-audit-report.md`

**Location:** `.github/workflows/ci.yml:36, 39`.

`cargo clippy` and `cargo test` in `check` omit `--locked`, unlike `package`, `build`, and `install`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
