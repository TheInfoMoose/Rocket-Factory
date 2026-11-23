## Problem Description
**Audit Finding:** `SEC-01` (High) — `docs/codebase-audit-report.md`

**Locations:** `src/cli.rs:67-74`, `src/main.rs:7-9`, `src/tui/terminal.rs:199-203, 244-254`, `src/adapter.rs:151-157`.

Whenever `--adapters-file` is omitted, `default_adapters_path()` falls back to CWD-relative `.rocket/adapters.toml` if `HOME` is unset or empty. On TUI launch, `run_tui` immediately runs `app.sync_pull(adapters_path)` without confirmation whenever `active_adapters` is non-empty (and repeats it on `poll_interval`).

**Impact:** Launching `rf` (or `rf pull`/`push`) in an untrusted repository in any `HOME`-less environment executes arbitrary commands from `.rocket/adapters.toml` automatically.

**Recommendation:** Remove the CWD `.rocket/adapters.toml` fallback from `default_adapters_path()` and require explicit trust/confirmation (`--adapters-file` or interactive prompt) before running workspace-local or startup adapter commands.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
