# Rocket Factory — Stepped Implementation Plan

## Step Execution Roadmap

**Workflow:** phased

**Base gate:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

**Forbidden paths:** `Rocket Factory.md:STEPPED_IMPLEMENTATION_PLAN.md:docs/*:.phased/*:.orrery/*:scripts/*`

**Planning files:** `Rocket Factory.md:docs/*`

**How to read this:** work goes strictly top to bottom through **Up Next**, one step at a time. The **#** column is the only order. Step IDs are kept because commits and notes cite them. A step whose "Waits on" names an Open Question or a **[USER]** action doesn't start until that's resolved.

**Who runs what (phased workflow + Orrery `gemini-phased`):**
- **Planner / Reviewer (Claude or review subagent)** writes the active phase (contract-based step blocks specifying scope, public interface, behavioral invariants, and test outcomes — without pre-writing implementation code), outlines future phases, and performs one phase review at the end of each phase using snapshot-backed step commits (`git show <sha>`) checking scope adherence, code quality, functionality, and consistency.
- **Orchestrator / Validator (Hydrogen / Orrery `agy-hydrogen-orch` + `agy-hydrogen-val`)** runs the phase. At the start of each step, `phased:step_start` (`scripts/phased-step.sh pregate` + `neb start` + `scripts/phased-step.sh start`) takes a repository snapshot. After Neon finishes, `phased:step_check` (`scripts/phased-step.sh check`) validates scope and `POST_GATE` strictly against that snapshot diff, the validator reads the diff, and `phased:step_commit` commits the step and closes its Nebulous task.
- **Coder (Neon / Orrery `agy-neon-impl`)** executes one step inside `ALLOWED_FILES`. It never commits, never edits this plan or forbidden paths, and files bugs or scope lapses as **pending** Nebulous items.
- **Two code-fault failures** on one step, or an unamendable scope lapse → the step is deferred (`phased:step_defer`: saves attempt patch, restores step files, parks the task). A step whose `PRE_GATE` fails is deferred without dispatching an agent. Deferrals exceeding `max(2, ⌈steps/3⌉)` stop the phase early.

**Phase goal (Phase 8):** Complete TUI CRUD workflows with confirmation-guarded deletion of components (`d`) and connections (`D`), deduplicate the multi-adapter feature push loop into a shared `src/adapter.rs` helper (`TASK-0039`), and add `README.md` documenting CLI usage, TUI keybindings, `.ron` themes, and external adapter contracts.

**Side findings, bugs and scope lapses** become **pending neb items**, reviewed once at phase end. Every Up Next row has a neb task (`source_ref` = `STEP <ID>`); commits start with `[<TASK-ID>](<scope>)`.

### Up Next

| # | Step | Command | Scope | Target Files | Waits on |
| :---: | :--- | :---: | :--- | :--- | :--- |
| 1 | **STEP 8A** | `/goal` | adapter | `src/adapter.rs`, `src/cli.rs`, `src/tui/app.rs` | — |
| 2 | **STEP 8B** | `/goal` | modal | `src/tui/modal.rs` | — |
| 3 | **STEP 8C** | `/goal` | tui | `src/tui/app.rs`, `src/tui/ui.rs`, `tests/e2e.rs` | STEP 8A, STEP 8B |
| 4 | **STEP 8D** | `/goal` | docs | `README.md` | STEP 8C |

### Completed

| Step ID | Status | Command | Scope | Target Files |
| :--- | :---: | :---: | :--- | :--- |
| **STEP 1A** | Done (`60de1ea`) | `/goal` | core | `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/error.rs`, `src/model.rs` |
| **STEP 1B** | Done (`c9ff16e`) | `/goal` | storage | `src/storage.rs`, `src/lib.rs` |
| **STEP 1C** | Done (`bcf6670`) | `/goal` | archetypes | `src/archetype.rs`, `src/lib.rs` |
| **STEP 1D** | Done (`d9d4c45`) | `/goal` | audit | `src/audit.rs`, `src/lib.rs` |
| **STEP 1E** | Done (`8659ced`) | `/goal` | export | `src/export.rs`, `src/lib.rs` |
| **STEP 1F** | Done (`3d4b5b0`) | `/goal` | theme | `src/theme.rs`, `src/lib.rs` |
| **STEP 2A** | Done (`c5c78e7`) | `/goal` | adapter | `src/adapter.rs`, `src/error.rs`, `src/lib.rs` |
| **STEP 2B** | Done (`56aca5a`) | `/goal` | adapter | `src/adapter.rs` |
| **STEP 2C** | Done (`94cdbb8`) | `/goal` | cli | `Cargo.toml`, `Cargo.lock`, `src/cli.rs`, `src/lib.rs` |
| **STEP 2D** | Done (`eb80b6e`) | `/goal` | cli | `Cargo.toml`, `src/main.rs` |
| **STEP 3A** | Done (`7e72722`) | `/goal` | tui | `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/tui/mod.rs`, `src/tui/app.rs` |
| **STEP 3B** | Done (`bc090b6`) | `/goal` | tui | `src/tui/style.rs`, `src/tui/mod.rs` |
| **STEP 3C** | Done (`b678069`) | `/goal` | tui | `src/tui/ui.rs`, `src/tui/mod.rs` |
| **STEP 3D** | Done (`da2155b`) | `/goal` | tui | `src/tui/terminal.rs`, `src/tui/mod.rs`, `src/main.rs` |
| **STEP 4A** | Done (`660165a`) | `/goal` | canvas | `src/tui/canvas.rs`, `src/tui/mod.rs` |
| **STEP 4B** | Done (`20884cc`) | `/goal` | canvas | `src/tui/canvas.rs` |
| **STEP 4C** | Withdrawn (→ 4E, 4F) | `/goal` | canvas | `src/tui/ui.rs`, `src/tui/app.rs` |
| **STEP 4D** | Withdrawn (→ 4F) | `/goal` | canvas | `src/tui/app.rs` |
| **STEP 4E** | Done (`5c5e038`) | `/goal` | canvas | `src/tui/canvas.rs` |
| **STEP 4F** | Done (`6035b49`) | `/goal` | canvas | `src/tui/app.rs`, `src/tui/ui.rs` |
| **STEP 5A** | Done (`3e71980`) | `/goal` | modal | `src/tui/modal.rs`, `src/tui/mod.rs` |
| **STEP 5B** | Done (`b488452`) | `/goal` | modal | `src/tui/app.rs`, `src/tui/ui.rs` |
| **STEP 5C** | Done (`2b3dd73`) | `/goal` | editor | `src/tui/app.rs`, `src/tui/terminal.rs` |
| **STEP 6A** | Done (`38f38eb`) | `/goal` | editor | `src/tui/terminal.rs` |
| **STEP 6B** | Done (`f1915a3`) | `/goal` | tui | `src/tui/app.rs`, `src/tui/ui.rs`, `src/tui/canvas.rs` |
| **STEP 6C** | Done (`b4676eb`) | `/goal` | sync | `src/tui/app.rs`, `src/tui/terminal.rs`, `src/main.rs` |
| **STEP 6D** | Done (`9bd03c9`) | `/goal` | theme | `src/theme.rs` |
| **STEP 6E** | Done (`362de34`) | `/goal` | overlay | `src/tui/modal.rs`, `src/tui/app.rs`, `src/tui/ui.rs` |
| **STEP 6F** | Done (`85d32c9`) | `/goal` | tests | `tests/e2e.rs` |
| **STEP 7A** | Done (`c27efb0`) | `/goal` | tui | `src/tui/terminal.rs`, `src/tui/app.rs` |
| **STEP 7B** | Done (`3ea3c39`) | `/goal` | ui | `src/tui/ui.rs` |
| **STEP 7C** | Done (`3a3422b`) | `/goal` | sync | `src/cli.rs`, `src/tui/app.rs`, `tests/e2e.rs` |

### Future phases

_None planned — Phase 8 completes TUI CRUD deletion flows, resolves `TASK-0039`, and documents Rocket Factory for end-user adoption._

### Open Questions

_None._

### Parked

Tracked in neb: `neb list --status pending`. Reviewed at phase end; only picked items become steps.

---

## Phase 1 Step Blocks

### STEP 1A: Cargo Manifest, Error Types, and Core Domain Model

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 1A: Create Cargo.toml, src/lib.rs, src/error.rs, and src/model.rs defining the Rocket Factory domain types and unit tests.

[SCOPE_LOCK]
ALLOWED_FILES:
  - Cargo.toml (NEW)
  - Cargo.lock (NEW)
  - src/lib.rs (NEW)
  - src/error.rs (NEW)
  - src/model.rs (NEW)
READ_ONLY_REFS:
  - Rocket Factory.md:1-25 (requirements overview)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/ or Cargo.toml.
  - No TUI or CLI crates (ratatui, crossterm, clap) in Cargo.toml yet.

[PRE_GATE]
true

[MUTATION_SPEC]
1. Cargo.toml (NEW) — create package manifest:
   - `[package]`: `name = "rocket-factory"`, `version = "0.1.0"`, `edition = "2021"`, `description = "Retro-futuristic TUI development planning platform"`.
   - `[lib]`: `name = "rocket_factory"`, `path = "src/lib.rs"`.
   - `[dependencies]`: `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `toml = "0.8"`, `ron = "0.8"`, `thiserror = "2"`.
   - `[dev-dependencies]`: `tempfile = "3"`.
2. src/lib.rs (NEW) — declare modules and re-export error result:
   - `pub mod error;`
   - `pub mod model;`
   - `pub use error::{RfError, RfResult};`
3. src/error.rs (NEW) — define `RfError` using `#[derive(Debug, thiserror::Error)]` and `pub type RfResult<T> = std::result::Result<T, RfError>;` with variants:
   - `Io { path: std::path::PathBuf, #[source] source: std::io::Error }` (`"I/O error at {path}: {source}"`)
   - `InvalidComponentId(String)` (`"invalid component id: {0}"`)
   - `PathEscape(std::path::PathBuf)` (`"path escapes workspace boundary: {0}"`)
   - `TomlParse { path: std::path::PathBuf, #[source] source: toml::de::Error }` (`"TOML parse error in {path}: {source}"`)
   - `TomlSerialize(#[from] toml::ser::Error)` (`"TOML serialization error: {0}"`)
   - `JsonSerialize(#[from] serde_json::Error)` (`"JSON serialization error: {0}"`)
   - `RonParse { path: std::path::PathBuf, #[source] source: ron::error::SpannedError }` (`"RON parse error in {path}: {source}"`)
   - `NotFound(String)` (`"not found: {0}"`)
   - `AlreadyExists(String)` (`"already exists: {0}"`)
4. src/model.rs (NEW) — define serde (`Serialize, Deserialize`) domain types:
   - `pub struct GridPos { pub col: u16, pub row: u16 }` (`Debug, Clone, Copy, PartialEq, Eq, Hash, Default`).
   - `pub enum ComponentKind { UiView, DataStream, ServiceCore, CliTool, Storage, Custom }` (`Debug, Clone, Copy, PartialEq, Eq, Hash, Default` with `#[default] Custom` and `#[serde(rename_all = "snake_case")]`).
   - `pub enum EdgeKind { DependsOn, DataFlow, Renders, Invokes, Custom }` (`Debug, Clone, Copy, PartialEq, Eq, Hash, Default` with `#[default] DependsOn` and `#[serde(rename_all = "snake_case")]`).
   - `pub enum TaskStatus { Pending, InProgress, Blocked, Complete }` (`Debug, Clone, Copy, PartialEq, Eq, Hash, Default` with `#[default] Pending` and `#[serde(rename_all = "snake_case")]`).
   - `pub enum CardStatus { Gap, Planned, InProgress, Blocked, Complete }` (`Debug, Clone, Copy, PartialEq, Eq, Hash`).
   - `pub struct ExternalTaskRef { pub external_id: String, pub title: String, pub status: TaskStatus }` (`Debug, Clone, PartialEq, Eq`).
   - `pub struct FeatureItem { pub id: String, pub title: String, pub pushed: bool, pub external_id: Option<String> }` (`Debug, Clone, PartialEq, Eq`).
   - `pub struct Connection { pub from: String, pub to: String, pub kind: EdgeKind, pub label: Option<String> }` (`Debug, Clone, PartialEq, Eq`).
   - `pub struct Component { pub id: String, pub name: String, pub kind: ComponentKind, pub description: String, pub relative_path: Option<String>, pub position: GridPos, pub tags: Vec<String>, pub tech_stack: Vec<String>, pub files: Vec<String>, pub flags: Vec<String>, pub features: Vec<FeatureItem>, pub external_tasks: Vec<ExternalTaskRef>, #[serde(skip)] pub markdown_body: String }` (`Debug, Clone, PartialEq, Eq`).
   - `pub struct GridConfig { pub cols: u16, pub rows: u16 }` (`Debug, Clone, Copy, PartialEq, Eq`; `Default` returns `cols: 8, rows: 6`).
   - `pub struct ProjectConfig { pub name: String, pub description: String, pub theme: String, pub active_adapters: Vec<String>, pub grid: GridConfig, pub connections: Vec<Connection> }` (`Debug, Clone, PartialEq, Eq`).
   - `pub struct Project { pub config: ProjectConfig, pub components: Vec<Component> }` (`Debug, Clone, PartialEq, Eq`).
   - Helper method `impl Project { pub fn component(&self, id: &str) -> Option<&Component> }` and `pub fn component_mut(&mut self, id: &str) -> Option<&mut Component>`.
5. Tests in `src/model.rs`:
   - `model_toml_roundtrip`: construct a `ProjectConfig` with 1 `Connection` (`EdgeKind::DataFlow`, label `Some("events".into())`) and a `Component` with `ComponentKind::UiView`, 1 `FeatureItem`, and 1 `ExternalTaskRef` (`TaskStatus::Complete`); serialize both with `toml::to_string_pretty` and deserialize back; assert equality (`assert_eq!(parsed_cfg, cfg)` and `assert_eq!(parsed_comp, comp)` with `markdown_body` empty).
   - `project_component_lookup`: construct a `Project` with 2 components (`"ui_view"`, `"stream_core"`); assert `project.component("ui_view").is_some()` and `project.component("missing").is_none()`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub struct Project\b' src/model.rs && grep -qE 'pub enum RfError\b' src/error.rs && ! grep -rqi 'nebulous' src/ Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 1B: Atomic `.rocket/` Workspace Storage

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 1B: Implement atomic, path-safe .rocket/ workspace persistence (TOML + Markdown) in src/storage.rs and expose it from src/lib.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/storage.rs (NEW)
  - src/lib.rs
READ_ONLY_REFS:
  - src/error.rs:1-40 (RfError, RfResult)
  - src/model.rs:1-140 (Project, ProjectConfig, Component, GridConfig)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub struct Project\b' src/model.rs && grep -qE 'pub enum RfError\b' src/error.rs

[MUTATION_SPEC]
1. src/lib.rs — anchor `pub mod model;` → add `pub mod storage;` directly beneath it.
2. src/storage.rs (NEW) — define workspace constants and functions:
   - `pub const ROCKET_DIR: &str = ".rocket";`
   - `pub const PROJECT_FILE: &str = "project.toml";`
   - `pub const COMPONENTS_DIR: &str = "components";`
   - `pub fn validate_component_id(id: &str) -> RfResult<()>`: rejects empty strings, strings longer than 64 chars, or any char outside `a-z`, `0-9`, `_`, `-` (and rejects leading `-`), returning `Err(RfError::InvalidComponentId(id.to_string()))`.
   - `fn atomic_write(path: &std::path::Path, contents: &str) -> RfResult<()>`: writes to a sibling temporary file `<filename>.tmp.<pid>` in the same parent directory, flushes/syncs, and atomically renames to `path` via `std::fs::rename`.
   - `fn ensure_contained(base: &std::path::Path, candidate: &std::path::Path) -> RfResult<std::path::PathBuf>`: canonicalizes `base` and verifies the target parent canonicalizes within `base` (`starts_with`), returning `Err(RfError::PathEscape(candidate.to_path_buf()))` on escape.
   - `pub fn init_workspace(root: &std::path::Path, name: &str, description: &str) -> RfResult<Project>`: creates `<root>/.rocket/components/`, fails with `RfError::AlreadyExists` if `<root>/.rocket/project.toml` already exists, writes default `ProjectConfig` (`theme = "crt_amber"`, `active_adapters = []`, `grid = GridConfig::default()`, `connections = []`) atomically, and returns `Project`.
   - `pub fn save_project_config(root: &std::path::Path, config: &ProjectConfig) -> RfResult<()>`: validates containment and atomically writes `<root>/.rocket/project.toml`.
   - `pub fn save_component(root: &std::path::Path, component: &Component) -> RfResult<()>`: calls `validate_component_id(&component.id)`, verifies containment inside `<root>/.rocket/components/`, atomically writes `<id>.toml` and `<id>.md` (`component.markdown_body`).
   - `pub fn delete_component(root: &std::path::Path, id: &str) -> RfResult<()>`: validates id and containment, removes `<id>.toml` (returning `RfError::NotFound` if missing) and removes `<id>.md` if present.
   - `pub fn load_project(root: &std::path::Path) -> RfResult<Project>`: reads `<root>/.rocket/project.toml` (returning `RfError::NotFound` if missing), scans `<root>/.rocket/components/*.toml` sorted by filename, parses each `Component`, loads companion `<id>.md` if present (default empty string), and returns `Project`.
3. Tests in `src/storage.rs`:
   - `storage_init_save_load_roundtrip`: use `tempfile::tempdir()`; call `init_workspace(dir.path(), "demo-app", "Demo")`; save a component `"view_box"` with `markdown_body = "# View Box\nDetails\n".to_string()`; call `load_project(dir.path())`; assert `loaded.config.name == "demo-app"`, `loaded.components.len() == 1`, `loaded.components[0].id == "view_box"`, and `loaded.components[0].markdown_body == "# View Box\nDetails\n"`.
   - `storage_rejects_invalid_and_traversal_ids`: assert `validate_component_id("../escape").is_err()`, `validate_component_id("UpperCase").is_err()`, `validate_component_id("").is_err()`, and `validate_component_id("valid_id-1").is_ok()`.
   - `storage_delete_component_removes_both_files`: init workspace, save `"stream_1"`, call `delete_component(dir.path(), "stream_1")`, assert `load_project(dir.path()).unwrap().components.is_empty()` and assert deleting again returns `Err(RfError::NotFound(_))`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn load_project\b' src/storage.rs && grep -qE 'pub fn save_component\b' src/storage.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 1C: Planning-First Component Archetypes

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 1C: Implement planning-first component archetype templates in src/archetype.rs and expose them from src/lib.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/archetype.rs (NEW)
  - src/lib.rs
READ_ONLY_REFS:
  - src/model.rs:1-140 (Component, ComponentKind, FeatureItem, GridPos)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No filesystem I/O inside src/archetype.rs (pure data constructors only).

[PRE_GATE]
grep -qE 'pub enum ComponentKind\b' src/model.rs

[MUTATION_SPEC]
1. src/lib.rs — anchor `pub mod model;` → add `pub mod archetype;` beneath `pub mod model;` (preserving any other existing `pub mod` lines).
2. src/archetype.rs (NEW) — define archetype descriptor and scaffold builder:
   - `pub struct ArchetypeSpec { pub kind: ComponentKind, pub label: &'static str, pub summary: &'static str, pub default_tags: &'static [&'static str], pub starter_features: &'static [&'static str], pub markdown_template: &'static str }` (`Debug, Clone, Copy`).
   - `pub const ARCHETYPES: &[ArchetypeSpec]` containing exactly 6 entries covering every `ComponentKind` variant:
     1. `ComponentKind::UiView` (`label: "UI View"`, `default_tags: &["ui", "view"]`, `starter_features: &["Define layout and container bounds", "Wire state bindings and input handling"]`)
     2. `ComponentKind::DataStream` (`label: "Data Stream"`, `default_tags: &["data", "stream"]`, `starter_features: &["Define payload schema and framing", "Implement producer/consumer pipeline"]`)
     3. `ComponentKind::ServiceCore` (`label: "Service Core"`, `default_tags: &["core", "service"]`, `starter_features: &["Define domain API and error contracts", "Implement core business logic"]`)
     4. `ComponentKind::CliTool` (`label: "CLI Tool"`, `default_tags: &["cli"]`, `starter_features: &["Define argument parser and subcommands", "Implement exit codes and output formatting"]`)
     5. `ComponentKind::Storage` (`label: "Storage"`, `default_tags: &["storage", "persistence"]`, `starter_features: &["Define persistence schema and migrations", "Implement atomic read/write layer"]`)
     6. `ComponentKind::Custom` (`label: "Custom"`, `default_tags: &["custom"]`, `starter_features: &["Define component responsibilities"]`)
   - `pub fn archetype_for(kind: ComponentKind) -> &'static ArchetypeSpec`: matches `kind` to its entry in `ARCHETYPES`.
   - `pub fn scaffold_component(id: &str, name: &str, kind: ComponentKind, position: GridPos) -> Component`: builds a `Component` with `id`, `name`, `kind`, `description` set to `spec.summary`, `position`, `tags` cloned from `spec.default_tags`, `features` generated as `FeatureItem { id: format!("{id}_f{}", idx + 1), title: (*title).to_string(), pushed: false, external_id: None }`, `external_tasks: Vec::new()`, and `markdown_body` formatted as `format!("# {name}\n\n{}", spec.markdown_template)`.
3. Tests in `src/archetype.rs`:
   - `archetype_covers_all_kinds`: iterate over `[ComponentKind::UiView, ComponentKind::DataStream, ComponentKind::ServiceCore, ComponentKind::CliTool, ComponentKind::Storage, ComponentKind::Custom]`; assert `archetype_for(kind).kind == kind` and `!archetype_for(kind).starter_features.is_empty()`.
   - `scaffold_ui_view_populates_features_and_markdown`: call `scaffold_component("main_view", "Main View", ComponentKind::UiView, GridPos { col: 2, row: 1 })`; assert `comp.id == "main_view"`, `comp.features.len() == 2`, `comp.features[0].id == "main_view_f1"`, `comp.markdown_body.starts_with("# Main View\n\n")`, and `comp.tags.contains(&"ui".to_string())`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn scaffold_component\b' src/archetype.rs && grep -qE 'pub const ARCHETYPES\b' src/archetype.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 1D: Coverage Audit and Card Status Engine

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 1D: Implement card status computation and overlooked-component coverage auditing in src/audit.rs and expose it from src/lib.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/audit.rs (NEW)
  - src/lib.rs
READ_ONLY_REFS:
  - src/model.rs:1-140 (Project, Component, CardStatus, TaskStatus, Connection, FeatureItem, ExternalTaskRef)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub enum CardStatus\b' src/model.rs

[MUTATION_SPEC]
1. src/lib.rs — anchor `pub mod model;` → add `pub mod audit;` beneath `pub mod model;` (preserving any other existing `pub mod` lines).
2. src/audit.rs (NEW) — define status computation, audit finding types, and project audit engine:
   - `pub enum FindingSeverity { Warning, Error }` (`Debug, Clone, Copy, PartialEq, Eq`).
   - `pub enum FindingKind { UntrackedComponent, UnpushedFeatures, DanglingConnection }` (`Debug, Clone, Copy, PartialEq, Eq`).
   - `pub struct AuditFinding { pub severity: FindingSeverity, pub kind: FindingKind, pub component_id: Option<String>, pub message: String }` (`Debug, Clone, PartialEq, Eq`).
   - `pub struct TaskProgress { pub complete: usize, pub total: usize }` (`Debug, Clone, Copy, PartialEq, Eq`).
   - `pub fn compute_card_status(component: &Component) -> CardStatus`:
     - If `component.external_tasks.is_empty()` and `component.features.is_empty()` → `CardStatus::Gap`.
     - If `component.external_tasks.is_empty()` and `!component.features.is_empty()` → `CardStatus::Planned`.
     - If any task in `component.external_tasks` has `TaskStatus::Blocked` → `CardStatus::Blocked`.
     - If all tasks in `component.external_tasks` have `TaskStatus::Complete` → `CardStatus::Complete`.
     - Otherwise → `CardStatus::InProgress`.
   - `pub fn task_progress(component: &Component) -> TaskProgress`: counts `complete` (`status == TaskStatus::Complete`) and `total` (`external_tasks.len()`).
   - `pub fn audit_project(project: &Project) -> Vec<AuditFinding>`:
     - Collects valid component IDs into a `HashSet<&str>`.
     - For each component:
       - If `component.external_tasks.is_empty() && component.features.is_empty()`: push `AuditFinding` with `FindingSeverity::Error`, `FindingKind::UntrackedComponent`, `component_id: Some(component.id.clone())`, `message: format!("Component '{}' has no features or linked tasks", component.id)`.
       - Else if `component.external_tasks.is_empty()` and any feature has `!feature.pushed`: push `AuditFinding` with `FindingSeverity::Warning`, `FindingKind::UnpushedFeatures`, `component_id: Some(component.id.clone())`, `message: format!("Component '{}' has unpushed local features and no linked tasks", component.id)`.
       - Else if any feature has `!feature.pushed`: push `AuditFinding` with `FindingSeverity::Warning`, `FindingKind::UnpushedFeatures`, `component_id: Some(component.id.clone())`, `message: format!("Component '{}' has {} unpushed local feature(s)", component.id, unpushed_count)`.
     - For each connection in `project.config.connections`:
       - If `!ids.contains(conn.from.as_str()) || !ids.contains(conn.to.as_str())`: push `AuditFinding` with `FindingSeverity::Error`, `FindingKind::DanglingConnection`, `component_id: None`, `message: format!("Connection '{} -> {}' references missing component", conn.from, conn.to)`.
3. Tests in `src/audit.rs`:
   - `card_status_transitions`: verify all 5 `CardStatus` outcomes: empty component -> `Gap`; local feature only -> `Planned`; mixed `Pending`/`Complete` external tasks -> `InProgress`; any `Blocked` task -> `Blocked`; all `Complete` external tasks -> `Complete`.
   - `audit_flags_untracked_view_box_and_dangling_edge`: build a `Project` with `"data_stream"` (1 `Complete` external task) and `"view_box"` (0 features, 0 external tasks), plus a connection `"data_stream" -> "ghost_node"`; call `audit_project(&project)`; assert findings contain an `UntrackedComponent` for `"view_box"` and a `DanglingConnection` for `"ghost_node"`.
   - `task_progress_counts_completed_tasks`: build component with 2 `Complete` and 1 `InProgress` external tasks; assert `task_progress(&comp) == TaskProgress { complete: 2, total: 3 }`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn compute_card_status\b' src/audit.rs && grep -qE 'pub fn audit_project\b' src/audit.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 1E: Tool-Agnostic Canonical Exporters (TOML Manifest, Roadmap MD, JSON)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 1E: Implement tool-agnostic TOML manifest, Markdown roadmap table, and canonical JSON exporters in src/export.rs and expose them from src/lib.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/export.rs (NEW)
  - src/lib.rs
READ_ONLY_REFS:
  - src/error.rs:1-40 (RfError, RfResult)
  - src/model.rs:1-140 (Project, Component, ComponentKind, Connection, FeatureItem)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub struct Project\b' src/model.rs

[MUTATION_SPEC]
1. src/lib.rs — anchor `pub mod model;` → add `pub mod export;` beneath `pub mod model;` (preserving any other existing `pub mod` lines).
2. src/export.rs (NEW) — implement three canonical exporters:
   - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)] pub struct ExportDependency { pub target: String, pub kind: EdgeKind, pub label: Option<String> }`
   - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)] pub struct ComponentManifestExport { pub id: String, pub name: String, pub component_type: String, pub relative_path: Option<String>, pub description: String, pub tags: Vec<String>, pub tech_stack: Vec<String>, pub files: Vec<String>, pub flags: Vec<String>, pub dependencies: Vec<ExportDependency> }`
   - `pub fn map_component_type(kind: ComponentKind) -> &'static str`: maps `UiView => "ui_crate"`, `DataStream => "rust_crate"`, `ServiceCore => "rust_crate"`, `CliTool => "cli_crate"`, `Storage => "rust_crate"`, `Custom => "custom"`.
   - `pub fn export_component_manifest_toml(project: &Project, component_id: &str) -> RfResult<String>`: looks up `component_id` (or `Err(RfError::NotFound(...))`), gathers outgoing `project.config.connections` where `conn.from == component_id` into `dependencies`, builds `ComponentManifestExport`, and serializes with `toml::to_string_pretty`.
   - `pub fn export_roadmap_markdown(project: &Project) -> String`: generates a Markdown document with `# <project.config.name> — Blueprint Roadmap`, an `## Up Next` table with columns `| # | Source Ref | Component | Feature | Status |` listing every component's features (1-based index, `Source Ref` = `FEAT <component.id>:<feature.id>`, `Status` = `"pushed"` if `feature.pushed` else `"planned"`), and a `## Components Without Tasks` section listing any component with `features.is_empty() && external_tasks.is_empty()`.
   - `pub fn export_project_json(project: &Project) -> RfResult<String>`: serializes `project` with `serde_json::to_string_pretty`.
3. Tests in `src/export.rs`:
   - `export_manifest_toml_includes_dependencies_and_type`: build a `Project` with `"ui_box"` (`ComponentKind::UiView`), `"core_bus"` (`ComponentKind::DataStream`), and connection `"ui_box" -> "core_bus"` (`EdgeKind::DataFlow`); call `export_component_manifest_toml(&project, "ui_box").unwrap()`; assert output contains `component_type = "ui_crate"`, `id = "ui_box"`, and `target = "core_bus"`.
   - `export_roadmap_markdown_lists_features_and_gaps`: build a `Project` with `"core_bus"` (1 unpushed feature `"f1": "Stream events"`) and `"view_box"` (0 features, 0 external tasks); call `export_roadmap_markdown(&project)`; assert output contains `FEAT core_bus:f1`, `Stream events`, and `view_box` under `## Components Without Tasks`.
   - `export_project_json_valid_json`: call `export_project_json(&project).unwrap()`; parse with `serde_json::from_str::<serde_json::Value>` and assert `val["config"]["name"] == "demo"`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn export_component_manifest_toml\b' src/export.rs && grep -qE 'pub fn export_roadmap_markdown\b' src/export.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 1F: RON Theme System and Default CRT Amber Palette

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 1F: Implement RON-based theme loading and bundled CRT amber-on-black default palette in src/theme.rs and expose it from src/lib.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/theme.rs (NEW)
  - src/lib.rs
READ_ONLY_REFS:
  - src/error.rs:1-40 (RfError, RfResult)
  - Rocket Factory.md:20-25 (aesthetics and CRT amber theme requirements)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No ratatui dependency in src/theme.rs (use plain RGB tuples/structs so core stays UI-library-agnostic).
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'ron =' Cargo.toml

[MUTATION_SPEC]
1. src/lib.rs — anchor `pub mod model;` → add `pub mod theme;` beneath `pub mod model;` (preserving any other existing `pub mod` lines).
2. src/theme.rs (NEW) — define RGB color and RON theme types + loader:
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub struct RgbColor { pub r: u8, pub g: u8, pub b: u8 }`
   - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub struct ThemePalette`:
     - `pub background: RgbColor`
     - `pub surface: RgbColor`
     - `pub foreground: RgbColor`
     - `pub muted: RgbColor`
     - `pub accent: RgbColor`
     - `pub border_default: RgbColor`
     - `pub border_focused: RgbColor`
     - `pub status_gap: RgbColor`
     - `pub status_planned: RgbColor`
     - `pub status_in_progress: RgbColor`
     - `pub status_blocked: RgbColor`
     - `pub status_complete: RgbColor`
     - `pub md_heading: RgbColor`
     - `pub md_code: RgbColor`
     - `pub alert_warn: RgbColor`
     - `pub alert_error: RgbColor`
   - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub struct Theme { pub name: String, pub palette: ThemePalette }`
   - `pub const DEFAULT_CRT_AMBER_RON: &str`: embedded RON string for `"crt_amber"` with pure CRT black background `(10, 10, 8)`, phosphor amber foreground `(255, 176, 0)`, bright amber focus `(255, 209, 102)`, phosphor green complete border `(64, 224, 112)`, alert red blocked/error `(255, 82, 82)`, cyan/teal code highlight `(94, 212, 212)`, and differentiated amber/gold heading & warning tones.
   - `pub fn default_theme() -> RfResult<Theme>`: parses `DEFAULT_CRT_AMBER_RON` via `ron::from_str`.
   - `pub fn load_theme_file(path: &std::path::Path) -> RfResult<Theme>`: reads file and parses RON, returning `RfError::Io` or `RfError::RonParse`.
   - `pub fn resolve_theme(theme_name: &str, search_dirs: &[std::path::PathBuf]) -> RfResult<Theme>`: if `<dir>/<theme_name>.ron` exists in any `search_dirs` (checked in order), loads it via `load_theme_file`; if `theme_name == "crt_amber"` and no override file exists, returns `default_theme()`; otherwise returns `Err(RfError::NotFound(format!("theme '{theme_name}'")))`.
3. Tests in `src/theme.rs`:
   - `default_crt_amber_theme_parses_with_green_complete_border`: call `let t = default_theme().unwrap();`; assert `t.name == "crt_amber"`, `t.palette.background == RgbColor { r: 10, g: 10, b: 8 }`, `t.palette.foreground == RgbColor { r: 255, g: 176, b: 0 }`, and `t.palette.status_complete.g > t.palette.status_complete.r`.
   - `resolve_theme_loads_override_file`: create `tempfile::tempdir()`, write a custom `ocean.ron` theme file, call `resolve_theme("ocean", &[dir.path().to_path_buf()]).unwrap()`, assert `theme.name == "ocean"`; also assert `resolve_theme("nonexistent", &[dir.path().to_path_buf()]).is_err()`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn resolve_theme\b' src/theme.rs && grep -qE 'pub const DEFAULT_CRT_AMBER_RON\b' src/theme.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```


---

## Phase 2 Step Blocks

### STEP 2A: Adapter Configuration Schema, Loader, and Status Reconciliation

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 2A: Add RfError::Adapter and implement adapter configuration loading, active adapter resolution, and pulled task status reconciliation in src/adapter.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/adapter.rs (NEW)
  - src/error.rs
  - src/lib.rs
READ_ONLY_REFS:
  - src/error.rs:35-45 (RfError, RfResult)
  - src/model.rs:48-90 (ExternalTaskRef, FeatureItem, Component, TaskStatus, Project)
  - src/lib.rs:1-10 (module declarations)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No subprocess execution in src/adapter.rs yet (belongs to STEP 2B).
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub struct ExternalTaskRef\b' src/model.rs && grep -qE 'pub enum RfError\b' src/error.rs

[MUTATION_SPEC]
1. src/error.rs:40 — anchor `AlreadyExists(String),` → add variant directly below it:
   - `#[error("adapter error: {0}")]`
   - `Adapter(String),`
2. src/lib.rs:1 — anchor `pub mod archetype;` → add `pub mod adapter;` directly above `pub mod archetype;`.
3. src/adapter.rs (NEW) — define adapter config, wire payload types, loader, and reconciler:
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)] #[serde(rename_all = "snake_case")] pub enum ExportFormat { TomlManifest, MarkdownRoadmap, #[default] Json }`
   - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub struct CommandSpec { pub program: String, #[serde(default)] pub args: Vec<String> }`
   - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub struct AdapterConfig { pub name: String, #[serde(default)] pub export_format: ExportFormat, pub push_component_cmd: Option<CommandSpec>, pub push_feature_cmd: Option<CommandSpec>, pub pull_status_cmd: Option<CommandSpec> }`
   - `#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)] pub struct AdaptersFile { #[serde(default)] pub adapters: Vec<AdapterConfig> }`
   - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub struct PulledComponentStatus { pub component_id: String, #[serde(default)] pub tasks: Vec<ExternalTaskRef> }`
   - `#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)] pub struct PullStatusResponse { #[serde(default)] pub components: Vec<PulledComponentStatus> }`
   - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub struct PushFeaturePayload { pub component_id: String, pub feature: FeatureItem }`
   - `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub struct PushResponse { pub external_id: Option<String>, pub message: Option<String> }`
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub struct ReconcileSummary { pub components_updated: usize, pub tasks_synced: usize, pub features_linked: usize }`
   - `pub fn load_adapters_file(path: &std::path::Path) -> RfResult<AdaptersFile>`: reads `path` (if `ErrorKind::NotFound`, returns `Ok(AdaptersFile::default())`) and parses TOML into `AdaptersFile`.
   - `pub fn resolve_active_adapters<'a>(file: &'a AdaptersFile, active_names: &[String]) -> RfResult<Vec<&'a AdapterConfig>>`: for each name in `active_names`, finds matching `AdapterConfig` by `name` in `file.adapters` or returns `Err(RfError::NotFound(format!("adapter '{name}'")))`.
   - `pub fn reconcile_status(project: &mut Project, response: &PullStatusResponse) -> ReconcileSummary`: for each `PulledComponentStatus` in `response.components`, looks up `project.component_mut(&entry.component_id)`. If found: replaces `component.external_tasks = entry.tasks.clone()`; increments `components_updated` by 1 and adds `entry.tasks.len()` to `tasks_synced`; for each `feature` in `&mut component.features`, if `feature.external_id` matches any task's `external_id` in `entry.tasks` (or if `feature.external_id.is_none()` and a task's `title == feature.title`), sets `feature.pushed = true`, sets `feature.external_id = Some(task.external_id.clone())`, and increments `features_linked`.
4. Tests in `src/adapter.rs`:
   - `load_adapters_file_missing_returns_default`: call `load_adapters_file(dir.path().join("missing.toml").as_path()).unwrap()`; assert `file.adapters.is_empty()`.
   - `load_and_resolve_active_adapters`: write an `adapters.toml` with 2 adapters (`"tracker_a"`, `"tracker_b"`); assert `resolve_active_adapters(&file, &["tracker_b".to_string()]).unwrap()` returns length 1 with `name == "tracker_b"`, and resolving `"unknown"` returns `Err(RfError::NotFound(_))`.
   - `reconcile_status_updates_tasks_and_links_features`: build a `Project` with component `"view_box"` having 1 unpushed feature (`id: "f1"`, `title: "Render card"`, `pushed: false`, `external_id: None`); reconcile a `PullStatusResponse` with `component_id: "view_box"` and 1 task (`external_id: "EXT-1"`, `title: "Render card"`, `status: TaskStatus::Complete`); assert `summary == ReconcileSummary { components_updated: 1, tasks_synced: 1, features_linked: 1 }`, `project.component("view_box").unwrap().external_tasks.len() == 1`, and `project.component("view_box").unwrap().features[0].pushed`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn reconcile_status\b' src/adapter.rs && grep -qE 'pub fn load_adapters_file\b' src/adapter.rs && grep -qE 'Adapter\(String\)' src/error.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 2B: Subprocess Adapter Runner (Push Component, Push Feature, Pull Status)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 2B: Implement subprocess execution for push_component, push_feature, and pull_status in src/adapter.rs with canonical payload piping and mock-command unit tests.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/adapter.rs
READ_ONLY_REFS:
  - src/export.rs:40-115 (export_component_manifest_toml, export_roadmap_markdown, export_project_json)
  - src/error.rs:1-50 (RfError::Adapter, RfResult)
  - src/model.rs:48-130 (Project, Component, FeatureItem)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub fn reconcile_status\b' src/adapter.rs && grep -qE 'pub fn export_component_manifest_toml\b' src/export.rs

[MUTATION_SPEC]
1. src/adapter.rs — add subprocess helper and three public runner functions:
   - `fn run_command_with_stdin(spec: &CommandSpec, workdir: &std::path::Path, stdin_payload: &str) -> RfResult<String>`: spawns `std::process::Command::new(&spec.program)` with `.args(&spec.args)`, `.current_dir(workdir)`, `.stdin(Stdio::piped())`, `.stdout(Stdio::piped())`, `.stderr(Stdio::piped())`. Writes `stdin_payload.as_bytes()` to child stdin (dropping stdin handle before waiting), calls `wait_with_output()`. If `!output.status.success()`, returns `Err(RfError::Adapter(format!("command '{}' exited with {}: {}", spec.program, output.status, String::from_utf8_lossy(&output.stderr).trim())))`. Otherwise returns `Ok(String::from_utf8_lossy(&output.stdout).into_owned())`.
   - `pub fn run_push_component(adapter: &AdapterConfig, workdir: &std::path::Path, project: &Project, component_id: &str) -> RfResult<PushResponse>`: requires `adapter.push_component_cmd` (else `Err(RfError::Adapter(format!("adapter '{}' has no push_component_cmd", adapter.name)))`); formats payload according to `adapter.export_format`: `ExportFormat::TomlManifest` → `export_component_manifest_toml(project, component_id)?`, `ExportFormat::MarkdownRoadmap` → `export_roadmap_markdown(project)`, `ExportFormat::Json` → `serde_json::to_string_pretty(project.component(component_id).ok_or_else(|| RfError::NotFound(component_id.to_string()))?)?`. Invokes `run_command_with_stdin`. If trimmed stdout is empty, returns `Ok(PushResponse { external_id: None, message: None })`; else parses stdout as JSON `PushResponse` (mapping parse failure to `RfError::Adapter`).
   - `pub fn run_push_feature(adapter: &AdapterConfig, workdir: &std::path::Path, component_id: &str, feature: &FeatureItem) -> RfResult<PushResponse>`: requires `adapter.push_feature_cmd` (else `Err(RfError::Adapter(...))`); serializes `PushFeaturePayload { component_id: component_id.to_string(), feature: feature.clone() }` as JSON; invokes `run_command_with_stdin`; returns `PushResponse` (empty stdout -> `None` fields, non-empty -> parsed JSON).
   - `pub fn run_pull_status(adapter: &AdapterConfig, workdir: &std::path::Path, project: &Project) -> RfResult<PullStatusResponse>`: requires `adapter.pull_status_cmd` (else `Err(RfError::Adapter(...))`); serializes `export_project_json(project)?` as stdin; invokes `run_command_with_stdin`; parses stdout JSON into `PullStatusResponse` (mapping parse error to `RfError::Adapter`).
2. Tests in `src/adapter.rs`:
   - `run_push_component_pipes_toml_manifest_and_parses_response`: configure an `AdapterConfig` with `ExportFormat::TomlManifest` and `push_component_cmd = Some(CommandSpec { program: "sh".into(), args: vec!["-c".into(), "cat >/dev/null && printf '{\"external_id\":\"CMP-1\",\"message\":\"ok\"}'".into()] })`; invoke `run_push_component`; assert `resp.external_id.as_deref() == Some("CMP-1")`.
   - `run_pull_status_parses_json_tasks`: configure `pull_status_cmd` with `sh -c` outputting `{"components":[{"component_id":"view_box","tasks":[{"external_id":"T-1","title":"Draw box","status":"complete"}]}]}`; invoke `run_pull_status`; assert `resp.components.len() == 1` and `resp.components[0].tasks[0].status == TaskStatus::Complete`.
   - `run_command_nonzero_exit_returns_adapter_error`: configure `pull_status_cmd` with `sh -c "echo boom >&2; exit 2"`; assert `matches!(run_pull_status(&adapter, dir.path(), &project), Err(RfError::Adapter(msg)) if msg.contains("boom"))`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn run_push_component\b' src/adapter.rs && grep -qE 'pub fn run_pull_status\b' src/adapter.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 2C: Headless CLI Command Module (`rf init | audit | export | push | pull`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 2C: Add clap dependency and implement headless CLI command definitions and execution in src/cli.rs, exposed from src/lib.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - Cargo.toml
  - Cargo.lock
  - src/cli.rs (NEW)
  - src/lib.rs
READ_ONLY_REFS:
  - Cargo.toml:1-20 (dependencies)
  - src/storage.rs:105-265 (init_workspace, load_project, save_component)
  - src/audit.rs:70-135 (audit_project, FindingSeverity)
  - src/export.rs:40-115 (export_component_manifest_toml, export_roadmap_markdown, export_project_json)
  - src/adapter.rs:1-200 (load_adapters_file, resolve_active_adapters, run_push_component, run_push_feature, run_pull_status, reconcile_status)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No TUI crates (ratatui, crossterm) in Cargo.toml yet.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/ or Cargo.toml.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub fn run_push_component\b' src/adapter.rs && grep -qE 'pub fn run_pull_status\b' src/adapter.rs

[MUTATION_SPEC]
1. Cargo.toml:16 — anchor `thiserror = "2"` → add `clap = { version = "4", features = ["derive"] }` directly beneath it.
2. src/lib.rs:2 — anchor `pub mod archetype;` → add `pub mod cli;` beneath `pub mod audit;` (preserving alphabetical module order).
3. src/cli.rs (NEW) — implement clap structs and `run_cli`:
   - `#[derive(Debug, clap::Parser)] #[command(name = "rf", about = "Rocket Factory — retro-futuristic architecture planning TUI & CLI")] pub struct Cli { #[arg(long, global = true, default_value = ".")] pub path: std::path::PathBuf, #[arg(long, global = true)] pub adapters_file: Option<std::path::PathBuf>, #[command(subcommand)] pub command: Option<Command> }`
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)] pub enum ExportKind { Manifest, Roadmap, Json }`
   - `#[derive(Debug, clap::Subcommand)] pub enum Command`:
     - `Init { #[arg(long)] name: String, #[arg(long, default_value = "")] description: String }`
     - `Audit { #[arg(long)] strict: bool }`
     - `Export { #[arg(long, value_enum)] format: ExportKind, #[arg(long)] component: Option<String> }`
     - `Push { #[arg(long)] component: Option<String>, #[arg(long)] adapter: Option<String> }`
     - `Pull { #[arg(long)] adapter: Option<String> }`
   - `pub fn default_adapters_path() -> std::path::PathBuf`: returns `$HOME/.config/rocket-factory/adapters.toml` if `HOME` env var is set, else `.rocket/adapters.toml`.
   - `pub struct CliOutput { pub stdout: String, pub exit_code: i32 }` (`Debug, Clone, PartialEq, Eq`).
   - `pub fn execute_command(root: &std::path::Path, adapters_path: &std::path::Path, command: Command) -> RfResult<CliOutput>`:
     - `Command::Init { name, description }`: calls `storage::init_workspace(root, &name, &description)?` and returns `Ok(CliOutput { stdout: format!("Initialized Rocket Factory workspace '{name}' in {}\n", root.display()), exit_code: 0 })`.
     - `Command::Audit { strict }`: loads project, runs `audit::audit_project(&project)`. Formats each finding as `"[ERROR] <msg>\n"` or `"[WARN] <msg>\n"` (or `"No audit findings.\n"` if empty). Sets `exit_code = 1` when `strict` is true and at least one finding has `FindingSeverity::Error`, otherwise `0`; returns `Ok(CliOutput { stdout, exit_code })`.
     - `Command::Export { format, component }`: loads project; `ExportKind::Manifest` requires `component` (else `Err(RfError::NotFound("export --format manifest requires --component <id>".into()))`) and calls `export::export_component_manifest_toml(&project, &id)?`; `ExportKind::Roadmap` calls `export::export_roadmap_markdown(&project)`; `ExportKind::Json` calls `export::export_project_json(&project)?`; returns `Ok(CliOutput { stdout, exit_code: 0 })`.
     - `Command::Push { component, adapter }`: loads project and adapters file; selects adapters (either `[adapter_name]` if `--adapter` passed, else `&project.config.active_adapters`); resolves active adapters; iterates target components (single `--component` or all components); runs `run_push_component` when `push_component_cmd.is_some()` and for each unpushed feature runs `run_push_feature` when `push_feature_cmd.is_some()`, marking pushed features and saving modified components via `storage::save_component`; returns summary in `CliOutput { stdout, exit_code: 0 }`.
     - `Command::Pull { adapter }`: loads project and adapters file; resolves target adapters; for each adapter with `pull_status_cmd.is_some()`, runs `run_pull_status`, calls `reconcile_status(&mut project, &resp)`, saves updated components via `storage::save_component`, and returns summary in `CliOutput { stdout, exit_code: 0 }`.
4. Tests in `src/cli.rs`:
   - `cli_init_and_export_roadmap`: use `tempfile::tempdir()`; run `execute_command(dir.path(), &adapters, Command::Init { name: "apollo".into(), description: "Plan".into() })`; save a scaffolded `UiView` component `"hud"`; run `Command::Export { format: ExportKind::Roadmap, component: None }`; assert `out.exit_code == 0` and `out.stdout.contains("# apollo — Blueprint Roadmap")` and `out.stdout.contains("hud_f1")`.
   - `cli_audit_strict_exits_nonzero_on_gap`: init workspace, save an empty component `"view_box"` (0 features, 0 tasks); run `Command::Audit { strict: true }`; assert `out.exit_code == 1` and `out.stdout.contains("view_box")`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn execute_command\b' src/cli.rs && grep -qE 'pub struct Cli\b' src/cli.rs && ! grep -rqi 'nebulous' src/ Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 2D: `rf` Binary Entrypoint

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 2D: Register the `rf` binary in Cargo.toml and implement src/main.rs dispatching CLI subcommands and default launch behavior.

[SCOPE_LOCK]
ALLOWED_FILES:
  - Cargo.toml
  - src/main.rs (NEW)
READ_ONLY_REFS:
  - Cargo.toml:1-22 (package and lib sections)
  - src/cli.rs:1-80 (Cli, Command, CliOutput, default_adapters_path, execute_command)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/ or Cargo.toml.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub fn execute_command\b' src/cli.rs && grep -qE 'pub struct Cli\b' src/cli.rs

[MUTATION_SPEC]
1. Cargo.toml:9 — anchor `path = "src/lib.rs"` → add `[[bin]]` section beneath `[lib]`:
   - `[[bin]]`
   - `name = "rf"`
   - `path = "src/main.rs"`
2. src/main.rs (NEW) — implement entrypoint:
   - Parse `let cli = rocket_factory::cli::Cli::parse();` (importing `clap::Parser`).
   - Compute `let adapters_path = cli.adapters_file.unwrap_or_else(rocket_factory::cli::default_adapters_path);`.
   - Match `cli.command`:
     - `Some(command)` → call `rocket_factory::cli::execute_command(&cli.path, &adapters_path, command)`:
       - `Ok(output)` → `print!("{}", output.stdout);` and if `output.exit_code != 0`, return `std::process::ExitCode::from(output.exit_code as u8)`.
       - `Err(err)` → `eprintln!("error: {err}");` and return `std::process::ExitCode::FAILURE`.
     - `None` → `println!("Rocket Factory TUI — launching in Phase 3 (use `rf --help` for CLI commands)");` and return `std::process::ExitCode::SUCCESS`.
   - Return `std::process::ExitCode` from `fn main() -> std::process::ExitCode`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo run --quiet --bin rf -- --help | grep -qE 'Rocket Factory' && ! grep -rqi 'nebulous' src/ Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```


---

## Phase 3 Step Blocks

### STEP 3A: Ratatui Dependencies and TUI Application State Machine

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 3A: Add ratatui and crossterm to Cargo.toml and implement the TUI App state machine and keyboard dispatcher in src/tui/app.rs and src/tui/mod.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - Cargo.toml
  - Cargo.lock
  - src/lib.rs
  - src/tui/mod.rs (NEW)
  - src/tui/app.rs (NEW)
READ_ONLY_REFS:
  - Cargo.toml:1-25 (dependencies)
  - src/lib.rs:1-12 (module declarations)
  - src/model.rs:1-140 (Project, Component, CardStatus)
  - src/audit.rs:1-85 (AuditFinding, compute_card_status, audit_project)
  - src/theme.rs:1-45 (Theme)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No rendering widgets or terminal raw-mode I/O in src/tui/app.rs (belongs to STEP 3C and STEP 3D).
  - No references to the string "nebulous" (case-insensitive) anywhere in src/ or Cargo.toml.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub fn audit_project\b' src/audit.rs && grep -qE 'pub struct Theme\b' src/theme.rs

[MUTATION_SPEC]
1. Cargo.toml:21 — anchor `clap = { version = "4", features = ["derive"] }` → add directly beneath it:
   - `ratatui = "0.29"`
   - `crossterm = "0.28"`
2. src/lib.rs:8 — anchor `pub mod theme;` → add `pub mod tui;` directly beneath it.
3. src/tui/mod.rs (NEW) — declare `pub mod app;` and re-export `pub use app::{App, FocusColumn, InputMode, ListFilter};`.
4. src/tui/app.rs (NEW) — define TUI state types and pure keyboard handler:
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum FocusColumn { List, Canvas, Detail }`
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum InputMode { Navigate, MoveCard, Connect, Modal }`
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ListFilter { All, GapsOnly }`
   - `#[derive(Debug, Clone)] pub struct App`:
     - `pub root: std::path::PathBuf`
     - `pub project: crate::model::Project`
     - `pub theme: crate::theme::Theme`
     - `pub focus: FocusColumn`
     - `pub mode: InputMode`
     - `pub filter: ListFilter`
     - `pub selected_index: usize`
     - `pub detail_scroll: u16`
     - `pub status_message: String`
     - `pub should_quit: bool`
     - `pub audit_findings: Vec<crate::audit::AuditFinding>`
   - `impl App`:
     - `pub fn new(root: std::path::PathBuf, project: Project, theme: Theme) -> Self`: initializes `focus: FocusColumn::List`, `mode: InputMode::Navigate`, `filter: ListFilter::All`, `selected_index: 0`, `detail_scroll: 0`, `status_message: "Ready".to_string()`, `should_quit: false`, and computes `audit_findings = crate::audit::audit_project(&project)`.
     - `pub fn refresh_audit(&mut self)`: recomputes `self.audit_findings = crate::audit::audit_project(&self.project)`.
     - `pub fn gap_count(&self) -> usize`: returns count of `self.project.components` whose `crate::audit::compute_card_status(c) == crate::model::CardStatus::Gap`.
     - `pub fn visible_component_indices(&self) -> Vec<usize>`: if `self.filter == ListFilter::All`, returns `(0..self.project.components.len()).collect()`; if `ListFilter::GapsOnly`, returns indices where `compute_card_status(&self.project.components[i]) == CardStatus::Gap`.
     - `pub fn selected_component(&self) -> Option<&crate::model::Component>`: uses `visible_component_indices()` and returns the component at `indices.get(self.selected_index)`.
     - `pub fn clamp_selection(&mut self)`: clamps `self.selected_index` to `0` if `visible_component_indices()` is empty, else `self.selected_index.min(len - 1)`.
     - `pub fn select_next(&mut self)`: advances `selected_index` wrapping modulo `visible.len()` (no-op if empty) and resets `detail_scroll = 0`.
     - `pub fn select_prev(&mut self)`: decrements `selected_index` wrapping modulo `visible.len()` (no-op if empty) and resets `detail_scroll = 0`.
     - `pub fn cycle_focus_next(&mut self)`: `List -> Canvas -> Detail -> List`.
     - `pub fn cycle_focus_prev(&mut self)`: `List -> Detail -> Canvas -> List`.
     - `pub fn toggle_gaps_filter(&mut self)`: toggles `All <-> GapsOnly`, calls `clamp_selection()`, resets `detail_scroll = 0`, and updates `status_message` to `"Filter: Gaps Only"` or `"Filter: All Components"`.
     - `pub fn handle_key(&mut self, key: crossterm::event::KeyEvent)`:
       - `Ctrl+c` or `KeyCode::Char('q')` (when `mode != InputMode::Modal`) → sets `self.should_quit = true`.
       - `KeyCode::Esc` → sets `self.mode = InputMode::Navigate` and `self.status_message = "Mode: Navigate".to_string()`.
       - `KeyCode::Tab` → `self.cycle_focus_next()`.
       - `KeyCode::BackTab` → `self.cycle_focus_prev()`.
       - `KeyCode::Char('f')` → `self.toggle_gaps_filter()`.
       - `KeyCode::Char('m')` → sets `self.mode = InputMode::MoveCard` and `self.status_message = "Mode: Move Card".to_string()`.
       - `KeyCode::Char('c')` → sets `self.mode = InputMode::Connect` and `self.status_message = "Mode: Connect".to_string()`.
       - `KeyCode::Down | KeyCode::Char('j')` → if `self.focus == FocusColumn::Detail`, increments `self.detail_scroll = self.detail_scroll.saturating_add(1)`; else calls `self.select_next()`.
       - `KeyCode::Up | KeyCode::Char('k')` → if `self.focus == FocusColumn::Detail`, decrements `self.detail_scroll = self.detail_scroll.saturating_sub(1)`; else calls `self.select_prev()`.
5. Tests in `src/tui/app.rs`:
   - `app_cycles_focus_and_quits_on_q`: create `App` with 2 components; send `Tab` (assert `Canvas`), `Tab` (assert `Detail`), `BackTab` (assert `Canvas`), `Char('q')` (assert `app.should_quit`).
   - `app_gaps_filter_narrows_visible_components`: create `App` with `"tracked"` (1 `Complete` external task) and `"view_box"` (0 features, 0 tasks); assert `app.gap_count() == 1` and `app.visible_component_indices() == vec![0, 1]`; send `Char('f')`; assert `app.visible_component_indices() == vec![1]` and `app.selected_component().unwrap().id == "view_box"`.
   - `app_detail_focus_scrolls_instead_of_changing_selection`: set `app.focus = FocusColumn::Detail`, send `Char('j')` twice then `Char('k')` once; assert `app.detail_scroll == 1` and `app.selected_index == 0`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub struct App\b' src/tui/app.rs && grep -qE 'pub fn handle_key\b' src/tui/app.rs && ! grep -rqi 'nebulous' src/ Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 3B: Ratatui Theme Style Adapter

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 3B: Implement CRT amber Ratatui color and Markdown styling helpers in src/tui/style.rs and expose them from src/tui/mod.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/style.rs (NEW)
  - src/tui/mod.rs
READ_ONLY_REFS:
  - src/theme.rs:1-70 (RgbColor, ThemePalette, Theme, default_theme)
  - src/model.rs:40-50 (CardStatus)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub struct App\b' src/tui/app.rs && grep -qE 'ratatui =' Cargo.toml

[MUTATION_SPEC]
1. src/tui/mod.rs:1 — anchor `pub mod app;` → add `pub mod style;` directly beneath `pub mod app;`.
2. src/tui/style.rs (NEW) — implement pure Ratatui style mapping functions:
   - `pub fn to_color(c: crate::theme::RgbColor) -> ratatui::style::Color`: returns `Color::Rgb(c.r, c.g, c.b)`.
   - `pub fn base_style(theme: &Theme) -> Style`: `Style::default().fg(to_color(theme.palette.foreground)).bg(to_color(theme.palette.background))`.
   - `pub fn muted_style(theme: &Theme) -> Style`: `Style::default().fg(to_color(theme.palette.muted)).bg(to_color(theme.palette.background))`.
   - `pub fn header_style(theme: &Theme) -> Style`: `Style::default().fg(to_color(theme.palette.background)).bg(to_color(theme.palette.foreground)).add_modifier(Modifier::BOLD)`.
   - `pub fn panel_border_style(theme: &Theme, focused: bool) -> Style`: returns `Style::default().fg(to_color(if focused { theme.palette.border_focused } else { theme.palette.border_default }))`.
   - `pub fn selected_row_style(theme: &Theme) -> Style`: `Style::default().fg(to_color(theme.palette.background)).bg(to_color(theme.palette.accent)).add_modifier(Modifier::BOLD)`.
   - `pub fn card_border_color(theme: &Theme, status: CardStatus) -> Color`: maps `CardStatus::Gap => status_gap`, `CardStatus::Planned => status_planned`, `CardStatus::InProgress => status_in_progress`, `CardStatus::Blocked => status_blocked`, `CardStatus::Complete => status_complete` through `to_color`.
   - `pub fn status_glyph(status: CardStatus) -> &'static str`: maps `Gap => "[!]"`, `Planned => "[ ]"`, `InProgress => "[~]"`, `Blocked => "[X]"`, `Complete => "[✓]"`.
   - `pub fn markdown_line_style(theme: &Theme, line: &str) -> Style`:
     - Trimmed line starting with `"#"` → `Style::default().fg(to_color(theme.palette.md_heading)).add_modifier(Modifier::BOLD)`.
     - Trimmed line starting with `` "```" `` or `"    "` (4 spaces) → `Style::default().fg(to_color(theme.palette.md_code))`.
     - Trimmed line containing `"ERROR"` or `"BLOCKED"` → `Style::default().fg(to_color(theme.palette.alert_error)).add_modifier(Modifier::BOLD)`.
     - Trimmed line containing `"WARN"` or `"TODO"` → `Style::default().fg(to_color(theme.palette.alert_warn))`.
     - Otherwise → `Style::default().fg(to_color(theme.palette.foreground))`.
3. Tests in `src/tui/style.rs`:
   - `card_border_color_maps_complete_to_green_and_gap_to_gap_color`: load `default_theme().unwrap()`; assert `card_border_color(&theme, CardStatus::Complete) == to_color(theme.palette.status_complete)` and `card_border_color(&theme, CardStatus::Gap) == to_color(theme.palette.status_gap)`.
   - `markdown_line_style_differentiates_headings_code_and_alerts`: load `default_theme().unwrap()`; assert `markdown_line_style(&theme, "# Overview").fg == Some(to_color(theme.palette.md_heading))`, `markdown_line_style(&theme, "```rust").fg == Some(to_color(theme.palette.md_code))`, and `markdown_line_style(&theme, "BLOCKED: missing view").fg == Some(to_color(theme.palette.alert_error))`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn card_border_color\b' src/tui/style.rs && grep -qE 'pub fn markdown_line_style\b' src/tui/style.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 3C: Top-Down 3-Column TUI Layout, Header, List, and Detail Breakdown

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 3C: Implement the top-down 3-column Ratatui layout (header bar, Left Column component list, Middle Column canvas frame, Right Column detail breakdown, footer) in src/tui/ui.rs with TestBackend render tests.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/ui.rs (NEW)
  - src/tui/mod.rs
READ_ONLY_REFS:
  - src/tui/app.rs:1-140 (App, FocusColumn, InputMode, ListFilter)
  - src/tui/style.rs:1-85 (base_style, header_style, panel_border_style, selected_row_style, card_border_color, status_glyph, markdown_line_style)
  - src/audit.rs:30-75 (compute_card_status, task_progress)
  - src/model.rs:50-135 (Component, Connection, FeatureItem, ExternalTaskRef)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No snap-grid card or vector-line rendering yet (Middle Column renders a clean bordered placeholder frame stating grid dimensions and component count; full canvas arrives in Phase 4).
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub fn card_border_color\b' src/tui/style.rs && grep -qE 'pub fn markdown_line_style\b' src/tui/style.rs

[MUTATION_SPEC]
1. src/tui/mod.rs:2 — anchor `pub mod style;` → add `pub mod ui;` directly beneath `pub mod style;`.
2. src/tui/ui.rs (NEW) — implement top-down container tree and panel renderers:
   - `pub fn render(frame: &mut ratatui::Frame, app: &App)`:
     - Fills full `frame.area()` with `Block::default().style(style::base_style(&app.theme))`.
     - Root vertical split (`Layout::vertical([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])`):
       - `[0]` → `render_header(frame, chunks[0], app)`
       - `[1]` → Body horizontal 3-column split (`Layout::horizontal([Constraint::Percentage(24), Constraint::Percentage(48), Constraint::Percentage(28)])`):
         - `[0]` → `render_component_list(frame, cols[0], app)`
         - `[1]` → `render_canvas_placeholder(frame, cols[1], app)`
         - `[2]` → `render_detail_panel(frame, cols[2], app)`
       - `[2]` → `render_footer(frame, chunks[2], app)`
   - `fn render_header(frame: &mut Frame, area: Rect, app: &App)`: bordered block showing `" ROCKET FACTORY :: {project.config.name} | MODE: {mode:?} | GAPS: {app.gap_count()} | {app.status_message}"` with alert styling on the gap badge when `app.gap_count() > 0`.
   - `fn render_component_list(frame: &mut Frame, area: Rect, app: &App)`: bordered block titled `" Components "` (or `" Components [GAPS] "` when `app.filter == ListFilter::GapsOnly`), border styled with `panel_border_style(&app.theme, app.focus == FocusColumn::List)`. Renders each visible component as a `ListItem` showing `"{glyph} {comp.name} ({progress.complete}/{progress.total})"` colored by `card_border_color` (and highlighted with `selected_row_style` for `app.selected_index`). When empty, renders `" (no components) "`.
   - `fn render_canvas_placeholder(frame: &mut Frame, area: Rect, app: &App)`: bordered block titled `" Snap Grid ({cols}x{rows}) "`, border styled with `panel_border_style(&app.theme, app.focus == FocusColumn::Canvas)`. Displays centered summary lines: `"{count} staged component(s), {edges} connection(s)"` and `"Snap Grid Canvas — interactive cards & connectors in Phase 4"`.
   - `fn render_detail_panel(frame: &mut Frame, area: Rect, app: &App)`: bordered block titled `" Component Details "`, border styled with `panel_border_style(&app.theme, app.focus == FocusColumn::Detail)`. If `app.selected_component()` is `Some(comp)`, builds styled `Vec<Line>` containing:
     - `Name: {comp.name} ({comp.id})`
     - `Kind: {:?} | Status: {:?} | Grid: ({}, {})`
     - `Path: {relative_path or "-"}`
     - `Tags: {joined tags or "-"}`
     - `Description: {comp.description}`
     - Section `Connections:` listing outgoing (`-> to [kind]`) and incoming (`<- from [kind]`) edges from `app.project.config.connections`.
     - Section `Features:` listing `[pushed]` / `[local]` + `feature.title`.
     - Section `External Tasks:` listing `[status]` + `task.external_id` + `task.title`.
     - Section `Specification:` rendering each line of `comp.markdown_body` with `style::markdown_line_style(&app.theme, line)`.
     - Renders as `Paragraph::new(lines).wrap(Wrap { trim: false }).scroll((app.detail_scroll, 0))`.
     - If `None`, renders `"Select a component to inspect details."`.
   - `fn render_footer(frame: &mut Frame, area: Rect, app: &App)`: renders `" [Tab] Focus  [j/k ↑/↓] Navigate/Scroll  [f] Gaps Filter  [m] Move  [c] Connect  [q] Quit"` using `muted_style`.
3. Tests in `src/tui/ui.rs`:
   - `render_three_column_layout_displays_header_list_and_details`: build `App` with `"view_box"` (gap) and `"core_bus"` (1 `Complete` external task) using `default_theme().unwrap()`; draw onto `Terminal::new(TestBackend::new(120, 35))`; collect rendered buffer text; assert text contains `"ROCKET FACTORY"`, `"GAPS: 1"`, `"Components"`, `"Snap Grid"`, `"Component Details"`, and `"view_box"`.
   - `render_gaps_filter_hides_tracked_component`: call `app.toggle_gaps_filter()`; draw onto `TestBackend::new(120, 35)`; assert rendered text contains `"Components [GAPS]"` and `"view_box"`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn render\b' src/tui/ui.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 3D: Terminal Lifecycle Manager and `rf` TUI Launch Wiring

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 3D: Implement terminal lifecycle management (raw mode, alternate screen, panic hook, $EDITOR suspend/resume, event loop) in src/tui/terminal.rs and wire default `rf` launch in src/main.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/terminal.rs (NEW)
  - src/tui/mod.rs
  - src/main.rs
READ_ONLY_REFS:
  - src/tui/app.rs:1-140 (App, handle_key)
  - src/tui/ui.rs:1-30 (render)
  - src/storage.rs:199-265 (load_project)
  - src/theme.rs:65-95 (resolve_theme)
  - src/main.rs:1-35 (main)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - Do not alter any CLI subcommand handling (`Some(command)` arm) in src/main.rs.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub fn render\b' src/tui/ui.rs && grep -qE 'fn main\b' src/main.rs

[MUTATION_SPEC]
1. src/tui/mod.rs:3 — anchor `pub mod ui;` → add `pub mod terminal;` and `pub use terminal::{default_theme_dirs, run_tui, suspend_for_editor};`.
2. src/tui/terminal.rs (NEW) — implement terminal lifecycle and main TUI event loop:
   - `pub type TuiTerminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>;`
   - `pub fn default_theme_dirs(root: &std::path::Path) -> Vec<std::path::PathBuf>`: returns `[root.join(".rocket/themes")]`, plus `$HOME/.config/rocket-factory/themes` if `HOME` env var is non-empty.
   - `pub fn install_panic_hook()`: wraps `std::panic::take_hook()` so that on panic it calls `let _ = crossterm::terminal::disable_raw_mode();` and `let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);` before delegating to the original hook.
   - `pub fn enter_terminal() -> RfResult<TuiTerminal>`: enables raw mode, executes `EnterAlternateScreen` on stdout, constructs `Terminal::new(CrosstermBackend::new(std::io::stdout()))`, and maps any I/O error to `RfError::Io { path: PathBuf::from("<terminal>"), source }`.
   - `pub fn restore_terminal(terminal: &mut TuiTerminal) -> RfResult<()>`: disables raw mode, executes `LeaveAlternateScreen`, calls `terminal.show_cursor()`, mapping I/O errors to `RfError::Io`.
   - `pub fn suspend_for_editor(terminal: &mut TuiTerminal, file_path: &std::path::Path) -> RfResult<()>`: calls `restore_terminal(terminal)?`, reads `std::env::var("EDITOR")` (defaulting to `"vi"` if unset/empty), spawns and waits on `Command::new(&editor).arg(file_path)`, re-enters raw mode + `EnterAlternateScreen` + `terminal.clear()`, and returns an error if the editor exited non-zero (`RfError::Io` with `ErrorKind::Other`).
   - `pub fn run_tui(root: &std::path::Path, theme_dirs: &[std::path::PathBuf]) -> RfResult<()>`: loads project via `crate::storage::load_project(root)?`, resolves theme via `crate::theme::resolve_theme(&project.config.theme, theme_dirs)?`, constructs `App::new(root.to_path_buf(), project, theme)`, calls `install_panic_hook()`, enters terminal, runs loop (`terminal.draw(|f| ui::render(f, &app))`, polls `crossterm::event::poll(Duration::from_millis(100))`, reads `Event::Key(key)` with `key.kind == KeyEventKind::Press` and calls `app.handle_key(key)` until `app.should_quit`), always restores terminal before returning.
3. src/main.rs:28 — anchor `None => {` → replace the `None => { ... }` arm so it calls `let theme_dirs = rocket_factory::tui::default_theme_dirs(&cli.path);` and `rocket_factory::tui::run_tui(&cli.path, &theme_dirs)`, returning `ExitCode::SUCCESS` on `Ok(())` and printing `eprintln!("error: {err}");` + `ExitCode::FAILURE` on `Err(err)`.
4. Tests in `src/tui/terminal.rs`:
   - `default_theme_dirs_includes_project_local_dir`: call `default_theme_dirs(Path::new("/tmp/rf_demo"))`; assert `dirs[0] == PathBuf::from("/tmp/rf_demo/.rocket/themes")`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo run --quiet --bin rf -- --help | grep -qE 'Rocket Factory' && grep -qE 'pub fn run_tui\b' src/tui/terminal.rs && grep -qE 'pub fn suspend_for_editor\b' src/tui/terminal.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```


---

## Phase 4 Step Blocks

### STEP 4A: Snap Grid Geometry and Viewport Projection

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 4A: Implement cell-based snap grid constants, CanvasViewport, card rectangle projection, viewport panning, and card anchor geometry in src/tui/canvas.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/canvas.rs (NEW)
  - src/tui/mod.rs
READ_ONLY_REFS:
  - src/model.rs:1-10 (GridPos)
  - src/tui/mod.rs:1-8 (module exports)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No edge routing or widget rendering in src/tui/canvas.rs yet (belongs to STEP 4B).
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub mod ui;' src/tui/mod.rs && grep -qE 'pub struct GridPos\b' src/model.rs

[MUTATION_SPEC]
1. src/tui/mod.rs:1 — anchor `pub mod app;` → add `pub mod canvas;` directly beneath `pub mod app;`.
2. src/tui/canvas.rs (NEW) — define snap-grid layout constants, viewport, and projection helpers:
   - `pub const CARD_WIDTH: u16 = 22;`
   - `pub const CARD_HEIGHT: u16 = 5;`
   - `pub const GUTTER_X: u16 = 4;`
   - `pub const GUTTER_Y: u16 = 2;`
   - `pub const CELL_STRIDE_X: u16 = CARD_WIDTH + GUTTER_X;`
   - `pub const CELL_STRIDE_Y: u16 = CARD_HEIGHT + GUTTER_Y;`
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub struct CanvasViewport { pub scroll_col: u16, pub scroll_row: u16 }`
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct CardAnchors { pub top: (u16, u16), pub bottom: (u16, u16), pub left: (u16, u16), pub right: (u16, u16) }`
   - `pub fn card_rect(inner: ratatui::layout::Rect, viewport: CanvasViewport, pos: crate::model::GridPos) -> Option<ratatui::layout::Rect>`:
     - If `pos.col < viewport.scroll_col || pos.row < viewport.scroll_row`, return `None`.
     - Compute `rel_col = pos.col - viewport.scroll_col` and `rel_row = pos.row - viewport.scroll_row`.
     - Compute `x = inner.x.saturating_add(1).saturating_add(rel_col.saturating_mul(CELL_STRIDE_X))` and `y = inner.y.saturating_add(1).saturating_add(rel_row.saturating_mul(CELL_STRIDE_Y))`.
     - If `x.saturating_add(CARD_WIDTH) > inner.x.saturating_add(inner.width)` or `y.saturating_add(CARD_HEIGHT) > inner.y.saturating_add(inner.height)`, return `None`.
     - Return `Some(Rect { x, y, width: CARD_WIDTH, height: CARD_HEIGHT })`.
   - `pub fn visible_grid_capacity(inner: ratatui::layout::Rect) -> (u16, u16)`: computes `cols = inner.width.saturating_sub(1).saturating_add(GUTTER_X) / CELL_STRIDE_X` (at least 1) and `rows = inner.height.saturating_sub(1).saturating_add(GUTTER_Y) / CELL_STRIDE_Y` (at least 1).
   - `pub fn ensure_visible(inner: ratatui::layout::Rect, viewport: &mut CanvasViewport, pos: crate::model::GridPos)`:
     - Computes `(vis_cols, vis_rows) = visible_grid_capacity(inner)`.
     - If `pos.col < viewport.scroll_col`, sets `viewport.scroll_col = pos.col`; else if `pos.col >= viewport.scroll_col.saturating_add(vis_cols)`, sets `viewport.scroll_col = pos.col.saturating_sub(vis_cols.saturating_sub(1))`.
     - Same for `pos.row` / `viewport.scroll_row` / `vis_rows`.
   - `pub fn card_anchor_points(rect: ratatui::layout::Rect) -> CardAnchors`:
     - `mid_x = rect.x + rect.width / 2`, `mid_y = rect.y + rect.height / 2`
     - `top = (mid_x, rect.y)`, `bottom = (mid_x, rect.y + rect.height.saturating_sub(1))`, `left = (rect.x, mid_y)`, `right = (rect.x + rect.width.saturating_sub(1), mid_y)`.
3. Tests in `src/tui/canvas.rs`:
   - `card_rect_projects_grid_positions_and_culls_offscreen`: use `inner = Rect::new(0, 0, 60, 20)` and default `viewport`; assert `card_rect(inner, viewport, GridPos { col: 0, row: 0 }) == Some(Rect::new(1, 1, 22, 5))`, `card_rect(inner, viewport, GridPos { col: 1, row: 1 }) == Some(Rect::new(27, 8, 22, 5))`, and `card_rect(inner, viewport, GridPos { col: 5, row: 0 }).is_none()`.
   - `ensure_visible_pans_viewport_to_target`: use `inner = Rect::new(0, 0, 60, 20)` and default `viewport`; call `ensure_visible(inner, &mut viewport, GridPos { col: 4, row: 3 })`; assert `card_rect(inner, viewport, GridPos { col: 4, row: 3 }).is_some()`.
   - `card_anchor_points_centers_edges`: use `Rect::new(10, 4, 22, 5)`; assert `anchors.top == (21, 4)`, `anchors.bottom == (21, 8)`, `anchors.left == (10, 6)`, `anchors.right == (31, 6)`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub struct CanvasViewport\b' src/tui/canvas.rs && grep -qE 'pub fn card_rect\b' src/tui/canvas.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 4B: Orthogonal Unicode Connector Router and Snap Grid Card Renderer

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 4B: Implement orthogonal Unicode box-drawing connector routing and component card rendering in src/tui/canvas.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/canvas.rs
READ_ONLY_REFS:
  - src/tui/app.rs:1-120 (App, FocusColumn, selected_component, visible_component_indices)
  - src/tui/style.rs:1-85 (base_style, muted_style, panel_border_style, card_border_color, status_glyph)
  - src/audit.rs:30-75 (compute_card_status, task_progress)
  - src/model.rs:50-135 (Component, Connection, EdgeKind)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub struct CanvasViewport\b' src/tui/canvas.rs && grep -qE 'pub fn card_anchor_points\b' src/tui/canvas.rs

[MUTATION_SPEC]
1. src/tui/canvas.rs — add orthogonal edge router and canvas renderer:
   - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct RoutedEdge { pub segments: Vec<(u16, u16, char)>, pub label_pos: Option<(u16, u16)>, pub label: Option<String> }`
   - `pub fn route_orthogonal_edge(from_rect: Rect, to_rect: Rect, label: Option<&str>, bounds: Rect) -> RoutedEdge`:
     - Computes `from_a = card_anchor_points(from_rect)` and `to_a = card_anchor_points(to_rect)`.
     - Chooses horizontal-first routing when `to_rect.x >= from_rect.x + from_rect.width` (start at `(from_a.right.0 + 1, from_a.right.1)`, end at `(to_a.left.0.saturating_sub(1), to_a.left.1)` with arrow `'▶'`) or `from_rect.x >= to_rect.x + to_rect.width` (start at `(from_a.left.0.saturating_sub(1), from_a.left.1)`, end at `(to_a.right.0 + 1, to_a.right.1)` with arrow `'◀'`).
     - Otherwise chooses vertical routing: downward if `to_rect.y >= from_rect.y + from_rect.height` (start `(from_a.bottom.0, from_a.bottom.1 + 1)`, end `(to_a.top.0, to_a.top.1.saturating_sub(1))`, arrow `'▼'`), else upward (start `(from_a.top.0, from_a.top.1.saturating_sub(1))`, end `(to_a.bottom.0, to_a.bottom.1 + 1)`, arrow `'▲'`).
     - Emits orthogonal segments using `'─'`, `'│'`, and corner characters (`'┌'`, `'┐'`, `'└'`, `'┘'`) via a midpoint gutter bend when both `x` and `y` differ, placing the directional arrow at the final endpoint cell. Filters all `(x, y, ch)` to lie inside `bounds`.
     - If `label` is `Some(text)` and `!text.is_empty()` and `!segments.is_empty()`, sets `label_pos` to the midpoint segment coordinate `(mid.0, mid.1)` and `label = Some(text.to_string())`.
   - `fn truncate_text(s: &str, max_chars: usize) -> String`: truncates cleanly at character boundary with `'…'` when `s.chars().count() > max_chars` (and `max_chars >= 1`).
   - `pub fn render_canvas(frame: &mut ratatui::Frame, area: Rect, app: &crate::tui::app::App, viewport: CanvasViewport)`:
     - Renders outer bordered block titled `format!(" Snap Grid ({}x{}) ", app.project.config.grid.cols, app.project.config.grid.rows)` with `panel_border_style(&app.theme, app.focus == FocusColumn::Canvas)` and computes `inner = block.inner(area)`.
     - Collects `(comp_idx, Rect)` for all visible components (`app.visible_component_indices()`) via `card_rect(inner, viewport, comp.position)` into a map keyed by `comp.id.as_str()`.
     - Determines `selected_id = app.selected_component().map(|c| c.id.as_str())`.
     - Draws routed edges for each `conn` in `&app.project.config.connections` whose `from` and `to` rects are both visible: writes each `(x, y, ch)` directly into `frame.buffer_mut()[(x, y)]` using accent color if `selected_id == Some(conn.from.as_str()) || selected_id == Some(conn.to.as_str())`, else `muted_style`; writes optional `conn.label` (or short edge kind tag) clipped inside `inner`.
     - Draws each visible component card on top of the connectors:
       - `status = compute_card_status(comp)`, `progress = task_progress(comp)`, `glyph = style::status_glyph(status)`, `is_selected = selected_id == Some(comp.id.as_str())`.
       - Uses `BorderType::Double` when `is_selected`, `BorderType::Plain` otherwise; border color from `style::card_border_color(&app.theme, status)` (adding `Modifier::BOLD` when selected).
       - Renders 3 lines inside the card (`Paragraph` with `Block::bordered()`):
         1. `truncate_text(&format!("{glyph} {}", comp.name), inner_w)`
         2. `truncate_text(&format!("Tasks {}/{} [{:?}]", progress.complete, progress.total, comp.kind), inner_w)`
         3. `truncate_text(&comp.description, inner_w)`
2. Tests in `src/tui/canvas.rs`:
   - `route_horizontal_edge_emits_line_and_right_arrow`: route from `Rect::new(1, 1, 22, 5)` to `Rect::new(27, 1, 22, 5)` inside `Rect::new(0, 0, 80, 25)`; assert `edge.segments.iter().any(|&(_, _, ch)| ch == '─')` and `edge.segments.last().map(|&(_, _, ch)| ch) == Some('▶')`.
   - `route_l_bend_edge_emits_corners_and_arrow`: route from `Rect::new(1, 1, 22, 5)` to `Rect::new(27, 8, 22, 5)` inside `Rect::new(0, 0, 80, 25)` with `Some("events")`; assert `edge.segments.iter().any(|&(_, _, ch)| ch == '│')`, `edge.segments.last().map(|&(_, _, ch)| ch) == Some('▶')`, and `edge.label.as_deref() == Some("events")`.
   - `route_vertical_edge_emits_down_arrow`: route from `Rect::new(1, 1, 22, 5)` to `Rect::new(1, 8, 22, 5)` inside `Rect::new(0, 0, 80, 25)`; assert `edge.segments.last().map(|&(_, _, ch)| ch) == Some('▼')`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn route_orthogonal_edge\b' src/tui/canvas.rs && grep -qE 'pub fn render_canvas\b' src/tui/canvas.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 4C: Wire Snap Grid Canvas into 3-Column TUI Layout

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 4C: Add CanvasViewport and connect_source_id to App and wire render_canvas into the Middle Column of src/tui/ui.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/app.rs
  - src/tui/ui.rs
READ_ONLY_REFS:
  - src/tui/canvas.rs:1-40 (CanvasViewport, render_canvas)
  - src/tui/app.rs:25-65 (App struct and App::new)
  - src/tui/ui.rs:1-40 (render, render_canvas_placeholder)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No keyboard MoveCard/Connect logic changes in src/tui/app.rs yet (belongs to STEP 4D).
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub fn render_canvas\b' src/tui/canvas.rs && grep -qE 'pub fn render\b' src/tui/ui.rs

[MUTATION_SPEC]
1. src/tui/app.rs:5 — add `use crate::tui::canvas::CanvasViewport;` and at anchor `pub audit_findings: Vec<AuditFinding>,` → add directly beneath it:
   - `pub viewport: CanvasViewport,`
   - `pub connect_source_id: Option<String>,`
   Initialize both in `App::new` (`viewport: CanvasViewport::default()`, `connect_source_id: None`).
2. src/tui/ui.rs:32 — anchor `render_canvas_placeholder(frame, cols[1], app);` → replace with `crate::tui::canvas::render_canvas(frame, cols[1], app, app.viewport);` and remove the now-unused `fn render_canvas_placeholder` function (and unused `Alignment` import if no longer used).
3. Tests in `src/tui/ui.rs`:
   - Add `render_canvas_draws_component_cards_and_connectors`: build `make_test_app()` with a `Connection { from: "view_box".into(), to: "core_bus".into(), kind: EdgeKind::DataFlow, label: Some("events".into()) }`; draw onto `TestBackend::new(140, 35)`; assert rendered text contains `"view_box"`, `"core_bus"`, `"Tasks 1/1"`, and `'▶'`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub viewport: CanvasViewport' src/tui/app.rs && grep -qE 'render_canvas\(' src/tui/ui.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 4D: Keyboard-Only Canvas Controls (Directional Navigation, Move Card, Connect Mode)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 4D: Implement keyboard-only directional selection, MoveCard snap-grid placement with collision detection, and Connect mode edge toggling in src/tui/app.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/app.rs
READ_ONLY_REFS:
  - src/model.rs:1-85 (GridPos, Connection, EdgeKind, Project)
  - src/storage.rs:138-165 (save_project_config, save_component, ROCKET_DIR)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub viewport: CanvasViewport' src/tui/app.rs && grep -qE 'pub connect_source_id: Option<String>' src/tui/app.rs

[MUTATION_SPEC]
1. src/tui/app.rs — add helper methods on `App`:
   - `pub fn select_in_direction(&mut self, dcol: i32, drow: i32)`: finds the visible component in `self.visible_component_indices()` whose `(col - cur_col, row - cur_row)` has positive dot product with `(dcol, drow)` and minimizes Manhattan distance `|dx| + |dy|` (tie-breaking by index). Updates `self.selected_index` and resets `self.detail_scroll = 0`. If no neighbor exists in that direction, leaves selection unchanged.
   - `pub fn move_selected_card(&mut self, dcol: i32, drow: i32)`: computes target `(new_col, new_row)` clamped to `0..self.project.config.grid.cols.saturating_sub(1)` and `0..self.project.config.grid.rows.saturating_sub(1)`. Checks if any other component occupies `GridPos { col: new_col, row: new_row }`; if occupied, sets `self.status_message = "Cell occupied".to_string()` and returns. Otherwise updates the selected component's `position`, sets `self.status_message = format!("Moved '{}' to ({new_col}, {new_row})", id)`, and if `self.root.join(crate::storage::ROCKET_DIR).is_dir()` persists via `crate::storage::save_component(&self.root, comp)` (recording error text in `self.status_message` on failure).
   - `pub fn toggle_connection_to_selected(&mut self)`: requires `self.connect_source_id` and `self.selected_component()`. If source == target, sets `self.status_message = "Cannot connect component to itself".to_string()` and returns. Otherwise checks `self.project.config.connections` for an existing connection with `from == src && to == dst`: if present, removes it and sets `status_message = format!("Disconnected {src} -> {dst}")`; if absent, pushes `Connection { from: src, to: dst, kind: EdgeKind::DependsOn, label: None }` and sets `status_message = format!("Connected {src} -> {dst}")`. Persists `project.toml` via `crate::storage::save_project_config(&self.root, &self.project.config)` when `self.root.join(crate::storage::ROCKET_DIR).is_dir()`, calls `self.refresh_audit()`, clears `self.connect_source_id = None`, and sets `self.mode = InputMode::Navigate`.
2. src/tui/app.rs — update `handle_key`:
   - `KeyCode::Esc` → clears `self.connect_source_id = None`, sets `self.mode = InputMode::Navigate`, `self.status_message = "Mode: Navigate".to_string()`.
   - `KeyCode::Char('c')` → captures `self.connect_source_id = self.selected_component().map(|c| c.id.clone())`, sets `self.mode = InputMode::Connect`, and updates `self.status_message` with the source ID (or `"Select a component first"` if none).
   - `KeyCode::Enter` when `self.mode == InputMode::Connect` → calls `self.toggle_connection_to_selected()`.
   - Arrow / `hjkl` keys:
     - When `self.mode == InputMode::MoveCard`: `Left|'h' => move_selected_card(-1, 0)`, `Right|'l' => move_selected_card(1, 0)`, `Up|'k' => move_selected_card(0, -1)`, `Down|'j' => move_selected_card(0, 1)`.
     - When `self.mode != InputMode::MoveCard` and `self.focus == FocusColumn::Canvas`: `Left|'h' => select_in_direction(-1, 0)`, `Right|'l' => select_in_direction(1, 0)`, `Up|'k' => select_in_direction(0, -1)`, `Down|'j' => select_in_direction(0, 1)`.
     - Keep existing `List` and `Detail` `j`/`k`/`Up`/`Down` behavior unchanged.
3. Tests in `src/tui/app.rs`:
   - `move_card_updates_grid_pos_and_blocks_collision`: place `"a"` at `(0, 0)` and `"b"` at `(1, 0)`; press `'m'` then `Right` (assert `"a"` stays at `(0, 0)` and `app.status_message == "Cell occupied"`); press `Down` (assert `"a"` moves to `(0, 1)`).
   - `connect_mode_adds_and_removes_edge`: place `"a"` at `(0, 0)` and `"b"` at `(1, 0)`; press `'c'` (assert `connect_source_id.as_deref() == Some("a")`); move selection to `"b"` and press `Enter` (assert `connections.len() == 1`, `from == "a"`, `to == "b"`, `mode == InputMode::Navigate`); repeat `'c'` from `"a"` to `"b"` + `Enter` (assert `connections.is_empty()`).
   - `canvas_directional_selection_picks_neighbor`: place `"a"` at `(0, 0)` and `"b"` at `(2, 0)`; set `focus = FocusColumn::Canvas`; press `Right`; assert `app.selected_component().unwrap().id == "b"`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn move_selected_card\b' src/tui/app.rs && grep -qE 'pub fn toggle_connection_to_selected\b' src/tui/app.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```


---

## Phase 5 Step Blocks

### STEP 4E: Fix Horizontal Edge Label Overwriting Arrowhead (`BUG-0001`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 4E: Fix edge label rendering in src/tui/canvas.rs so labels never overwrite connector arrowheads or card interiors (BUG-0001).

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/canvas.rs
READ_ONLY_REFS:
  - src/tui/canvas.rs (render_canvas, route_orthogonal_edge)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal:
  Resolve BUG-0001 so connection labels rendered by `render_canvas` do not overwrite the connector arrowhead cell or bleed into component card rectangles.

Contract & Behavior:
  - Keep existing `route_orthogonal_edge` and `render_canvas` public signatures intact.
  - Place edge labels cleanly off the arrowhead cell (e.g., on the row above a horizontal segment when inside canvas bounds) and clip/skip any character cell that would fall outside `inner`, onto the arrowhead cell, or inside any visible component card `Rect`.
  - Short or long labels must degrade gracefully via truncation/clipping without panicking or corrupting card borders.

Tests:
  - Add unit test(s) in `src/tui/canvas.rs` rendering `render_canvas` onto a `TestBackend` with a labeled horizontal edge between two cards:
    - Verify the arrowhead (`'▶'`) is always preserved.
    - Verify the label is rendered in available space (using card spacing or label length where the label fits, and/or verifying adjacent-card clipping preserves card borders and the arrowhead).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 4F: Wire Snap Grid Canvas and Keyboard-Only Canvas Controls

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 4F: Wire render_canvas into the middle column of src/tui/ui.rs and implement keyboard-only canvas controls (directional selection, MoveCard with collision detection, Connect mode edge toggling) in src/tui/app.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/app.rs
  - src/tui/ui.rs
READ_ONLY_REFS:
  - src/tui/canvas.rs (CanvasViewport, render_canvas)
  - src/model.rs (GridPos, Connection, EdgeKind, Project)
  - src/storage.rs (save_project_config, save_component, ROCKET_DIR)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No modal implementation yet (belongs to STEP 5A–5B).
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal:
  Replace the canvas placeholder with the live Snap Grid Canvas and add keyboard-only spatial navigation, card repositioning, and connection toggling.

Contract & Behavior:
  - `App` fields (public, initialized in `App::new`):
    - `pub viewport: CanvasViewport` (default)
    - `pub connect_source_id: Option<String>` (`None`)
  - UI wiring (`src/tui/ui.rs`):
    - Replace the middle-column placeholder in `render` with `crate::tui::canvas::render_canvas(frame, cols[1], app, app.viewport)` and remove unused placeholder code/imports.
  - Canvas controls (`src/tui/app.rs`):
    - Directional selection (`Left`/`Right`/`Up`/`Down` or `h`/`j`/`k`/`l` when focused on `FocusColumn::Canvas` and not in `MoveCard`): move selection to the nearest visible component in the pressed direction (minimizing Manhattan distance; leave selection unchanged if none exists; reset `detail_scroll = 0`). Keep existing `List` and `Detail` navigation unchanged.
    - MoveCard mode (`m` toggles `InputMode::MoveCard`; arrows/`hjkl` move the selected card by 1 cell): clamp to grid bounds (`0..cols`, `0..rows`), refuse occupied cells with a clear `status_message`, update position, and persist via `save_component` when `<root>/.rocket` exists (surfacing any I/O error in `status_message`).
    - Connect mode (`c` captures selected component ID into `connect_source_id` and enters `InputMode::Connect`; `Enter` toggles a `Connection` from source to currently selected component): reject self-connections, remove an existing matching `(from, to)` edge or add a new `EdgeKind::DependsOn` edge, persist via `save_project_config` when `<root>/.rocket` exists, call `refresh_audit()`, clear `connect_source_id`, and return to `InputMode::Navigate`.
    - `Esc` clears `connect_source_id`, returns to `InputMode::Navigate`, and updates `status_message`.

Tests:
  - In `src/tui/ui.rs`: render a test `App` with two connected components onto `TestBackend` and verify component names, task progress, and connector arrow appear.
  - In `src/tui/app.rs`: test directional canvas selection, MoveCard movement + occupied-cell collision blocking, and Connect mode adding then removing an edge.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 5A: Interactive Modal Forms (New Component, Edit Component, Edit Connection)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 5A: Implement structured TUI modal form state and key handling in src/tui/modal.rs and expose it from src/tui/mod.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/modal.rs (NEW)
  - src/tui/mod.rs
READ_ONLY_REFS:
  - src/archetype.rs (ARCHETYPES, ArchetypeSpec, scaffold_component)
  - src/model.rs (Component, ComponentKind, EdgeKind, FeatureItem, GridPos)
  - src/storage.rs (validate_component_id)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No App or ui.rs edits in this step (belongs to STEP 5B).
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal:
  Create a self-contained, testable modal form state machine supporting component creation from archetypes, structured component editing (including adding a feature), and connection editing.

Contract & Behavior:
  - Expose `pub mod modal;` in `src/tui/mod.rs`.
  - Public types in `src/tui/modal.rs`:
    - `pub const EDGE_KINDS: &[EdgeKind]` covering all `EdgeKind` variants.
    - `pub enum ModalState` with variants `NewComponent(NewComponentForm)`, `EditComponent(EditComponentForm)`, and `EditConnection(EditConnectionForm)` (with public form fields needed by `ui.rs` to render labels, values, active field index, and validation errors).
    - `pub enum ModalAction` with variants:
      - `None`
      - `Cancel`
      - `CreateComponent { id: String, name: String, kind: ComponentKind }`
      - `UpdateComponent { component_id: String, name: String, description: String, relative_path: Option<String>, tags: Vec<String>, add_feature: Option<String> }`
      - `UpdateConnection { from: String, to: String, kind: EdgeKind, label: Option<String> }`
  - `ModalState` constructors & methods:
    - `new_component()`, `edit_component(comp: &Component)`, `edit_connection(from: &str, to: &str, kind: EdgeKind, label: Option<&str>)`, `title(&self) -> &'static str`, and `handle_key(&mut self, key: crossterm::event::KeyEvent) -> ModalAction`.
  - Key handling invariants:
    - `Esc` returns `ModalAction::Cancel`.
    - `Tab`/`Down` and `BackTab`/`Up` cycle active fields with wrap-around.
    - `Left`/`Right` cycle selector fields (archetype in `NewComponent`, edge kind in `EditConnection`).
    - Printable chars (without Ctrl/Alt) append to the active text field; `Backspace` pops the last char.
    - `Enter` validates inputs: `NewComponent` validates ID via `validate_component_id` and requires non-empty trimmed name (setting `error` and returning `ModalAction::None` on failure); `EditComponent` splits comma-separated tags (trimming empties) and turns non-empty new feature title into `add_feature: Some(...)`; `EditConnection` returns `UpdateConnection` with trimmed optional label.

Tests:
  - Unit tests in `src/tui/modal.rs` covering:
    - `NewComponent` validation rejection on empty/invalid input, then valid submission returning `CreateComponent`.
    - `EditComponent` comma-separated tag parsing and optional `add_feature`.
    - `EditConnection` kind cycling and label update, plus `Esc` cancellation.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 5B: Wire Interactive Modals and Centered Overlay into App and UI

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 5B: Wire ModalState into App ('n', 'e', 'E') and render centered modal overlays in src/tui/ui.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/app.rs
  - src/tui/ui.rs
READ_ONLY_REFS:
  - src/tui/modal.rs (ModalState, ModalAction, EDGE_KINDS)
  - src/archetype.rs (ARCHETYPES, scaffold_component)
  - src/storage.rs (save_project_config, save_component, ROCKET_DIR)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub enum ModalState\b' src/tui/modal.rs

[MUTATION_SPEC]
Goal:
  Connect the modal forms to `App` keybindings and persistence, and render active modals as a centered overlay on top of the 3-column layout.

Contract & Behavior:
  - `App` state (`src/tui/app.rs`):
    - Add `pub modal: Option<crate::tui::modal::ModalState>` (initialized to `None`).
    - When `self.modal` is `Some` (and key is not `Ctrl+c`), intercept all key events in `handle_key`, delegate to `modal.handle_key(key)`, and apply the resulting `ModalAction`:
      - `Cancel` closes the modal and returns to `InputMode::Navigate`.
      - `CreateComponent` rejects duplicate IDs cleanly, scaffolds via `scaffold_component` at the first unoccupied grid position, persists via `save_component` when `<root>/.rocket` exists, appends and selects the new component, calls `refresh_audit()`, and closes the modal.
      - `UpdateComponent` updates component fields, appends a new `FeatureItem` when `add_feature` is `Some`, persists via `save_component` when `<root>/.rocket` exists, calls `refresh_audit()`, and closes the modal.
      - `UpdateConnection` updates the matching connection in `project.config.connections`, persists `project.toml` when `<root>/.rocket` exists, calls `refresh_audit()`, and closes the modal.
    - Non-modal keybindings:
      - `'n'` opens `ModalState::new_component()` (`InputMode::Modal`).
      - `'e'` opens `ModalState::edit_component` for the selected component (`InputMode::Modal`).
      - `'E'` opens `ModalState::edit_connection` for the selected component's first outgoing connection (`InputMode::Modal`), or sets a helpful `status_message` if none exists.
  - Overlay rendering (`src/tui/ui.rs`):
    - When `app.modal` is `Some(modal)`, render a centered bordered overlay on top of the main UI using `ratatui::widgets::Clear`, showing `modal.title()`, all fields with the active field highlighted, any validation error in the theme's error color, and a concise keybinding footer hint (`Tab`, `←/→`, `Enter`, `Esc`).

Tests:
  - In `src/tui/app.rs`: drive `'n'` through typing an ID/name and pressing `Enter`, verifying the new component is scaffolded, added, selected, and the modal closes back to `Navigate`.
  - In `src/tui/ui.rs`: render an `App` with an open modal onto `TestBackend` and verify the modal title and keybinding hints appear.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 5C: Suspend-to-`$EDITOR` Markdown Workflow with Automatic Reload

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 5C: Wire suspend-to-$EDITOR Markdown editing ('v' key) in App and run_event_loop with automatic disk reload and audit refresh.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/app.rs
  - src/tui/terminal.rs
READ_ONLY_REFS:
  - src/tui/terminal.rs (suspend_for_editor, run_event_loop)
  - src/storage.rs (ROCKET_DIR, COMPONENTS_DIR, save_component)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal:
  Allow the user to press `'v'` on any selected component to open its `.rocket/components/<id>.md` spec in `$EDITOR`, then automatically reload the edited Markdown and refresh audit state upon return.

Contract & Behavior:
  - `src/tui/app.rs`:
    - Add `pub pending_editor_component: Option<String>` to `App` (initialized to `None`).
    - In non-modal `handle_key`: `'v'` queues the selected component's ID into `pending_editor_component` and updates `status_message`.
    - Add `pub fn reload_component_markdown(&mut self, component_id: &str) -> crate::RfResult<()>`: reads `<root>/.rocket/components/<component_id>.md`, updates that component's `markdown_body`, calls `refresh_audit()`, and updates `status_message`.
  - `src/tui/terminal.rs`:
    - In `run_event_loop`, after `app.handle_key(key)`, drain `app.pending_editor_component.take()`:
      - Ensure the target `.md` file exists on disk (persisting the component via `save_component` first if missing).
      - Invoke `suspend_for_editor(terminal, &md_path)`.
      - On `Ok(())`, call `app.reload_component_markdown(&comp_id)` (capturing any error in `app.status_message`); on `Err(err)`, record the error in `app.status_message` without crashing the TUI.

Tests:
  - In `src/tui/app.rs`: using a `tempfile::tempdir()` workspace, save a component, modify its `.rocket/components/<id>.md` file on disk, press `'v'` (asserting `pending_editor_component` is set), call `reload_component_markdown`, and assert `markdown_body` matches the updated file.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```


---

## Phase 6 Step Blocks

### STEP 6A: Editor Path-Containment Parity, `$EDITOR` Args, and Unit-Tested Editor Flow (`BUG-0002`, `TASK-0027`, `TASK-0028`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 6A: Enforce ID validation and canonical path containment before $EDITOR launch, support $EDITOR arguments, and extract + unit-test the editor workflow in src/tui/terminal.rs (BUG-0002, TASK-0027, TASK-0028).

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/terminal.rs
READ_ONLY_REFS:
  - src/tui/terminal.rs:68-174 (suspend_for_editor, run_event_loop)
  - src/storage.rs:12-106 (validate_component_id, ensure_contained, save_component, COMPONENTS_DIR, ROCKET_DIR)
  - src/tui/app.rs:90-125 (reload_component_markdown)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal:
  Close BUG-0002 (path-containment parity before launching $EDITOR), resolve TASK-0027 (unit-test the editor orchestration flow), and fix TASK-0028 item (6) ($EDITOR strings with arguments).

Contract & Behavior:
  - `$EDITOR` parsing:
    - Split the `$EDITOR` value on ASCII whitespace into `(program, args)` (defaulting to `"vi"` with empty args when unset/blank) so values like `"code --wait"` execute `Command::new("code").args(["--wait"]).arg(file_path)`.
  - Path containment parity & component validation:
    - Before launching the editor for a queued `comp_id`:
      1. Validate `comp_id` with `crate::storage::validate_component_id(&comp_id)`.
      2. Confirm the component exists in `app.project.component(&comp_id)` (never launch `$EDITOR` on a nonexistent component or nonexistent `.md` file; record a clear error in `app.status_message` and return).
      3. Ensure `<root>/.rocket/components/<comp_id>.md` exists on disk (persisting via `save_component` if missing; record any save error in `app.status_message` and return without launching `$EDITOR`).
      4. Canonicalize `<root>/.rocket/components` and the target `<comp_id>.md` path and verify `canon_md.starts_with(&canon_dir)` (rejecting symlink escapes or traversal with an error in `app.status_message` without launching `$EDITOR`).
  - Testable orchestration helper (no unwired scaffolding):
    - Extract a helper (e.g., `fn process_pending_editor_with<F>(app: &mut App, mut run_editor: F) where F: FnMut(&Path) -> RfResult<()>`) that drains `app.pending_editor_component.take()`, runs the validation/auto-save/containment checks, invokes `run_editor(&canon_md)`, and calls `app.reload_component_markdown(&comp_id)` on `Ok(())` (surfacing reload or editor errors in `app.status_message`).
    - Wire `run_event_loop` to call `process_pending_editor_with(app, |path| suspend_for_editor(terminal, path))`.

Tests:
  - Unit tests in `src/tui/terminal.rs` covering:
    - `$EDITOR` parsing with arguments (e.g. `"code --wait"`) and blank/default fallback (`"vi"`).
    - Auto-saving a missing `.md` file, invoking the mock editor closure to modify the file, and verifying `app.project` reloads the updated `markdown_body`.
    - Rejecting an invalid component ID (e.g. `"../escape"`), an unknown component ID, and a symlink pointing outside `.rocket/components/` — verifying the mock editor closure is never called and `app.status_message` reports the error.
    - Surfacing a mock editor `Err(...)` in `app.status_message` without panicking.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 6B: Transactional TUI Mutation Rollback, Canvas Viewport Panning & Phase 5 Polish (`BUG-0003`, `BUG-0004`, `TASK-0028`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 6B: Wire CanvasViewport auto-panning via ensure_visible, add rollback on disk-save failure for TUI mutations, and resolve remaining Phase 5 polish items in src/tui/app.rs, src/tui/ui.rs, and src/tui/canvas.rs (BUG-0003, BUG-0004, TASK-0028).

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/app.rs
  - src/tui/ui.rs
  - src/tui/canvas.rs
READ_ONLY_REFS:
  - src/tui/app.rs:280-660 (move_selected_card, confirm_connect, first_unoccupied_grid_pos, apply_modal_action, handle_key)
  - src/tui/ui.rs:13-42,258-265,380-406 (render, render_footer, render_modal)
  - src/tui/canvas.rs:72-86,294-348 (ensure_visible, compute_label_pos)
  - src/tui/terminal.rs:110-120 (run_event_loop draw call passing &mut App)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal:
  Resolve BUG-0003 (wire `App::viewport` / `ensure_visible` so off-screen cards pan into view), BUG-0004 (prevent in-memory state desync when disk saves fail), and TASK-0028 items (1)–(5).

Contract & Behavior:
  - Canvas viewport panning (`BUG-0003`, no unwired scaffolding):
    - Update `src/tui/ui.rs` `pub fn render(frame: &mut Frame, app: &mut App)` (note `terminal.rs` already passes `app: &mut App`) so before calling `crate::tui::canvas::render_canvas(frame, cols[1], app, app.viewport)`, it computes the canvas inner `Rect` (`Block::default().borders(Borders::ALL).inner(cols[1])`) and calls `crate::tui::canvas::ensure_visible(canvas_inner, &mut app.viewport, selected.position)` whenever `app.selected_component()` is `Some(selected)`.
  - Transactional rollback on disk save failure (`BUG-0004`):
    - In `src/tui/app.rs`, ensure `move_selected_card`, `confirm_connect`, `ModalAction::UpdateComponent`, and `ModalAction::UpdateConnection` never leave `self.project` mutated in memory if `save_component` or `save_project_config` returns `Err`:
      - Validate/save a candidate clone (or restore the previous component / connection state on `Err`), keep `self.audit_findings` consistent via `refresh_audit()`, and record the failure in `self.status_message`.
  - Phase 5 polish (`TASK-0028` items 1–5):
    1. `src/tui/canvas.rs` `compute_label_pos`: avoid placing L-bend corner midpoint labels onto occupied horizontal segment cells (prefer placing above/below the corner or on a clear non-segment cell inside `bounds`).
    2. `src/tui/app.rs` `first_unoccupied_grid_pos`: return `Option<GridPos>` instead of silently falling back to `(0, 0)` when the grid is full; in `ModalAction::CreateComponent`, reject creation with a clear error message on the modal and `status_message` when no grid cell is free.
    3. `src/tui/ui.rs` `render_modal`: compute `popup_width` using `u32` arithmetic (`(u32::from(area.width) * 60 / 100) as u16`) before `.clamp(54, 76)` so wide terminals (`width > 1092`) cannot overflow `u16`.
    4. `src/tui/app.rs` `'v'` key handler: clear `self.connect_source_id = None` and set `self.mode = InputMode::Navigate` before queueing `pending_editor_component`.
    5. `src/tui/ui.rs` `render_footer`: include `[v] Edit MD` in the footer keybind hint bar.

Tests:
  - In `src/tui/ui.rs`:
    - Render an `App` with a selected component at an off-screen grid cell (e.g. `col: 4, row: 2`) onto `TestBackend::new(120, 35)`; assert `app.viewport` updated (`scroll_col > 0 || scroll_row > 0`) and the off-screen component's name appears in the rendered buffer.
    - Verify footer text includes `[v]`.
  - In `src/tui/app.rs`:
    - Test save-failure rollback for at least `move_selected_card`, `confirm_connect`, `UpdateComponent`, and `UpdateConnection` (e.g., making `.rocket/components` or `.rocket/project.toml` unwritable or a directory so save fails) and assert in-memory `self.project` remains unchanged.
    - Test `CreateComponent` on a full `1x1` grid rejects creation without overwriting `(0, 0)`.
    - Test pressing `'v'` while in `Connect` mode clears `connect_source_id` and returns to `InputMode::Navigate`.
  - In `src/tui/canvas.rs`:
    - Test L-bend labeled edge rendering preserves all horizontal/corner segment cells and arrowhead without label overwrite.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 6C: Live Adapter Sync in TUI (Startup Pull, Manual Pull `r`/`S`, Push `p`, and Optional Auto-Poll)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 6C: Implement live adapter sync in the TUI (startup pull, manual pull 'r'/'S', push selected 'p', and optional auto-poll interval) across src/tui/app.rs, src/tui/terminal.rs, and src/main.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/app.rs
  - src/tui/terminal.rs
  - src/main.rs
READ_ONLY_REFS:
  - src/adapter.rs:75-265 (load_adapters_file, resolve_active_adapters, reconcile_status, run_push_component, run_push_feature, run_pull_status)
  - src/cli.rs:140-235 (push and pull execution semantics)
  - src/storage.rs:138-200 (save_component, ROCKET_DIR)
  - src/main.rs:1-39 (main, adapters_path)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - Do not alter any CLI subcommand handling (`Some(command)` arm) in src/main.rs.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub fn run_pull_status\b' src/adapter.rs && grep -qE 'pub fn reconcile_status\b' src/adapter.rs

[MUTATION_SPEC]
Goal:
  Enable live external tracker synchronization inside the TUI: automatic pull on startup (when active adapters are configured), manual pull (`'r'` / `'S'`), manual push of the selected component and its unpushed features (`'p'`), and optional periodic auto-poll in the event loop.

Contract & Behavior:
  - `src/tui/app.rs`:
    - Define `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum SyncRequest { Pull, PushSelected }`.
    - Add `pub pending_sync: Option<SyncRequest>` to `App` (initialized to `None`).
    - In non-modal `handle_key`:
      - `'r'` or `'S'` clears `connect_source_id`, resets `mode` to `InputMode::Navigate`, queues `self.pending_sync = Some(SyncRequest::Pull)`, and sets `status_message = "Syncing status from active adapters...".to_string()`.
      - `'p'` clears `connect_source_id`, resets `mode` to `InputMode::Navigate`; if a component is selected, queues `self.pending_sync = Some(SyncRequest::PushSelected)` with a status message, else sets `"No component selected to push"`.
    - Add `pub fn sync_pull(&mut self, adapters_path: &Path) -> RfResult<String>`:
      - If `self.project.config.active_adapters.is_empty()`, sets `status_message = "No active adapters configured"` and returns `Ok(...)`.
      - Loads `adapters_path` via `load_adapters_file`, resolves `active_adapters`, runs `run_pull_status` on each adapter with `pull_status_cmd`, reconciles into a candidate/clone (or rolls back on `save_component` failure), persists updated components via `save_component` when `<root>/.rocket` exists, calls `refresh_audit()`, updates `status_message` with a summary (components updated, tasks synced, features linked), and returns `Ok(summary_msg)`.
    - Add `pub fn sync_push_selected(&mut self, adapters_path: &Path) -> RfResult<String>`:
      - Requires a selected component and non-empty `active_adapters` (returning clear status message if none).
      - Runs `run_push_component` (if configured) and `run_push_feature` for each unpushed feature (marking `pushed = true` and recording `external_id` on success), persists the component via `save_component` when `<root>/.rocket` exists (without leaving partial desync on save failure), calls `refresh_audit()`, updates `status_message`, and returns `Ok(summary_msg)`.
  - `src/tui/terminal.rs`:
    - Update `pub fn run_tui(root: &Path, theme_dirs: &[PathBuf], adapters_path: &Path, poll_interval: Option<Duration>) -> RfResult<()>`:
      - On startup, if `!app.project.config.active_adapters.is_empty()`, run an initial `app.sync_pull(adapters_path)` (recording any error in `app.status_message` without aborting TUI startup).
      - Pass `adapters_path` and `poll_interval` into `run_event_loop`.
    - Extract a testable helper `fn process_pending_sync(app: &mut App, adapters_path: &Path)` that drains `app.pending_sync.take()` and dispatches `SyncRequest::Pull` / `SyncRequest::PushSelected`, surfacing any `Err(err)` in `app.status_message`.
    - In `run_event_loop`, call `process_pending_sync(app, adapters_path)` after key handling; when `poll_interval` is `Some(interval)` and `!app.project.config.active_adapters.is_empty()` and `last_poll.elapsed() >= interval` (and no modal is open), trigger `app.sync_pull(adapters_path)` (recording errors in `app.status_message`) and reset `last_poll`.
  - `src/main.rs`:
    - In the `None =>` TUI launch arm, read optional `RF_POLL_INTERVAL_SECS` env var (parsing positive `u64` seconds into `Some(Duration::from_secs(secs))`, otherwise `None`) and pass `&adapters_path` and `poll_interval` to `rocket_factory::tui::run_tui`.

Tests:
  - In `src/tui/app.rs`:
    - Verify `'r'`, `'S'`, and `'p'` queue the expected `SyncRequest` variants.
    - Using `tempfile::tempdir()` and a mock `sh` adapter in `adapters.toml`, test `sync_pull` updates `external_tasks`, flips component `CardStatus` to `Complete`, persists to disk, and updates `status_message`.
    - Using a mock `sh` adapter, test `sync_push_selected` marks unpushed features as `pushed = true` with returned `external_id` and persists to disk.
  - In `src/tui/terminal.rs`:
    - Test `process_pending_sync` executes queued `Pull` and surfaces adapter errors into `app.status_message` without panicking.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 6D: Path-Safe Theme Resolution and RON Theme Discovery (`src/theme.rs`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 6D: Add path-traversal validation to resolve_theme and implement list_available_themes in src/theme.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/theme.rs
READ_ONLY_REFS:
  - src/theme.rs:62-125 (default_theme, load_theme_file, resolve_theme)
  - src/storage.rs:12-23 (validate_component_id style)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No ratatui dependency in src/theme.rs (keep core theme module UI-library-agnostic).
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub fn resolve_theme\b' src/theme.rs

[MUTATION_SPEC]
Goal:
  Ensure theme name resolution enforces path-containment parity and provide deterministic discovery of available `.ron` themes for the interactive theme switcher.

Contract & Behavior:
  - Path-safe theme name validation in `src/theme.rs`:
    - Validate `theme_name` inside `resolve_theme` (and helper `pub fn validate_theme_name(name: &str) -> RfResult<()>`): reject empty names, names containing path separators (`'/'`, `'\\'`), `".."`, or characters outside `[a-zA-Z0-9_-]`.
    - When resolving `<dir>/<theme_name>.ron`, verify canonical path containment (`canonicalize(&candidate)` starts with `canonicalize(dir)`) before calling `load_theme_file`, returning `RfError::PathEscape` on symlink escape.
  - `pub fn list_available_themes(search_dirs: &[PathBuf]) -> Vec<String>`:
    - Always includes `"crt_amber"`.
    - Scans each existing directory in `search_dirs` for `.ron` files whose file stem passes `validate_theme_name` and whose canonical path is contained within that directory.
    - Deduplicates and returns sorted theme names (`Vec<String>`).

Tests:
  - Unit tests in `src/theme.rs` covering:
    - `resolve_theme` rejects traversal names (`"../secret"`, `"a/b"`, `""`) and symlink escapes pointing outside the search dir.
    - `list_available_themes` always includes `"crt_amber"`, discovers custom `.ron` files across multiple dirs, ignores non-`.ron` files and invalid names, and returns deduplicated sorted names.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn list_available_themes\b' src/theme.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 6E: Keyboard Help Overlay (`?`) and RON Theme Switcher Modal (`t`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 6E: Implement the '?' keyboard help overlay and 't' RON theme switcher modal across src/tui/modal.rs, src/tui/app.rs, and src/tui/ui.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/modal.rs
  - src/tui/app.rs
  - src/tui/ui.rs
READ_ONLY_REFS:
  - src/theme.rs:62-140 (resolve_theme, list_available_themes, Theme)
  - src/tui/terminal.rs:28-36 (default_theme_dirs)
  - src/tui/modal.rs:1-150 (ModalState, ModalAction, handle_key)
  - src/tui/ui.rs:258-406 (render_footer, render_modal)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub fn list_available_themes\b' src/theme.rs && grep -qE 'pub enum ModalState\b' src/tui/modal.rs

[MUTATION_SPEC]
Goal:
  Provide an in-app `?` keyboard reference overlay and an interactive `t` RON theme switcher modal that applies and persists the selected theme.

Contract & Behavior:
  - `src/tui/modal.rs`:
    - Add `#[derive(Debug, Clone, PartialEq, Eq)] pub struct ThemeSwitcherForm { pub themes: Vec<String>, pub selected_index: usize, pub error: Option<String> }` with helper `pub fn selected_theme(&self) -> Option<&str>`.
    - Extend `ModalState` with:
      - `Help`
      - `ThemeSwitcher(ThemeSwitcherForm)`
    - Extend `ModalAction` with:
      - `SelectTheme { name: String }`
    - Add constructors `ModalState::help()` and `ModalState::theme_switcher(themes: Vec<String>, current_theme: &str)` (pre-selecting `current_theme` if present).
    - Update `title(&self)` (`"Keyboard Help"`, `"Switch Theme"`) and `handle_key`:
      - `Help`: `Esc`, `Enter`, `'?'`, or `'q'` returns `ModalAction::Cancel`.
      - `ThemeSwitcher`: `Up`/`'k'` and `Down`/`'j'` move `selected_index` with wrap-around; `Enter` returns `ModalAction::SelectTheme { name }` (or `Cancel` if empty); `Esc` returns `ModalAction::Cancel`.
  - `src/tui/app.rs`:
    - Add `pub theme_dirs: Vec<PathBuf>` to `App`, initialized in `App::new` to `crate::tui::terminal::default_theme_dirs(&root)` (preserving `App::new` signature).
    - Non-modal `handle_key`:
      - `'?'` clears `connect_source_id`, opens `ModalState::help()`, sets `mode = InputMode::Modal`, and updates `status_message`.
      - `'t'` clears `connect_source_id`, discovers themes via `crate::theme::list_available_themes(&self.theme_dirs)`, opens `ModalState::theme_switcher(themes, &self.project.config.theme)`, sets `mode = InputMode::Modal`, and updates `status_message`.
    - In `apply_modal_action`, handle `ModalAction::SelectTheme { name }`:
      - Resolve the theme via `crate::theme::resolve_theme(&name, &self.theme_dirs)`. On error, display error in the modal and `status_message` without closing.
      - Persist updated `project.config.theme = name` via `save_project_config` when `<root>/.rocket` exists — rolling back `project.config.theme` and keeping the previous `self.theme` if saving fails (BUG-0004 parity).
      - On success, update `self.theme`, close modal, return to `InputMode::Navigate`, and set `status_message = format!("Theme switched to {name}")`.
  - `src/tui/ui.rs`:
    - Render `ModalState::Help` in `render_modal` with a concise, organized list of all TUI keybindings (`Tab/Shift+Tab`, `hjkl`/Arrows, `n`, `e`, `E`, `m`, `c`, `v`, `f`, `r`/`S`, `p`, `t`, `?`, `Esc`, `q`).
    - Render `ModalState::ThemeSwitcher` in `render_modal` listing available themes with the selected row highlighted, active marker for current theme, error line if any, and `[↑/↓] Select  [Enter] Apply  [Esc] Cancel` footer hint.
    - Update `render_footer` to include concise hints for `[r/S] Sync`, `[p] Push`, `[t] Theme`, and `[?] Help`.

Tests:
  - In `src/tui/modal.rs`: test `Help` dismissal (`Esc`/`?`) and `ThemeSwitcher` navigation + `Enter` returning `ModalAction::SelectTheme`.
  - In `src/tui/app.rs`: write a custom `amber_alt.ron` in a tempdir, set `app.theme_dirs`, drive `'t'` → select `amber_alt` → `Enter`, and verify `app.theme.name == "amber_alt"`, `project.config.theme == "amber_alt"`, and persisted `project.toml` reflects `"amber_alt"`.
  - In `src/tui/ui.rs`: render both `ModalState::Help` and `ModalState::ThemeSwitcher` onto `TestBackend` and assert titles and keybind/theme entries appear.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 6F: End-to-End Integration Test Suite (`tests/e2e.rs`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 6F: Add end-to-end integration tests in tests/e2e.rs covering CLI workflows, mock adapter push/pull scripts, live TUI sync, theme switching, and Ratatui TestBackend snapshot rendering.

[SCOPE_LOCK]
ALLOWED_FILES:
  - tests/e2e.rs (NEW)
READ_ONLY_REFS:
  - src/lib.rs (public module exports)
  - src/cli.rs (Command, ExportKind, execute_command)
  - src/tui/app.rs (App, sync_pull, sync_push_selected, handle_key)
  - src/tui/ui.rs (render)
  - src/theme.rs (DEFAULT_CRT_AMBER_RON, resolve_theme)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/ or tests/.
  - Test fixtures must use reserved placeholder data only (`example.com`, `192.0.2.0/24`, `testuser`).

[PRE_GATE]
grep -qE 'pub fn sync_pull\b' src/tui/app.rs && grep -qE 'pub fn list_available_themes\b' src/theme.rs

[MUTATION_SPEC]
Goal:
  Verify the complete Rocket Factory platform end-to-end from the public library API across CLI commands, subprocess mock adapters, persistence, live TUI sync, overlays, and Ratatui `TestBackend` rendering.

Contract & Behavior:
  - Create `tests/e2e.rs` with integration tests exercising:
    1. **CLI lifecycle + mock adapter roundtrip**:
       - `init` a workspace in a `tempfile::tempdir()`.
       - Scaffold and save components (`view_box`, `data_stream`) with a connection and an unpushed feature.
       - Run `Command::Audit { strict: true }` and assert non-zero exit code when a gap component exists.
       - Run `Command::Export` for `Manifest`, `Roadmap`, and `Json` formats and verify key output fields.
       - Configure a mock `sh` adapter in `adapters.toml` and run `Command::Push` followed by `Command::Pull`, verifying features become `pushed = true` with `external_id` and `external_tasks` reconcile to `TaskStatus::Complete`.
       - Re-run `Command::Audit { strict: true }` after closing gaps and assert exit code `0`.
    2. **TUI live sync + complete-green border snapshot**:
       - Load the workspace into `App`, trigger `sync_pull` against a mock `sh` adapter returning `Complete` tasks for a component, render onto `TestBackend::new(140, 40)` via `rocket_factory::tui::ui::render`, and assert:
         - Rendered buffer shows the component name, `Tasks 1/1`, connector arrow, and zero gaps.
         - At least one border cell of the completed card uses the theme's `status_complete` RGB color (`Color::Rgb(r, g, b)` with `g > r`).
    3. **TUI interactive workflows snapshot (viewport panning, modals, theme switch, help)**:
       - Place a component off-screen (e.g. `col: 4, row: 2`), select it, render onto `TestBackend::new(120, 35)`, and assert viewport pans and renders the off-screen component.
       - Drive `'t'` to switch to a custom `.ron` theme file in a tempdir, verify persistence in `.rocket/project.toml`, and render onto `TestBackend` verifying the new theme's background/foreground RGB color appears in the buffer.
       - Drive `'?'` and verify the `"Keyboard Help"` overlay renders with keybinding documentation.

Tests:
  - All integration tests in `tests/e2e.rs` must pass under `cargo test --workspace` with zero clippy warnings.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/ tests/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```


---

## Phase 7 Step Blocks

### STEP 7A: Propagate `theme_dirs` in `run_tui` and Preserve Modal State on Edit Save Failure (`BUG-0005`, `BUG-0006`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 7A: Propagate caller-supplied theme_dirs into App inside run_tui and keep EditComponent/EditConnection modals open with form.error on disk-save failure (BUG-0005, BUG-0006).

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/terminal.rs
  - src/tui/app.rs
READ_ONLY_REFS:
  - src/tui/terminal.rs:180-206 (run_tui)
  - src/tui/app.rs:615-745 (apply_modal_action: UpdateComponent, UpdateConnection, SelectTheme)
  - src/tui/modal.rs:15-95 (EditComponentForm, EditConnectionForm error fields)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub fn run_tui\b' src/tui/terminal.rs && grep -qE 'pub theme_dirs:' src/tui/app.rs

[MUTATION_SPEC]
Goal:
  Close BUG-0005 (`run_tui` ignoring caller-provided `theme_dirs` for the `t` theme switcher) and BUG-0006 (`UpdateComponent` and `UpdateConnection` discarding user edits by closing the modal on disk-save failure).

Contract & Behavior:
  - `src/tui/terminal.rs` (`BUG-0005`):
    - Extract a testable helper `fn build_tui_app(root: &Path, theme_dirs: &[PathBuf]) -> RfResult<App>` that loads the project via `crate::storage::load_project(root)?`, resolves the initial theme via `crate::theme::resolve_theme(&project.config.theme, theme_dirs)?`, constructs `App::new(root.to_path_buf(), project, theme)`, and sets `app.theme_dirs = theme_dirs.to_vec()`.
    - Wire `run_tui` to call `let mut app = build_tui_app(root, theme_dirs)?;` (no unwired scaffolding).
  - `src/tui/app.rs` (`BUG-0006`):
    - In `apply_modal_action`, for `ModalAction::UpdateComponent`:
      - On `Err(err)` from `save_component`: format `msg = format!("Failed to save {name}: {err}")`, set `form.error = Some(msg.clone())` on `ModalState::EditComponent(ref mut form)`, set `self.status_message = msg`, re-attach `self.modal = Some(modal)`, keep `self.mode = InputMode::Modal`, leave `self.project` unchanged, and `return`.
    - In `apply_modal_action`, for `ModalAction::UpdateConnection`:
      - On `Err(err)` from `save_project_config`: format `msg = format!("Failed to save connections: {err}")`, set `form.error = Some(msg.clone())` on `ModalState::EditConnection(ref mut form)`, set `self.status_message = msg`, re-attach `self.modal = Some(modal)`, keep `self.mode = InputMode::Modal`, leave `self.project.config` unchanged, and `return`.

Tests:
  - In `src/tui/terminal.rs`:
    - Unit test `build_tui_app` with a custom theme directory containing a valid `.ron` file (e.g. `cobalt_custom.ron`), asserting `app.theme_dirs == custom_dirs` and `crate::theme::list_available_themes(&app.theme_dirs)` contains `"cobalt_custom"`.
  - In `src/tui/app.rs`:
    - Update/add save-failure rollback tests for `UpdateComponent` and `UpdateConnection` asserting that on disk-save error: `self.project` is unchanged, `app.mode == InputMode::Modal`, `app.modal.is_some()`, and `form.error.is_some()`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 7B: Dynamic Modal Width for Unclipped 80-Column Keyboard Help (`BUG-0007`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 7B: Size render_modal popup width from rendered line widths so Keyboard Help renders without wrapping or footer clipping on 80-column terminals (BUG-0007).

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/ui.rs
READ_ONLY_REFS:
  - src/tui/ui.rs:385-505 (ModalState::Help lines and render_modal sizing)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'ModalState::Help =>' src/tui/ui.rs

[MUTATION_SPEC]
Goal:
  Close BUG-0007 so `ModalState::Help` (and all other modals) render cleanly on standard 80x24 terminals without horizontal line wrapping pushing the footer hint below the modal border.

Contract & Behavior:
  - In `src/tui/ui.rs` `render_modal`:
    - Compute `content_width` from the longest rendered line in `lines` (`lines.iter().map(Line::width).max().unwrap_or(0)`) plus border/padding allowance (`4` cols), converted safely via `u16::try_from(...).unwrap_or(u16::MAX)`.
    - Compute the target `popup_width` as `((u32::from(area.width) * 60 / 100) as u16).max(content_width).clamp(54, 76)` capped by `area.width.saturating_sub(2)` (when `area.width > 2`, else `area.width`), preserving overflow safety on wide terminals (`width > 1092`).
    - Ensure every line in `ModalState::Help` fits within `72` display columns (and `<= 70` cols) so at `80x24` no line wraps and the bottom footer hint `[Esc/?/Enter/q] Close` remains fully visible inside the modal border.

Tests:
  - In `src/tui/ui.rs`:
    - Add a `TestBackend::new(80, 24)` test rendering `ModalState::Help` and asserting `"Keyboard Help"`, `"New component / Edit component / Edit edge"`, and `"[Esc/?/Enter/q] Close"` all appear intact in the buffer.
    - Keep existing `render_modal_handles_wide_terminal_without_overflow` (`1200x35`) passing.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 7C: Fix Multi-Adapter Feature Push Across CLI and TUI + E2E Verification (`TASK-0035`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 7C: Fix multi-adapter feature push short-circuiting in src/cli.rs and src/tui/app.rs and add E2E integration coverage in tests/e2e.rs (TASK-0035).

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/cli.rs
  - src/tui/app.rs
  - tests/e2e.rs
READ_ONLY_REFS:
  - src/cli.rs:153-195 (Command::Push loop)
  - src/tui/app.rs:195-258 (sync_push_selected)
  - src/adapter.rs:135-225 (run_push_component, run_push_feature)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/ or tests/.
  - No `.unwrap()` or `.expect()` in non-test code.
  - Test fixtures must use reserved placeholder data only (`example.com`, `192.0.2.0/24`, `testuser`).

[PRE_GATE]
grep -qE 'pub fn sync_push_selected\b' src/tui/app.rs && grep -qE 'Command::Push' src/cli.rs

[MUTATION_SPEC]
Goal:
  Close TASK-0035 so configuring multiple active adapters pushes every unpushed feature to all active adapters (instead of short-circuiting after the first adapter flips `feature.pushed = true`), and verify both multi-adapter push and 80-column help rendering end-to-end.

Contract & Behavior:
  - `src/cli.rs` (`Command::Push`):
    - For each target component, snapshot the indices of features where `!feature.pushed` before iterating across `active_adapters` (or push per-component across all `active_adapters` before marking `feature.pushed = true`).
    - For each unpushed feature index, invoke `adapter::run_push_feature` on every active adapter that has `push_feature_cmd.is_some()`, incrementing `features_pushed` for each successful push and recording the first non-`None` `external_id` returned.
    - Only after all active adapters with `push_feature_cmd` succeed for a component's unpushed features, mark those features `pushed = true` (with their recorded `external_id`) and persist via `storage::save_component`.
  - `src/tui/app.rs` (`sync_push_selected`):
    - Apply the exact same multi-adapter feature-push semantics on the candidate `Project` clone: collect unpushed feature indices for `comp_id` up front, run `run_push_feature` across every active adapter with `push_feature_cmd.is_some()`, mark pushed + first non-`None` `external_id` once pushed, persist via `save_component(&self.root, comp)?`, and commit `self.project = candidate` only after disk save succeeds (preserving BUG-0004 rollback).
  - `tests/e2e.rs`:
    - Add an end-to-end test configuring two mock `sh` adapters (`tracker_a`, `tracker_b`) that each log pushed feature payloads to separate temp files, run both CLI `Command::Push` and TUI `sync_push_selected` (on separate components), and assert both adapters received the unpushed features, `features_pushed == 2`, and `external_id` / `pushed = true` persisted to disk.
    - Add an 80x24 `TestBackend` snapshot assertion for `ModalState::Help` verifying the footer hint `[Esc/?/Enter/q] Close` is visible.

Tests:
  - Unit tests in `src/cli.rs` and `src/tui/app.rs` verifying 2 active mock `sh` adapters both receive unpushed features.
  - Integration tests in `tests/e2e.rs` passing under `cargo test --workspace`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/ tests/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```


---

## Phase 8 Step Blocks

### STEP 8A: Shared Multi-Adapter Feature Push Helper (`TASK-0039`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 8A: Extract a shared push_unpushed_features helper in src/adapter.rs and refactor src/cli.rs and src/tui/app.rs to use it (TASK-0039).

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/adapter.rs
  - src/cli.rs
  - src/tui/app.rs
READ_ONLY_REFS:
  - src/adapter.rs:230-260 (run_push_feature)
  - src/cli.rs:150-225 (Command::Push loop)
  - src/tui/app.rs:215-285 (sync_push_selected)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub fn run_push_feature\b' src/adapter.rs && grep -qE 'pub fn sync_push_selected\b' src/tui/app.rs

[MUTATION_SPEC]
Goal:
  Close TASK-0039 by eliminating the duplicated ~45-line multi-adapter feature-push loop between CLI (`Command::Push`) and TUI (`sync_push_selected`) while preserving identical behavior and rollback guarantees.

Contract & Behavior:
  - `src/adapter.rs`:
    - Define `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub struct FeaturePushSummary { pub features_pushed: usize, pub modified: bool }`.
    - Define `pub fn push_unpushed_features(adapters: &[&AdapterConfig], workdir: &Path, component: &mut Component) -> RfResult<FeaturePushSummary>`:
      - Filter `adapters` to those with `push_feature_cmd.is_some()`. If empty, return `Ok(FeaturePushSummary::default())`.
      - Snapshot indices of features in `component.features` where `!feature.pushed`. If empty, return `Ok(FeaturePushSummary::default())`.
      - For each unpushed feature index, invoke `run_push_feature(active, workdir, &component.id, feature)?` across every filtered adapter, incrementing `features_pushed` and recording the first non-`None` `external_id`.
      - Only after all adapter calls succeed for every unpushed feature, mark those features `pushed = true`, assign the recorded `external_id` if `Some`, and set `modified = true`.
      - Return `Ok(FeaturePushSummary { features_pushed, modified })`.
  - `src/cli.rs` (`Command::Push`):
    - Replace the inline unpushed-feature loop with a call to `adapter::push_unpushed_features(&active_adapters, root, comp)?`, accumulating `features_pushed` and persisting via `storage::save_component(root, comp)?` when `summary.modified` is true.
  - `src/tui/app.rs` (`sync_push_selected`):
    - Replace the inline unpushed-feature loop with a call to `push_unpushed_features(&active_adapters, &self.root, comp)?` on the `candidate` project clone, preserving disk-save-before-commit rollback (`BUG-0004`).

Tests:
  - In `src/adapter.rs`:
    - Unit test `push_unpushed_features_sends_to_all_adapters_and_records_first_id` using 2 mock `sh` adapters.
    - Unit test `push_unpushed_features_leaves_unpushed_on_second_adapter_error` verifying `feature.pushed` stays `false` and `external_id` stays `None` when the second adapter exits non-zero.
  - All existing CLI, TUI, and E2E multi-adapter tests must continue to pass without modification.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub fn push_unpushed_features\b' src/adapter.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 8B: Confirm-Delete Modal State and Key Handling

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 8B: Add ConfirmDelete modal state, target enum, and confirmation key handling in src/tui/modal.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/modal.rs
READ_ONLY_REFS:
  - src/tui/modal.rs:107-160 (ModalState, ModalAction)
  - src/tui/modal.rs:200-350 (ModalState::title, handle_modal_key)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/.
  - No `.unwrap()` or `.expect()` in non-test code.

[PRE_GATE]
grep -qE 'pub enum ModalState\b' src/tui/modal.rs && grep -qE 'pub enum ModalAction\b' src/tui/modal.rs

[MUTATION_SPEC]
Goal:
  Provide a pure, testable confirmation modal for deleting components or connections so destructive TUI actions require explicit confirmation (`Enter`/`y`) and can display disk-error feedback without closing.

Contract & Behavior:
  - In `src/tui/modal.rs`:
    - Define `#[derive(Debug, Clone, PartialEq, Eq)] pub enum ConfirmDeleteTarget`:
      - `Component { id: String, name: String, affected_connections: usize }`
      - `Connection { from: String, to: String }`
    - Define `#[derive(Debug, Clone, PartialEq, Eq)] pub struct ConfirmDeleteForm { pub target: ConfirmDeleteTarget, pub error: Option<String> }`.
    - Add variant `ModalState::ConfirmDelete(ConfirmDeleteForm)`.
    - Add variants to `ModalAction`:
      - `DeleteComponent { id: String }`
      - `DeleteConnection { from: String, to: String }`
    - Add constructors on `ModalState`:
      - `pub fn confirm_delete_component(id: impl Into<String>, name: impl Into<String>, affected_connections: usize) -> Self`
      - `pub fn confirm_delete_connection(from: impl Into<String>, to: impl Into<String>) -> Self`
    - Update `ModalState::title(&self)` to return `"Delete Component"` for `ConfirmDeleteTarget::Component { .. }` and `"Delete Connection"` for `ConfirmDeleteTarget::Connection { .. }`.
    - Update `handle_modal_key`:
      - For `ModalState::ConfirmDelete(form)`:
        - `KeyCode::Enter | KeyCode::Char('y' | 'Y')`: clear `form.error = None` and return `ModalAction::DeleteComponent { id }` or `ModalAction::DeleteConnection { from, to }`.
        - `KeyCode::Esc | KeyCode::Char('n' | 'N')`: return `ModalAction::Cancel`.
        - All other keys: return `ModalAction::None`.

Tests:
  - In `src/tui/modal.rs`:
    - Unit tests verifying `Enter` and `'y'` emit `ModalAction::DeleteComponent` / `ModalAction::DeleteConnection`, `'n'` and `Esc` emit `ModalAction::Cancel`, and unrelated keys emit `ModalAction::None`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -qE 'pub enum ConfirmDeleteTarget\b' src/tui/modal.rs && ! grep -rqi 'nebulous' src/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 8C: Wire Delete Component (`d`) and Delete Connection (`D`) in TUI + E2E Verification

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 8C: Wire d (delete component) and D (delete connection) keybinds, atomic rollback persistence, ConfirmDelete/Help modal rendering, and E2E tests in src/tui/app.rs, src/tui/ui.rs, and tests/e2e.rs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - src/tui/app.rs
  - src/tui/ui.rs
  - tests/e2e.rs
READ_ONLY_REFS:
  - src/tui/modal.rs (ConfirmDeleteForm, ConfirmDeleteTarget, ModalAction::DeleteComponent, ModalAction::DeleteConnection)
  - src/storage.rs:165-197 (delete_component, save_project_config)
  - src/tui/ui.rs:385-510 (ModalState::Help, render_modal)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No references to the string "nebulous" (case-insensitive) anywhere in src/ or tests/.
  - No `.unwrap()` or `.expect()` in non-test code.
  - Test fixtures must use reserved placeholder data only (`example.com`, `192.0.2.0/24`, `testuser`).

[PRE_GATE]
grep -qE 'pub enum ConfirmDeleteTarget\b' src/tui/modal.rs && grep -qE 'pub fn delete_component\b' src/storage.rs

[MUTATION_SPEC]
Goal:
  Complete TUI CRUD workflows by enabling safe, confirmation-guarded deletion of components (`d`, including cleanup of incident connections) and connections (`D`), with disk-failure rollback and unclipped 80x24 help rendering.

Contract & Behavior:
  - `src/tui/app.rs`:
    - In `InputMode::Navigate`:
      - `'d'`: if a component is selected, count `affected_connections` where `conn.from == comp.id || conn.to == comp.id`, open `ModalState::confirm_delete_component(comp.id.clone(), comp.name.clone(), affected_connections)`, set `self.mode = InputMode::Modal`, and set status `"Confirm delete component {}"`. If no component is selected, set status `"No component selected to delete"`.
      - `'D'`: if a component is selected, locate the first connection on it (preferring outgoing `conn.from == comp.id`, falling back to incoming `conn.to == comp.id`, matching `'E'`), open `ModalState::confirm_delete_connection(conn.from.clone(), conn.to.clone())`, set `self.mode = InputMode::Modal`, and set status `"Confirm delete connection {} -> {}"`. If none exists, set status `"No connection on selected component to delete"`.
    - In `apply_modal_action`:
      - `ModalAction::DeleteComponent { id }`:
        - Build `candidate_config` with all connections where `conn.from == id || conn.to == id` removed.
        - When `self.root.join(ROCKET_DIR).exists()`:
          - Delete component files via `crate::storage::delete_component(&self.root, &id)`.
          - On `Err(err)`: set `form.error = Some(msg.clone())`, `self.status_message = msg`, re-attach `self.modal = Some(modal)`, keep `self.mode = InputMode::Modal`, leave `self.project` unchanged, and return.
          - If `candidate_config.connections.len() != self.project.config.connections.len()`, persist `save_project_config(&self.root, &candidate_config)`. On `Err(err)`: best-effort re-save the deleted component via `save_component`, set `form.error = Some(msg.clone())`, re-attach modal, keep `InputMode::Modal`, leave `self.project` unchanged, and return.
        - On success: remove the component from `self.project.components`, commit `self.project.config = candidate_config`, call `self.clamp_selection()`, reset `self.detail_scroll = 0`, call `self.refresh_audit()`, set `self.mode = InputMode::Navigate`, and set status `"Deleted component {id}"`.
      - `ModalAction::DeleteConnection { from, to }`:
        - Build `candidate_config` with the `(from, to)` connection removed.
        - When `self.root.join(ROCKET_DIR).exists()`, persist `save_project_config(&self.root, &candidate_config)`. On `Err(err)`: set `form.error = Some(msg.clone())`, `self.status_message = msg`, re-attach `self.modal = Some(modal)`, keep `self.mode = InputMode::Modal`, leave `self.project.config` unchanged, and return.
        - On success: commit `self.project.config = candidate_config`, call `self.refresh_audit()`, set `self.mode = InputMode::Navigate`, and set status `"Deleted connection {from} -> {to}"`.
  - `src/tui/ui.rs`:
    - Handle `ModalState::ConfirmDelete(form)` in `render_modal`:
      - For `ConfirmDeleteTarget::Component { id, name, affected_connections }`: display `"  Delete component '{name}' ({id})?"` and when `affected_connections > 0`, a warning line noting that `{affected_connections}` connection(s) will also be removed.
      - For `ConfirmDeleteTarget::Connection { from, to }`: display `"  Delete connection '{from} -> {to}'?"`.
      - Return `form.error.as_deref()` for standard error rendering and use footer hint `"  [Enter/y] Confirm Delete  [Esc/n] Cancel"`.
    - Update `ModalState::Help` under `"  Editing & Canvas"` to document `d / D` (`"    d / D             Delete component / Delete edge"`) while keeping every Help line `<= 70` cols and total Help popup height `<= 22` rows so `80x24` terminals render unclipped with `"[Esc/?/Enter/q] Close"` visible.
  - `tests/e2e.rs`:
    - Add E2E test `e2e_tui_delete_connection_and_component_persists_to_disk` driving `'D'` + `'y'` to delete a connection, `'d'` + `'n'` to cancel component deletion, and `'d'` + `'y'` to delete a component with incident edges, verifying `.rocket/` on disk via `storage::load_project` after each step.

Tests:
  - Unit tests in `src/tui/app.rs` for delete component (including incident connection cleanup), delete connection, cancel via `Esc`/`n`, and save-failure rollback keeping modal open with `form.error`.
  - Unit tests in `src/tui/ui.rs` verifying `ModalState::ConfirmDelete` rendering and `80x24` `ModalState::Help` rendering with both `"Delete component / Delete edge"` and `"[Esc/?/Enter/q] Close"` intact.
  - E2E test in `tests/e2e.rs` passing under `cargo test --workspace`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -rqi 'nebulous' src/ tests/

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 8D: User & Adapter Documentation (`README.md`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 8D: Create README.md documenting Rocket Factory installation, CLI commands, TUI keybindings, .rocket/ workspace layout, RON themes, and external adapter contracts.

[SCOPE_LOCK]
ALLOWED_FILES:
  - README.md (NEW)
READ_ONLY_REFS:
  - Rocket Factory.md (product overview)
  - src/cli.rs (CLI subcommands and flags)
  - src/main.rs (RF_POLL_INTERVAL_SECS, default paths)
  - src/tui/ui.rs (ModalState::Help keybindings)
  - src/adapter.rs (AdaptersFile, AdapterConfig, PushFeaturePayload, PushResponse, PullStatusResponse)
  - src/theme.rs (DEFAULT_CRT_AMBER_RON, theme search paths)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.
  - No sensitive data (use only reserved placeholders: `example.com`, `192.0.2.0/24`, `testuser`).
  - No marketing, sales, or hype language; list only features, flags, keybindings, and contracts verified directly in the current codebase.

[PRE_GATE]
test ! -f README.md && grep -qE 'Delete component / Delete edge' src/tui/ui.rs

[MUTATION_SPEC]
Goal:
  Provide strictly factual, informative documentation at the repository root covering only features, commands, keybindings, file formats, and adapter contracts actually implemented and verifiable in the codebase (no sales or hype).

Contract & Behavior:
  - Create `README.md` with sections:
    1. **Overview & Quickstart**: build/install (`cargo build --release`, `cargo install --path .`), `rf init --name <name> --description <desc>`, launching TUI (`rf --path <dir>`).
    2. **CLI Reference**: `rf init`, `rf audit [--strict]`, `rf export --format <manifest|roadmap|json> [--component <id>]`, `rf push [--component <id>] [--adapter <name>]`, `rf pull [--adapter <name>]`, global flags (`--path`, `--adapters-file`), and environment variables (`RF_POLL_INTERVAL_SECS`, `$EDITOR`).
    3. **TUI Keyboard Reference**: complete table matching `ModalState::Help` (`Tab`/`Shift+Tab`, `hjkl`/Arrows, `f`, `n`/`e`/`E`, `d`/`D`, `m`/`c`, `v`, `r`/`S`, `p`, `t`, `?`, `Esc`/`q`).
    4. **Workspace Layout**: `.rocket/project.toml`, `.rocket/components/<id>.toml`, `.rocket/components/<id>.md`, and `.rocket/themes/*.ron`.
    5. **RON Themes**: search order (`.rocket/themes/`, `~/.config/rocket-factory/themes/`, built-in `crt_amber`) and minimal custom `.ron` example.
    6. **External Adapters (`adapters.toml`)**: default path (`~/.config/rocket-factory/adapters.toml`), `AdapterConfig` schema, and JSON stdin/stdout contracts for `push_component_cmd`, `push_feature_cmd`, and `pull_status_cmd`.

[POST_GATE]
test -s README.md && grep -q 'rf init' README.md && grep -q 'adapters.toml' README.md && grep -q 'Delete component' README.md && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```
