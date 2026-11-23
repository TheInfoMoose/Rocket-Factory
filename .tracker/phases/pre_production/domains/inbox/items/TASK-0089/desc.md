## Problem Description
**Audit Finding:** `PKG-02` (Medium) — `docs/codebase-audit-report.md`

**Location:** `Cargo.toml:11-21`.

`package.exclude` omits `"docs/**"`, so `cargo package` bundles `docs/codebase-audit-report.md` into published `.crate` tarballs.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
