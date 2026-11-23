## Problem Description
STEP 15B spec gap: ALLOWED_FILES lists 8 files (`src/error.rs`, `src/lib.rs`, `src/adapter.rs`, `src/cli.rs`, `src/export.rs`, `src/storage.rs`, `src/theme.rs`, `src/tui/app.rs`) and drops `RfError`/`RfResult`, but `src/tui/terminal.rs` also imports and uses `RfError` and `RfResult` on 14 lines (lines 14, 21, 22, 47, 61, 79, 91, 97, 108, 183, 196, 218, 367, 445). Renaming `RfError`/`RfResult` without updating `src/tui/terminal.rs` breaks compilation and fails POST_GATE (`cargo clippy` / `cargo test`). Location: `src/tui/terminal.rs:14`.
Lapse: L2 file outside ALLOWED_FILES
Kind: scope-lapse
Found during: STEP 15B (Neon)

## Reproduction Steps
n/a (spec gap)

## Fix Log & Agent Notes
