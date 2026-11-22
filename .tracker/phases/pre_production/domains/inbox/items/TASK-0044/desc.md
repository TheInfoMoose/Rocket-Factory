## Problem Description
STEP 8B spec gap: ALLOWED_FILES is limited to `src/tui/modal.rs`, while MUTATION_SPEC adds `ModalState::ConfirmDelete(ConfirmDeleteForm)` and `ModalAction::DeleteComponent` / `ModalAction::DeleteConnection`. However, `src/tui/app.rs:546` exhaustively matches `ModalAction`, and `src/tui/ui.rs:309` and `src/tui/ui.rs:473` exhaustively match `ModalState` with no wildcard arms. Adding the new variants in `src/tui/modal.rs` alone cannot pass `cargo clippy --workspace --all-targets -- -D warnings` or `cargo test --workspace` (E0004 non-exhaustive patterns) without editing `src/tui/app.rs` and `src/tui/ui.rs` (or merging STEP 8B with STEP 8C). Also note minor anchor mismatch: MUTATION_SPEC refers to `handle_modal_key`, whereas `src/tui/modal.rs:234` defines `ModalState::handle_key`. Location: `src/tui/app.rs:546`, `src/tui/ui.rs:309`, `src/tui/ui.rs:473`.
Lapse: L2 file outside ALLOWED_FILES
Kind: scope-lapse
Found during: STEP 8B (Neon)

## Reproduction Steps
n/a (spec gap)

## Fix Log & Agent Notes
