## Problem Description
**Audit Finding:** `SCR-06` (Low) — `docs/codebase-audit-report.md`

**Location:** `scripts/phased-step.sh:133, 216, 218, 229, 246, 269, 361, 422, 438, 468, 485, 489`.

Uses GNU `find -printf`, GNU `sha256sum`, Bash 4+ `mapfile -t`, Python 3.11+ `tomllib`; `:216` breaks on backticked `ALLOWED_FILES`, `:218` matches `Cargo.lock` substring in comments, and `:438` `|| true` ignores `git diff --no-index` I/O errors (`exit > 1`).

## Reproduction Steps
1. Inspect `scripts`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `scripts`
