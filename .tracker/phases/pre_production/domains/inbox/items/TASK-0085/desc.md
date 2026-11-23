## Problem Description
**Audit Finding:** `CI-03` (Medium) — `docs/codebase-audit-report.md`

**Location:** `.github/workflows/ci.yml:17, 46`.

Runs solely on `ubuntu-latest` (macOS/Windows unverified) and has no `v*` tag-triggered release artifact/checksum workflow.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
