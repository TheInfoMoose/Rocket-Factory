# Phase 17 review

Range: `ebaf3060e964a70969076677da8394c24a40ad68..7e92a0f41d842e221e09b8853d0c78f702804972`

## Independent pass

### Findings

1. **`src/adapter.rs:171-181`, `src/adapter.rs:285-321` — `regression`**
   - **Trigger:** Run `sync_pull_project` / `reconcile_status` after a previously synced external task is deleted or moved off a component in the external tracker (including when an adapter returns `tasks: []`).
   - **Effect:** `reconcile_status` now only upserts tasks by `external_id` and never removes tasks absent from the pulled response(s). Stale deleted tasks remain in `component.external_tasks` indefinitely, skewing `task_progress` and `compute_card_status` (e.g., preventing a component from transitioning to `Complete` or `Gap`). At `<base>`, `component.external_tasks = entry.tasks.clone()` replaced the list so removed tasks were pruned.

2. **`src/audit.rs:193-205`, `src/tui/app/actions.rs:191-266`, `src/tui/app/actions.rs:331-385`, `src/tui/app/actions.rs:132-138`, `src/tui/app/actions.rs:601-630` — `new defect`**
   - **Trigger:** Create a sub-component in the TUI (`a` / `ModalAction::CreateComponent` with `parent_id`, or `e` / `ModalAction::UpdateComponent` with `add_sub_component`), or delete/toggle off a `Contains` edge (`D` / `ModalAction::DeleteConnection` or `c` / `confirm_connect`).
   - **Effect:** `audit_project` now flags `FindingKind::HierarchyDrift` whenever a component has `parent_id` without a matching `EdgeKind::Contains` connection, yet TUI sub-component creation never adds a `Contains` connection and `DeleteConnection`/`confirm_connect` never clear `parent_id` when removing a `Contains` edge. Normal TUI sub-component workflows immediately generate `HierarchyDrift` warnings in `app.audit_findings` and `rktb audit`.

3. **`src/audit.rs:207-245` — `new defect`**
   - **Trigger:** A sub-component (`parent_id: Some(...)`) has a `GridPos` outside `grid.cols x grid.rows`, or occupies the same `GridPos` as a top-level component or a child of a different parent.
   - **Effect:** Because the grid-bounds check is in an `else if` branch after `if let Some(parent_id)` (line 207) and `occupied_positions` is keyed by `(component.parent_id.as_deref(), component.position)` (line 225), `audit_project` misses out-of-bounds sub-components and cross-parent/top-level overlaps even though all components share the same global canvas grid in `src/tui/app/grid.rs`.

4. **`src/tui/app/actions.rs:349-396` — `old bug`**
   - **Trigger:** Submit `ModalAction::UpdateComponent` with `add_sub_component` (and/or grid expansion) when `save_component(sub_comp)` or `save_project_config` fails after `save_component(parent)` succeeds.
   - **Effect:** Unlike `move_selected_card` and `ModalAction::CreateComponent`, no disk rollback is attempted; the updated parent (and possibly sub-component) persists on disk while `self.project` in memory remains unmutated, leaving disk and memory out of sync.

### Traced & verified clean
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` (147 unit tests + 13 e2e tests) all pass.
- `src/error.rs`, `src/main.rs`, `src/model.rs`, `src/theme.rs`, `src/export.rs`, `src/cli.rs`, `src/storage.rs`, `src/tui/canvas.rs`, `src/tui/mod.rs`, `src/tui/modal.rs`, `src/tui/terminal.rs`, and the `src/tui/app/` module split (`state.rs`, `grid.rs`, `actions.rs`, `sync.rs`, `mod.rs`) were inspected in full against `<base>`.
- Path containment (`ensure_contained`), `resolve_theme` fallback across multiple `search_dirs`, `save_component` `.toml`/`.md` rollback, `EditComponentForm` legacy `ComponentKind` preservation, and `push_unpushed_features` partial-progress persistence all behaved as intended.
- Sensitive-data scan of the diff found no real hosts, IPs, emails, tokens, or internal paths.

### Not checked
- Live interactive terminal rendering, real `$EDITOR` invocation, and real external adapter binaries outside the unit/e2e test suite.
- Non-Linux (`macOS`/`Windows`) behavior of `is_dir_sync_unsupported` (`raw_os_error() == Some(9)`).

<!-- SEALED: Independent pass locked -->

## Contract pass

### Step compliance (`17A`–`17F`)
- **STEP 17A (`TASK-0106`, `c448677`)**: Met all contract items (`RbError::{InvalidArgument, Validation, Json}`, `validate_theme_name`, CLI `--component` validation, `main.rs` exit code and `RKTB_POLL_INTERVAL_SECS` warning). All required tests present and passing.
- **STEP 17B (`TASK-0107`, `2bac916`)**: Met all contract items (`model.rs` serde defaults and `GridConfig::validate`, `storage.rs` `O_EXCL` temp files + dir sync + `.toml`/`.md` rollback + duplicate-ID/stem-mismatch checks, removal of `.rocket/` bypasses in `app.rs`).
- **STEP 17C (`TASK-0108`, `1049fb5`)**: Met all contract items as written in the plan (`audit.rs` new `FindingKind` variants and checks, `export.rs` `parent_id`/`position`, `Contains` exclusion, Markdown escaping, JSON trailing newline). However, the plan's spec for `HierarchyDrift` and scoped grid checks produced Findings #2 and #3.
- **STEP 17D (`TASK-0109`, `bc61b2f`)**: Met all contract items (`sync_pull_project`, `sync_push_project`, single JSON serialization, zero full-`Project` clone, `reconcile_status` upsert/dedup/unique-title linking, `push_unpushed_features` partial progress, `MarkdownRoadmap` component check, strict UTF-8 decoding, surfaced `PushResponse`). However, the plan's upsert-only `reconcile_status` spec introduced Finding #1 (`regression`).
- **STEP 17E (`TASK-0110`, `a7ed114`)**: Met all contract items (`EditComponentForm` legacy `ComponentKind` preservation, `pub(crate)` narrowing in `tui/{mod,terminal,app}`, dead canvas exports removed).
- **STEP 17F (`TASK-0111`, `6939805`)**: Met all contract items (`src/tui/app.rs` split into `mod.rs`/`state.rs`/`grid.rs`/`actions.rs`/`sync.rs`, `ensure_grid_contains` reuse, `storage::ensure_contained` deduplication across `theme.rs`, `terminal.rs`, and `actions.rs`).
- **Unrequested changes / validator misses:** None beyond the plan-originated defects below.
- **Deferrals & amendments:** `STEP 17E` and `STEP 17F` were deferred mid-phase (`2e15cc4`, `0efafe1`) due to a `POST_GATE` grep mismatch on `TuiTerminal` (`TASK-0112`), then restaged (`9eb1a69`) and completed cleanly. No amendments were filed.

### Independent finding origins & tracker mapping
1. **Finding #1 (`src/adapter.rs` — `reconcile_status`/`sync_pull_project` never prunes deleted external tasks, `regression`)**: Origin **`plan`** (STEP 17D required upsert-only reconciliation without specifying deleted-task pruning). Filed as **`BUG-0054`** (`pending`, `medium`).
2. **Finding #2 (`src/audit.rs` vs `src/tui/app/actions.rs` — TUI sub-component creation & `Contains` deletion trigger `HierarchyDrift`, `new defect`)**: Origin **`plan`** (STEP 17C added `HierarchyDrift` while TUI `Contains` sync was deferred to Phase 18 / `BUG-0029`). Filed as **`BUG-0055`** (`pending`, `medium`; `DeleteConnection`/`confirm_connect` part already tracked in `BUG-0029`).
3. **Finding #3 (`src/audit.rs` — skips grid-bounds for sub-components and misses cross-parent overlaps, `new defect`)**: Origin **`plan`** (STEP 17C explicitly scoped bounds to top-level and overlaps to `(parent_id, pos)` despite shared global grid). Filed as **`BUG-0056`** (`pending`, `medium`).
4. **Finding #4 (`src/tui/app/actions.rs` — `UpdateComponent` lacks multi-file rollback, `old bug`)**: Origin **`pre-existing`**. Duplicate of open **`BUG-0026`** (scheduled for Phase 18); not re-filed.

### `Advances:` items ready to close (pending user approval)
- `TASK-0069`, `TASK-0070`, `TASK-0071`, `TASK-0072`, `TASK-0073`, `TASK-0076`, `TASK-0112`
- `BUG-0033`, `BUG-0034`, `BUG-0035`, `BUG-0036` (follow-up regression in `BUG-0054`), `BUG-0037`, `BUG-0038` (follow-up defects in `BUG-0055`/`BUG-0056`), `BUG-0039`, `BUG-0044`, `BUG-0045`, `BUG-0046`

## Handoff

- **Verdict:** PASS WITH FOLLOW-UPS (all 6 steps `17A`–`17F` landed cleanly and pass fmt/clippy/tests; 1 regression and 2 plan-originated defects filed for follow-up).
- **New pending items (medium):**
  - `BUG-0054`: `sync_pull_project` / `reconcile_status` never prunes deleted external tasks (`regression`, origin `plan`).
  - `BUG-0055`: TUI sub-component creation triggers immediate `HierarchyDrift` audit warnings (`new defect`, origin `plan`; pair with `BUG-0029` in Phase 18).
  - `BUG-0056`: `audit_project` skips grid-bounds checks for sub-components and misses cross-parent grid overlaps (`new defect`, origin `plan`).
- **Pre-existing duplicate (not re-filed):** Finding #4 (`UpdateComponent` rollback) is already tracked as `BUG-0026` (Phase 18).
- **Deferred steps to re-split:** None (`17E` and `17F` were restaged and completed).
- **Ready to close on approval:** `TASK-0069`, `TASK-0070`, `TASK-0071`, `TASK-0072`, `TASK-0073`, `TASK-0076`, `TASK-0112`, `BUG-0033`, `BUG-0034`, `BUG-0035`, `BUG-0036`, `BUG-0037`, `BUG-0038`, `BUG-0039`, `BUG-0044`, `BUG-0045`, `BUG-0046`.
- **Decisions for user:** Confirm whether `BUG-0054`–`BUG-0056` should be folded into Phase 18 alongside `BUG-0029`, and whether `BUG-0036`/`BUG-0038` should be closed now (with follow-ups tracked in `BUG-0054`–`BUG-0056`).
