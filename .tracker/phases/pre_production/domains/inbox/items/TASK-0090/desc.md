## Problem Description
**Audit Finding:** `PKG-03` (Low) — `docs/codebase-audit-report.md`

**Location:** `Cargo.toml:1-43`.

Missing `[profile.release]` (LTO/strip tuning) and `[lints]` table in `Cargo.toml`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
