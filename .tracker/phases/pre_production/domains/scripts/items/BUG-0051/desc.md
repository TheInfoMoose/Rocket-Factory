## Problem Description
**Audit Finding:** `SCR-05` (Low) — `docs/codebase-audit-report.md`

**Location:** `scripts/phased-step.sh:193`.

`out.write_text(...)` overwrites `.orrery/phase.toml` in place instead of staging a sibling temp file and using `os.replace()`.

## Reproduction Steps
1. Inspect `scripts`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `scripts`
