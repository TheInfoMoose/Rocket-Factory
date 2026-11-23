## Problem Description
**Audit Finding:** `CI-05` (Low) — `docs/codebase-audit-report.md`

**Location:** `.gitignore:1-5`.

Omits `.agents/` (observed live as untracked `?? .agents/teamwork/`), `.DS_Store`, `*.swp`, and `*~`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
