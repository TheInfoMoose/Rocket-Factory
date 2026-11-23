## Problem Description
**Audit Finding:** `SCR-07` (Low) — `docs/codebase-audit-report.md`

**Location:** `scripts/neon-dispatch:24-25, 43-44`.

`${2:?...}` rejects empty `--note ""` with bash parameter-expansion exit `1` instead of usage exit `2`; `${NEON_FLAGS:---dangerously-skip-permissions}` overrides intentional empty `NEON_FLAGS=""` and splits quoted spaces via `read -r -a`; 1s UTC log timestamp can collide on sub-second retries.

## Reproduction Steps
1. Inspect `scripts`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `scripts`
