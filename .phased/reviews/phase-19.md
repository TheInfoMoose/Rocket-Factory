# Phase 19 review

Range: `75cc0a3d9a8dd4008b69ceec1f2252fa4270d0e5..0706fa5d9ea1b3dddff6f1be2386cf5b8fc95530`

## Independent pass

Traced all 12 changed code files (`src/adapter.rs`, `src/audit.rs`, `src/cli.rs`, `src/export.rs`, `src/model.rs`, `src/storage.rs`, `src/theme.rs`, `src/tui/app/sync.rs`, `src/tui/modal.rs`, `src/tui/style.rs`, `src/tui/terminal.rs`, `tests/e2e.rs`) in full against base `75cc0a3d`, ran `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` (181 unit + 13 e2e tests passed), and scanned diffs for sensitive data (none found).

### Findings

1. **Partial push progress lost on error in TUI async sync** — `regression`
   - **Location:** `src/tui/app/sync.rs:66-122`, `src/tui/app/sync.rs:172-175`, `src/adapter.rs:805-821`
   - **Trigger:** User triggers an async push (`SyncRequest::PushSelected`) on a component with multiple unpushed features; feature 1 succeeds against the external adapter (marking `pushed = true` and `external_id = Some(...)` on the worker's cloned `Project`), and feature 2 fails.
   - **Effect:** At `75cc0a3d`, `run_sync_job` called `sync_push_project`, which saved partial push progress to disk before returning `Err`. Now `run_sync_job` calls `sync_push_project_in_memory` (no disk write) and returns `SyncOutcome::Error { status_message }` without carrying partial `updates`. Neither disk nor in-memory `app.project` records that feature 1 was pushed, so retrying push will re-push feature 1 and create duplicate external items.

2. **EditConnection footer hint contradicts `[`/`]` behavior on Label field** — `new defect`
   - **Location:** `src/tui/modal.rs:597-616`, `src/tui/ui.rs:1053-1055`
   - **Trigger:** User opens `EditConnection` with multiple candidate edges, tabs to the Label field (`active_field == 1`), and reads the footer hint `[[/]] Edge`.
   - **Effect:** Pressing `[` or `]` inserts literal bracket characters into the Label instead of cycling edges, and the footer hint does not mention `PageUp`/`PageDown` (the only keys that cycle edges while on the Label field).

3. **Windows mock command helpers in `tests/e2e.rs` produce invalid JSON under `cmd /C` escaping; new unit tests hardcode `sh`** — `test defect`
   - **Location:** `tests/e2e.rs:56-89`, `src/cli.rs:353-387`, `src/tui/terminal.rs:1045`, `src/tui/terminal.rs:1205`
   - **Trigger:** Running `cargo test` on Windows (`#[cfg(windows)]`).
   - **Effect:** `mock_echo_command` and `mock_log_and_echo_command` pass `echo {response_json}` and `findstr "^"` inside a `cmd /C` argument; Rust's Windows `Command` escapes `"` as `\"`, which `cmd.exe` does not unescape, producing invalid JSON (`{\"external_id\":...}`) and broken `findstr` arguments. `mock_log_and_echo_command` also uses `more` (an interactive pager). Meanwhile, the newly added adapter tests in `src/cli.rs` and `src/tui/terminal.rs` invoke `sh -c` without `#[cfg(unix)]` or cross-platform helpers.

4. **Deletion of `e2e_readme_documents_release_build_and_tui_features`** — `test defect`
   - **Location:** `tests/e2e.rs` (lines 1396-1448 at `75cc0a3d`)
   - **Trigger:** Test was removed and not relocated elsewhere in `src/` or `tests/`.
   - **Effect:** Removes automated verification that `README.md` stays in sync with release build/install commands, TUI keybindings, and component archetypes/sub-components.

5. **Concurrent foreground `features` edits still clobbered by async sync apply** — `old bug`
   - **Location:** `src/tui/app/sync.rs:142-143`
   - **Trigger:** While an async pull/push is in flight, the user adds or edits a local feature on component `A`, and the sync returns updated `external_tasks` or `features` for component `A`.
   - **Effect:** `apply_sync_outcome` overwrites `comp.features = update.features` from the stale worker snapshot, discarding the foreground feature change (whereas `description`, `position`, `markdown_body`, and deletions are now safely preserved).

### Not checked
- Windows build and test execution (reviewed on Linux only).
- Real TTY/PTY rendering and panic-hook terminal restoration.
- Line/branch coverage metrics (`cargo llvm-cov` / `tarpaulin`).

<!-- SEALED: Independent pass locked -->

## Contract pass

### Step-by-step check
- **STEP 19A (`TASK-0121`, Advances `BUG-0057`, `BUG-0059`):** Met all explicit contract items (`sync_pull_project_in_memory`, `sync_push_project_in_memory`, UI-thread persistence with rollback, immediate redraw with `Duration::ZERO`, and both required tests). However, moving push persistence out of `run_sync_job` without returning partial `updates` on `SyncOutcome::Error` introduced **Finding #1 (`BUG-0060`)**.
- **STEP 19B (`TASK-0122`, Advances `BUG-0058`):** Met contract and test (`edit_connection_allows_brackets_in_label_field`). `src/tui/ui.rs` was outside `ALLOWED_FILES`, leaving the footer hint out of sync (**Finding #2 / `BUG-0061`**).
- **STEP 19C (`TASK-0123`, Advances `TASK-0078`, `TASK-0082`):** Deferred once (`26b1b43`, `.phased/deferred/19C.md`) due to scope lapse on `TASK-0127` (`--version` not enabled in `Cli`), restaged (`c430cfc`) without `--version`, and completed (`1318d4f`). Met `TASK-0078` subprocess tests and Unix `TMPDIR` / README decoupling (`TASK-0082`), but the Windows `cmd /C` helpers break JSON quoting and use interactive `more` (**Finding #3 / `BUG-0062`**).
- **STEP 19D (`TASK-0124`, Advances `TASK-0079`, `TASK-0080`):** Met all `terminal.rs` and `storage.rs` test contracts. New adapter tests in `src/tui/terminal.rs` hardcode `sh -c` (included in **`BUG-0062`**).
- **STEP 19E (`TASK-0125`, Advances `TASK-0081` Part 1):** Met all `cli.rs`, `audit.rs`, and `export.rs` test contracts (coder accurately adapted minor plan typos `export_component_manifest` → `export_component_manifest_toml` and `[pushed]` → `| pushed |`). New adapter tests in `src/cli.rs` hardcode `sh -c` (included in **`BUG-0062`**).
- **STEP 19F (`TASK-0126`, Advances `TASK-0081` Part 2):** Met all `model.rs`, `theme.rs`, and `tui/style.rs` test contracts (coder accurately adapted non-existent `ComponentKind::ALL`/`as_str`/`Display` in plan to full 11-variant TOML roundtrip + `Default`).

### Finding origins & tracker mapping
| # | Class | Origin | Neb Item | Notes |
| :---: | :--- | :--- | :--- | :--- |
| 1 | `regression` | `plan` / `coder` | `BUG-0060` (`high`, pending) | `STEP 19A` omitted partial-push `updates` on `SyncOutcome::Error`. |
| 2 | `new defect` | `plan` | `BUG-0061` (`low`, pending) | `STEP 19B` scoped only `modal.rs`, omitting `ui.rs` footer hint. |
| 3 | `test defect` | `coder` | `BUG-0062` (`medium`, pending) | Windows `cmd /C` escaping in `tests/e2e.rs`; un-gated `sh` in `cli.rs`/`terminal.rs`. |
| 4 | `test defect` | `plan` | — (decision) | Explicitly requested by `STEP 19C` (`TASK-0082`); not withdrawn, flagged for user confirmation. |
| 5 | `old bug` | `pre-existing` | `BUG-0063` (`low`, pending) | Pre-existing snapshot overwrite of concurrent `features` edits. |

### Deferrals & amendments
- **Deferral `.phased/deferred/19C.md`:** Resolved by restage (`c430cfc`) and completion (`1318d4f`). `TASK-0127` remains `pending` for Phase 20 decision on `--version`.
- **Amendments:** None in `.phased/amendments/`.

### Ready to close (pending user `approved`)
`BUG-0057`, `BUG-0058`, `BUG-0059`, `TASK-0078`, `TASK-0079`, `TASK-0080`, `TASK-0081`, `TASK-0082` (with Windows mock defect split into `BUG-0062`).

## Handoff

- **Verdict:** Pass with follow-ups (`BUG-0060` high regression, `BUG-0062` medium Windows test defect blocking Phase 20 Windows CI, `BUG-0061`/`BUG-0063` low).
- **Filed pending items by priority:**
  - High: `BUG-0060` (TUI async push drops partial push progress on error → duplicate pushes on retry)
  - Medium: `BUG-0062` (Windows `cmd /C` JSON escaping in `tests/e2e.rs` + un-gated `sh` in `src/cli.rs` and `src/tui/terminal.rs`)
  - Low: `BUG-0061` (`EditConnection` footer hint on Label field), `BUG-0063` (pre-existing: async sync clobbers concurrent foreground `features` edits)
- **Prior pending items still open:** `TASK-0127` (high: enable `--version` in `Cli` or drop), `TASK-0120` (low: missing `delete_component_rolls_back_detached_children_on_failure` test).
- **Deferred steps to re-split:** None (`STEP 19C` was deferred once on `TASK-0127`, restaged, and completed).
- **Decisions for user:**
  1. Confirm closing `BUG-0057`, `BUG-0058`, `BUG-0059`, `TASK-0078`, `TASK-0079`, `TASK-0080`, `TASK-0081`, `TASK-0082` (Windows mock defect tracked in `BUG-0062`).
  2. Decide on `TASK-0127`: add `#[command(version)]` to `Cli` in Phase 20 or close as won't-fix.
  3. Accept removal of `e2e_readme_documents_release_build_and_tui_features` (`STEP 19C`), or track README drift checks in Phase 20 (`TASK-0091`).
