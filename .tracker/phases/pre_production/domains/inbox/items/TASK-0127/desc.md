## Problem Description
STEP 19C spec gap: MUTATION_SPEC item 1a requires verifying `rktb --version` exits 0 with expected CLI name/usage on stdout, but `src/cli.rs:10-14` (`#[command(name = "rktb", about = ...)]`) does not declare `version`, so `rktb --version` exits 2 with `error: unexpected argument '--version' found`. Modifying `src/cli.rs` is outside `ALLOWED_FILES` (`FORBIDDEN_SCOPE` forbids modifying `src/`). Location: `src/cli.rs:10-14`.
Lapse: L2 file outside ALLOWED_FILES
Kind: scope-lapse
Found during: STEP 19C (Neon)

## Reproduction Steps
n/a (spec gap)

## Fix Log & Agent Notes
