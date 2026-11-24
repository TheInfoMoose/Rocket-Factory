## Problem Description
STEP 18D spec gap: `src/tui/app/actions.rs` previously selected the last outgoing edge via `rfind` on `E`, whereas STEP 18D specifies `ModalState::edit_connections(outgoing)` initialized to index `0` and requires `multi_edge_edit_and_delete_select_non_first_edge` (where pressing `E` then `]` selects the second outgoing edge `alpha -> gamma`). However, `tests/e2e.rs:1017-1051` (`e2e_edge_contains_ingestion_mutable_archetype_and_connect_semantics`) creates a sub-component `nav_header` (adding `frontend -> nav_header` `Contains` at index 0 in STEP 18C), then connects `frontend -> widget_lib` at index 1, and presses `E` without `]` expecting `frontend -> widget_lib` to be selected in both step 4 (`tests/e2e.rs:1017`) and step 5 (`tests/e2e.rs:1037`, where both edges are `Contains`). `tests/e2e.rs` is outside `ALLOWED_FILES`. Location: `tests/e2e.rs:1017-1042`.
Lapse: L2 file outside ALLOWED_FILES
Kind: scope-lapse
Found during: STEP 18D (Neon)

## Reproduction Steps
n/a (spec gap)

## Fix Log & Agent Notes
