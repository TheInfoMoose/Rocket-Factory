## Problem Description
**Audit Finding:** `CI-01` (Medium) — `docs/codebase-audit-report.md`

**Location:** `.github/workflows/ci.yml:22, 25, 30, 51, 54, 57`.

Uses mutable tag-pinned actions (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`) instead of immutable 40-char commit SHAs (and `@stable` floats over time).

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
