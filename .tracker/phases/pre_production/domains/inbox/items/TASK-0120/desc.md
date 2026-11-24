## Problem Description
STEP 18C (STEPPED_IMPLEMENTATION_PLAN.md:4622) listed delete_component_rolls_back_detached_children_on_failure in Tests:, but it was omitted from [POST_GATE] and is missing from src/tui/app/mod.rs.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
