## Problem Description
STEP 11A spec gap: MUTATION_SPEC requires all 6 broad archetypes (and legacy ComponentKind mappings via archetype_for) to have `starter_features: &[]`, and POST_GATE runs `cargo test --workspace`, but `tests/e2e.rs` is not listed in `ALLOWED_FILES`. Two integration tests (`e2e_cli_lifecycle_and_mock_adapter_roundtrip` at `tests/e2e.rs:91-97,202` and `e2e_multi_adapter_feature_push_cli_and_tui` at `tests/e2e.rs:433-448,508`) rely on `scaffold_component` populating starter features before calling `.features.truncate(1)` and fail when `starter_features` is empty. Location: `tests/e2e.rs:91`.
Lapse: L2 file outside ALLOWED_FILES
Kind: scope-lapse
Found during: STEP 11A (Neon)

## Reproduction Steps
n/a (spec gap)

## Fix Log & Agent Notes
