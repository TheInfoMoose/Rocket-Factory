# Phase 18 review

Range: `07a6052515fe3eb72ce9d9686024d21824560cb7..6fc49e46c1673f8b523ecac7effff8788c8d7b4f`

## Independent pass

1. `src/tui/terminal.rs:321-336`, `src/adapter.rs:686` (`new defect`)
   - Trigger: Start an async pull or push (`r`, `S`, `p`, or auto-poll) and while `sync_in_flight` is true, edit/move/delete a component on disk (`e`, `m`, `d`, or `v` in `$EDITOR`).
   - Effect: `spawn_sync_request` clones `app.project` into `project_snapshot` and `run_sync_job` runs `sync_pull_project`/`sync_push_project`, which calls `save_component(root, &project.components[idx])` with the entire stale `Component` from `project_snapshot`. Foreground edits saved to disk while sync is in flight are overwritten on disk (or a deleted component is recreated on disk), while `apply_sync_outcome` only merges `external_tasks` and `features` in memory.

2. `src/tui/modal.rs:597-606` (`new defect`)
   - Trigger: Open `EditConnection` (`E`) on a component with more than one outgoing edge (`form.candidates.len() > 1`), `Tab` to the `Label` field (`active_field == 1`), and type `[` or `]`.
   - Effect: `handle_edit_connection_key` matches `KeyCode::Char('[')` and `KeyCode::Char(']')` without `&& form.active_field == 0` (unlike `Left`/`Right`), so it cycles to the previous/next edge candidate and discards unsaved edits instead of typing `[` or `]` into `form.label`.

3. `src/tui/terminal.rs:421-428` (`new defect`, low)
   - Trigger: A background sync thread sends `SyncOutcome` on `sync_rx` while the terminal is idle.
   - Effect: `step_event_loop` drains `sync_rx` at the top of the step and sets `dirty = true`, then still blocks on `next_event(Duration::from_millis(100))` before returning `dirty` to `run_event_loop`, delaying the status/task redraw by up to 100 ms.

What was not checked:
- Live interactive terminal rendering in a real PTY (only `TestBackend` unit/e2e tests and `cargo test --workspace` were run: 168 unit + 13 e2e passed).
- `cargo clippy` was not run during the blind pass.

<!-- SEALED: Independent pass locked -->

## Contract pass

- **Unmet `Contract & Behavior` / `Tests`:**
  - `STEP 18C`: `delete_component_rolls_back_detached_children_on_failure` test is missing from `src/tui/app/mod.rs` (omitted from `18C` `POST_GATE` grep; tracked as `TASK-0120` `pending`).
  - `STEP 18F`: Background sync preserves in-memory UI edits in `apply_sync_outcome`, but `run_sync_job` calls `sync_pull_project`/`sync_push_project` on a stale `project_snapshot`, overwriting concurrent foreground disk edits (Finding 1 / `BUG-0057`).
- **Deviations & deferrals:**
  - `18D` was deferred once for scope review (`tests/e2e.rs` added to `ALLOWED_FILES` on restage), cascading `18E`/`18F` dependency deferrals; all 6 steps (`18A`–`18F`) subsequently completed (`TASK-0113`–`TASK-0118` closed). No unrequested changes or open amendments.
  - `STEP 18D` test `multi_edge_edit_and_delete_select_non_first_edge` did not type `[`/`]` in `Label` (Finding 2 / `BUG-0058`); `STEP 18F` test `async_sync_channel_applies_pull_outcome_without_blocking` only mutated memory, not disk (Finding 1 / `BUG-0057`).
- **Independent finding origins:**
  - Finding 1 (`BUG-0057`): `plan` + `coder` (`STEP 18F` forbade `src/adapter.rs` changes while moving full-component-saving `sync_pull_project` to a background thread).
  - Finding 2 (`BUG-0058`): `coder` (`STEP 18D` omitted `active_field == 0` guard on `[`/`]` in `handle_edit_connection_key`).
  - Finding 3 (`BUG-0059`): `coder` (`STEP 18F` `step_event_loop` waits 100 ms on `next_event` after draining `sync_rx`).
- **Duplicate check (`neb list --json`):** All 3 independent findings are already filed as `pending` (`BUG-0057`, `BUG-0058`, `BUG-0059`); `TASK-0119` (`tests/e2e.rs` multi-edge `E` update) is already satisfied in `tests/e2e.rs`.

## Handoff

- **Verdict:** Pass with follow-ups (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace` 168 unit + 13 e2e pass).
- **High:** `BUG-0057` (background sync overwrites concurrent foreground disk edits from stale `project_snapshot`).
- **Medium:** `BUG-0058` (`[`/`]` in `EditConnectionForm` intercepts `Label` text input when `candidates.len() > 1`).
- **Low:** `BUG-0059` (`step_event_loop` blocks up to 100 ms after `sync_rx` drain), `TASK-0120` (add missing `delete_component_rolls_back_detached_children_on_failure` test).
- **Ready to close on approval:** `BUG-0026`–`BUG-0032`, `BUG-0040`–`BUG-0043`, `BUG-0054`–`BUG-0056`, `TASK-0074`, `TASK-0075`, `TASK-0077`, `TASK-0119`.
- **Decisions:** None.


## Contract pass

- **Origins for Independent pass findings:**
  - Finding 1 (`BUG-0057`): `plan` — `STEP 18F` specified `apply_sync_outcome` merging only `external_tasks`/`features` in memory while keeping `sync_pull_project`/`sync_push_project` in `src/adapter.rs` under `FORBIDDEN_SCOPE`, leaving background `save_component` calls writing stale `project_snapshot` fields to disk.
  - Finding 2 (`BUG-0058`): `coder` — `STEP 18D` `handle_edit_connection_key` omitted `form.active_field == 0` guard on `[` / `]`.
  - Finding 3 (`BUG-0059`): `coder` — `STEP 18F` `step_event_loop` blocks on `next_event(100ms)` even after draining `sync_rx` and setting `dirty = true`.
- **Unmet contract / test items:**
  - `STEP 18C` `Tests:` listed `delete_component_rolls_back_detached_children_on_failure`, which was omitted from `[POST_GATE]` and is missing in `src/tui/app/mod.rs` (filed as `TASK-0120`, origin: `plan`/`coder`).
- **Deferrals & amendments:**
  - `18D` was deferred once (needing `tests/e2e.rs` in `ALLOWED_FILES`), which cascaded `18E` and `18F` (`PRE_GATE` blocked). `18D` was restaged (`Restaged: 1`) and `18D`–`18F` all passed. No `.phased/amendments/` files exist.
- **Ready to close (`Advances:` + restage task):**
  - `BUG-0026`, `BUG-0027`, `BUG-0028`, `BUG-0029`, `BUG-0030`, `BUG-0031`, `BUG-0032`, `BUG-0040`, `BUG-0041`, `BUG-0042`, `BUG-0043`, `BUG-0054`, `BUG-0055`, `BUG-0056`, `TASK-0074`, `TASK-0075`, `TASK-0077`, and `TASK-0119` (`tests/e2e.rs` update for multi-edge `E` ordering). (`BUG-0024` and `BUG-0025` are already closed.)

## Handoff

- **Verdict:** PASS WITH FINDINGS (all 6 steps `18A`–`18F` implemented; `cargo fmt`, `clippy`, and all 181 tests pass).
- **New pending items by priority:**
  - High: `BUG-0057` (async sync `project_snapshot` overwrites concurrent foreground disk edits).
  - Medium: `BUG-0058` (`EditConnectionForm` `[`/`]` cycling intercepts `Label` input).
  - Low: `BUG-0059` (`step_event_loop` 100ms poll delay after draining `SyncOutcome`), `TASK-0120` (missing `delete_component_rolls_back_detached_children_on_failure` test from `STEP 18C`).
- **Deferred steps to re-split:** None (`18D`–`18F` completed after `18D` restage).
- **Decisions for user:** Approve closing the 17 open/pending `Advances:` items (`BUG-0026`..`BUG-0032`, `BUG-0040`..`BUG-0043`, `BUG-0054`..`BUG-0056`, `TASK-0074`, `TASK-0075`, `TASK-0077`) plus `TASK-0119`.
