## Problem Description
**Audit Finding:** `SCR-04` (Medium) — `docs/codebase-audit-report.md`

**Location:** `scripts/phased-step.sh:556`.

`git log -n1 --format=%h -F --grep="STEP ${STEP_ID}: " HEAD` searches both subject and commit body, so an earlier commit mentioning `"STEP <ID>: "` in its body falsely marks an uncommitted step `committed-earlier`.

## Reproduction Steps
1. Inspect `scripts`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `scripts`
