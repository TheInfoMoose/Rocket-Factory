## Problem Description
Regression from STEP 17D (src/adapter.rs:171-181, 285-321). Changing reconcile_status from replacing component.external_tasks to upsert-only prevents deleted or moved external tasks from ever being removed on pull, skewing task_progress and compute_card_status.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
