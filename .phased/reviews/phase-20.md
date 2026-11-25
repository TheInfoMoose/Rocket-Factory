# Phase 20 review

Range: `fd2aee2..eac0980`

## Independent pass

### Findings

1. **`.github/workflows/release.yml:13-92`** — `new defect`
   - **Trigger:** Pushing a new `v*` tag triggers all 3 matrix jobs (`ubuntu-latest`, `macos-latest`, `windows-latest`) in parallel, and each job independently invokes `softprops/action-gh-release` (`release.yml:86-92`) without a pre-created release or a single downstream publish job.
   - **Effect:** Two runners finishing close together can race to create the same GitHub Release (`POST /releases`), causing a spurious HTTP 422 `already_exists` failure on one of the platform jobs. A two-stage `build` (matrix + `actions/upload-artifact`) → `publish` (single job + `softprops/action-gh-release`) pattern avoids the race.

2. **`.github/workflows/release.yml:59,78`** — `new defect`
   - **Trigger:** Dispatching the `Release` workflow manually (`workflow_dispatch`) from a branch whose name contains `/` (e.g., `release/v0.1`).
   - **Effect:** `GITHUB_REF_NAME` is interpolated directly into the archive filename (`rktb-${GITHUB_REF_NAME}-${TARGET}.tar.gz` / `rktb-$env:GITHUB_REF_NAME-$env:TARGET.zip`). The `/` is interpreted as a directory separator and `tar` / `Compress-Archive` fails because the parent directory does not exist.

### What was traced
- All 11 non-tracker/non-plan changed files in `fd2aee2..eac0980`: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.gitignore`, `README.md`, `src/adapter.rs`, `src/cli.rs`, `src/tui/app/mod.rs`, `src/tui/app/sync.rs`, `src/tui/terminal.rs`, `src/tui/ui.rs`, `tests/e2e.rs`.
- Local verification executed: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` (185 unit + 14 e2e passed), `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --locked`, and `bash -n scripts/phased-step.sh scripts/neon-dispatch`.
- Traced `src/tui/app/sync.rs` (`diff_component_updates`, `run_sync_job`, `apply_sync_outcome`) against `src/adapter.rs` for partial push persistence on error, concurrent foreground feature/component edits and deletions, and rollback on partial `save_component` failure.
- Traced `src/tui/ui.rs:1053-1059` footer hint width (75 cols on Kind field, 59 cols on Label field; fits 80-col terminal `popup_inner_width = 76` and wraps cleanly below 77 cols) against `src/tui/modal.rs:595-660`.
- Traced `#[cfg(unix)]` test gating and imports across `src/adapter.rs`, `src/cli.rs`, `src/tui/app/mod.rs`, `src/tui/terminal.rs`, and Windows PowerShell UTF-16LE base64 mock helpers in `tests/e2e.rs`.
- Checked all changed files for sensitive data (none found).

### What was not checked
- `shellcheck -S error scripts/phased-step.sh scripts/neon-dispatch` (`shellcheck` is not installed in this environment).
- Native Windows and macOS compilation, clippy, and test execution (including PowerShell `-EncodedCommand` mock execution in `tests/e2e.rs` and Windows `#[cfg(not(unix))]` dead-code/unused-import checks).
- Live GitHub Actions execution (`rustsec/audit-check`, `cargo package --locked` in a clean runner, `softprops/action-gh-release` tag upload).

<!-- SEALED: Independent pass locked -->

## Contract pass

### Step compliance
- **STEP 20A (`TASK-0128`, closed):** All `Contract & Behavior`, `Must still work`, and `Tests` requirements met. Note: `async_sync_preserves_concurrent_foreground_feature_edits` exercises `run_sync_job` with a real `sh` mock adapter and is gated `#[cfg(unix)]` (stronger on Unix, skipped on Windows).
- **STEP 20B (`TASK-0129`, closed):** All contract items met (`EditConnection` active-field footer hint, `Cli` `version`, `rktb --version` subprocess E2E test).
- **STEP 20C (`TASK-0130`, closed):** All contract items met (UTF-16LE base64 PowerShell mock helpers + unit test, `#[cfg(unix)]` gating in `src/cli.rs` and `src/tui/terminal.rs`).
- **STEP 20D (`TASK-0131`, closed):** All contract items met (`#[cfg(unix)]` gating in `src/adapter.rs` and `src/tui/app/mod.rs` with matching import gates, plus `sync_push_project_rejects_missing_component_cross_platform`).
- **STEP 20E (`TASK-0132`, closed):** All contract items met. Both independent findings originate in the step contract itself (single matrix job invoking `softprops/action-gh-release` directly and interpolating `github.ref_name` unsanitized).
- **STEP 20F (`TASK-0133`, closed):** Deferred once (`e606a6f`) due to `scripts/phased-step.sh` flagging pre-existing untracked `.agents/` files that became newly gitignored (`BUG-0064` filed as `pending`), restaged (`6253f9b`), and completed cleanly (`beb9a90`). All `.gitignore` and `README.md` contract items met.

### Amendments & deviations
- No `.phased/amendments/` files in Phase 20.
- No unrequested code changes or validator lapses.

### Finding origins & tracker IDs
| # | Location | Class | Origin | Tracker ID | Severity |
| :-: | :--- | :--- | :--- | :--- | :--- |
| 1 | `.github/workflows/release.yml:13-92` | `new defect` | `plan` | `BUG-0065` | medium |
| 2 | `.github/workflows/release.yml:59,78` | `new defect` | `plan` | `BUG-0066` | low |

### `Advances:` items ready to close on approval
- `BUG-0060`, `BUG-0063` (STEP 20A)
- `BUG-0061`, `TASK-0127` (STEP 20B)
- `BUG-0062` (STEP 20C, STEP 20D)
- `TASK-0083`, `TASK-0084`, `TASK-0085`, `TASK-0086` (STEP 20E)
- `TASK-0087`, `TASK-0091`, `TASK-0094` (STEP 20F)
- `TASK-0120` (already satisfied by `delete_component_rolls_back_detached_children_on_failure` in `src/tui/app/mod.rs:2313`, per 2026-11-24 plan decision)

## Handoff

**Verdict:** PASS WITH FOLLOW-UPS (all 6 steps `20A`–`20F` complete; 0 regressions; 2 plan-originated defects in `release.yml`, plus 1 pre-existing harness bug `BUG-0064`).

**Pending items by priority:**
- **Medium:** `BUG-0065` (split `.github/workflows/release.yml` into matrix `build` + single `publish` job to avoid parallel `action-gh-release` create race), `BUG-0064` (`scripts/phased-step.sh` flags newly gitignored pre-existing untracked files as out-of-scope).
- **Low:** `BUG-0066` (sanitize `/` in `GITHUB_REF_NAME` for `release.yml` `workflow_dispatch` archive names).

**Deferred steps to re-split:** None (`STEP 20F` was restaged and completed cleanly).

**Decisions for user:**
- Approve closing Phase 20 `Advances:` items (`BUG-0060`, `BUG-0061`, `BUG-0062`, `BUG-0063`, `TASK-0083`–`TASK-0087`, `TASK-0091`, `TASK-0094`, `TASK-0127`) plus already-satisfied `TASK-0120`.
- Decide whether to schedule a short Phase 21 polish pass for `BUG-0065`, `BUG-0066`, and `BUG-0064` (note: `scripts/*` requires temporarily unlocking `**Forbidden paths:**` if fixing `BUG-0064`), or cut `v0.1.0` directly.
