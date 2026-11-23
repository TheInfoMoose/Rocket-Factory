## Problem Description
Rebrand the application from **Rocket Factory** to **Rocket Bench** so it can be folded in as a component of the pending **Rocket Factory** development suite of apps.

Scope to address in `project-plan` / `phase-plan`:
- Crate/package name (`Cargo.toml` `rocket-factory` -> `rocket-bench`)
- Binary name (`rf` -> `rb` or `rocket-bench` — confirm with user)
- Project workspace directory (`.rocket/` -> `.rocket-bench/` or `.bench/` or keep `.rocket/` — confirm with user)
- User config directory (`~/.config/rocket-factory/` -> `~/.config/rocket-bench/`)
- Env var (`RF_POLL_INTERVAL_SECS` -> `RB_POLL_INTERVAL_SECS` or similar — confirm with user)
- Error/type names (`RfError`, `RfResult` if desired, or keep internal)
- TUI title bar, CLI `--help`/about text, `README.md`, `LICENSE` copyright/attribution, CI/test references.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
