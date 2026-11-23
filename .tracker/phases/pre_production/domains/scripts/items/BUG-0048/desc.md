## Problem Description
**Audit Finding:** `SCR-02` (High) — `docs/codebase-audit-report.md`

**Location:** `scripts/phased-step.sh:294-307, 420, 479, 596`.

Substring blocklist (`|| true`, `||true`, `|| :`, `; true`, `set +e`, `2>/dev/null ||`) in `read_amendment()` is bypassable via spacing or alternate zero-exit tails while preserving `base_gate in gate`, causing `bash -c "$EFFECTIVE_GATE"` to exit `0` even when `BASE_GATE` fails. Also `done < <(read_amendment)` suppresses non-zero Python exit codes under `set -euo pipefail`.

## Reproduction Steps
1. Inspect `scripts`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `scripts`
