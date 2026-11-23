## Problem Description
**Audit Finding:** `SCR-03` (Medium) — `docs/codebase-audit-report.md`

**Location:** `scripts/phased-step.sh:171-174, 258-261`.

`except Exception: continue` in `new-phase` and `find_task()` silently skips malformed `item.toml` files, dropping `TASK_ID` during `commit` or omitting `item` in `.orrery/phase.toml`.

## Reproduction Steps
1. Inspect `scripts`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `scripts`
