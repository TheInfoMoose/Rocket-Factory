## Problem Description
**Audit Finding:** `META-02` (Low) — `docs/codebase-audit-report.md`

215 workflow/tracker files (`.tracker/`, `.orrery/`, `.phased/`) plus `STEPPED_IMPLEMENTATION_PLAN.md` and `scripts/` are tracked on `master`, encoding local harness assumptions in every `git clone`. Evaluate whether to keep or relocate/document them.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
