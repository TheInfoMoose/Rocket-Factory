## Problem Description
**Audit Finding:** `CI-02` (Medium) — `docs/codebase-audit-report.md`

**Location:** `.github/workflows/ci.yml:15-43`.

No `cargo audit` / `cargo deny` (nor `schedule:` trigger), no `cargo doc --no-deps`, and no `bash -n` / `shellcheck` on `scripts/*`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
