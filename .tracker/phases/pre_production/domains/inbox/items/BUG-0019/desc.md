## Problem Description
**Audit Finding:** `SEC-02` (High) — `docs/codebase-audit-report.md`

**Location:** `src/adapter.rs:146-190`.

`run_command_with_stdin` writes `stdin_payload` synchronously via `stdin.write_all(...)` before `child.wait_with_output()` starts draining `stdout`/`stderr`, with no execution timeout and no output byte limit.

**Impact:** If `stdin_payload` and child `stdout`/`stderr` exceed OS pipe buffers (~64 KiB) concurrently, parent and child deadlock permanently. A stalled or verbose adapter hangs CLI/TUI or exhausts memory.

**Recommendation:** Stream `stdin` in a scoped thread while draining `stdout`/`stderr` concurrently with bounded buffers and a configurable timeout (killing the child on expiry).

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
