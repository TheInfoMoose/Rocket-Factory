## Problem Description
**Audit Finding:** `SCR-01` (High) — `docs/codebase-audit-report.md`

**Location:** `scripts/phased-step.sh:430`.

Pre-existing dirty check runs `if grep -qF "  ${p}" "${SNAP}/tree"; then`. Without end-of-line anchoring, dirty `src/foo.rs_bak` or `src/foo.rs/sub` falsely matches `src/foo.rs`, classifying a clean-before-start `ALLOWED_FILES` path as `MANUAL`, leaving edits unrestored, and exiting `1`.

## Reproduction Steps
1. Inspect `scripts`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `scripts`
