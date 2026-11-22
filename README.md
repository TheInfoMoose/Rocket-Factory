# Rocket Factory (`rf`)

Rocket Factory is a Rust terminal user interface (`ratatui`) and CLI tool for planning application architectures using `.rocket/` TOML metadata files, Markdown component specifications, and external command-based adapters.

## Prerequisites

- **Rust** (stable toolchain, installed via [rustup](https://rustup.rs/))
- **Git** (`git` CLI)

## Installation

### Install directly from Git (recommended)

```bash
cargo install --git https://github.com/TheInfoMoose/Rocket-Factory.git --locked
```

### Build and install from a local clone

```bash
git clone https://github.com/TheInfoMoose/Rocket-Factory.git
cd Rocket-Factory

# Install `rf` into ~/.cargo/bin
cargo install --path . --locked

# Or build the release binary without installing (target/release/rf)
cargo build --release --locked
```

### Verify installation & PATH

`cargo install` places the `rf` binary in `~/.cargo/bin`. Ensure `$HOME/.cargo/bin` is on your `PATH`, then verify the installation:

```bash
rf --help
```

## Quick Start

1. **Initialize a workspace** (creates `./my-project/.rocket/` automatically):

   ```bash
   rf --path ./my-project init --name "my-project" --description "Service architecture blueprint"
   ```

   Or initialize inside an existing project directory (`--path` defaults to `.`):

   ```bash
   rf init --name "my-project"
   ```

2. **Launch the interactive TUI** (requires an initialized `.rocket/` workspace — run `rf init` first):

   ```bash
   rf --path ./my-project
   ```

   Or run `rf` directly inside the workspace directory.

3. **Explore the TUI & verify via CLI**:

   - Press `n` to add a component, `?` for keyboard help, and `q` to quit.
   - Optionally run a CLI audit on the workspace:

     ```bash
     rf --path ./my-project audit
     ```

## CLI Reference

Running `rf` without a subcommand opens the interactive TUI.

### Global Flags

| Flag | Default | Description |
| :--- | :--- | :--- |
| `--path <DIR>` | `.` | Workspace root directory containing `.rocket/`. |
| `--adapters-file <PATH>` | `~/.config/rocket-factory/adapters.toml` | Path to the external adapters configuration file. Falls back to `.rocket/adapters.toml` if `$HOME` is unset or empty. |

### Subcommands

| Command | Description |
| :--- | :--- |
| `rf init --name <NAME> [--description <DESC>]` | Initializes `.rocket/project.toml` and `.rocket/components/` in the workspace root. Fails if `.rocket/project.toml` already exists. `--description` defaults to `""`. |
| `rf audit [--strict]` | Audits the workspace and prints `[ERROR]` and `[WARN]` findings (or `No audit findings.`). With `--strict`, exits with status `1` if any `[ERROR]` finding is present. |
| `rf export --format <manifest\|roadmap\|json> [--component <ID>]` | Exports workspace data to stdout: `manifest` exports a single component's TOML manifest (requires `--component <ID>`), `roadmap` exports a Markdown roadmap, and `json` exports the full project as JSON. |
| `rf push [--component <ID>] [--adapter <NAME>]` | Pushes component(s) via `push_component_cmd` and unpushed features via `push_feature_cmd`. Targets a single component if `--component` is given, otherwise all components. Uses `--adapter` if given, otherwise `active_adapters` from `.rocket/project.toml`. Persists updated `pushed` and `external_id` fields on features. |
| `rf pull [--adapter <NAME>]` | Pulls external task status via `pull_status_cmd` from `--adapter` or all `active_adapters`, reconciles `external_tasks` and feature links, and saves updated component files. |

### Environment Variables

| Variable | Behavior |
| :--- | :--- |
| `RF_POLL_INTERVAL_SECS` | Positive integer interval in seconds for automatic `pull` sync in the TUI when `active_adapters` is non-empty and no modal is open. Disabled when unset, invalid, or `0`. |
| `EDITOR` | External editor command launched when pressing `v` in the TUI to edit `.rocket/components/<id>.md`. Split on ASCII whitespace into program and arguments; falls back to `vi` when unset or blank. |

## TUI Keyboard Reference

The TUI layout consists of a header bar, three columns (Component List, Snap Grid Canvas, and Component Details), and a footer bar.

### Navigation

| Key | Action |
| :--- | :--- |
| `Tab` / `Shift+Tab` | Cycle focus (List / Canvas / Detail) |
| `hjkl` / Arrows | Move selection / Scroll detail |
| `f` | Toggle Gaps Only filter |

### Editing & Canvas

| Key | Action |
| :--- | :--- |
| `n` / `e` / `E` | New component / Edit component / Edit edge |
| `d` / `D` | Delete component / Delete edge |
| `m` / `c` | Move card mode / Connect components |
| `v` | Open component Markdown in `$EDITOR` |

### Sync & Display

| Key | Action |
| :--- | :--- |
| `r` / `S` | Pull status from active adapters |
| `p` | Push selected component & features |
| `t` | Switch RON theme |

### General

| Key | Action |
| :--- | :--- |
| `?` | Toggle this keyboard help |
| `Esc` / `q` | Cancel mode / Quit |

### Modal Controls

- **Component / Connection forms (`n`, `e`, `E`)**: `Tab` switches fields, `←`/`→` cycles enum options, `Enter` saves, `Esc` cancels.
- **Delete confirmation (`d`, `D`)**: `Enter` or `y` confirms deletion, `Esc` or `n` cancels.
- **Theme switcher (`t`)**: `↑`/`↓` selects a theme, `Enter` applies and saves to `.rocket/project.toml`, `Esc` cancels.
- **Keyboard help (`?`)**: `Esc`, `?`, `Enter`, or `q` closes the modal.

## Workspace Layout

```text
<workspace>/
└── .rocket/
    ├── project.toml
    ├── components/
    │   ├── <id>.toml
    │   └── <id>.md
    └── themes/
        └── <theme_name>.ron
```

- **`.rocket/project.toml`**: Stores `ProjectConfig` — `name`, `description`, `theme` (default `"crt_amber"`), `active_adapters` (list of adapter names from `adapters.toml`), `grid` (`cols` and `rows`, default `8` × `6`), and `connections` (`from`, `to`, `kind`: `depends_on` | `data_flow` | `renders` | `invokes` | `custom`, optional `label`).
- **`.rocket/components/<id>.toml`**: Stores `Component` metadata — `id` (1–64 characters from `[a-z0-9_-]`, cannot start with `-`), `name`, `kind` (`ui_view` | `data_stream` | `service_core` | `cli_tool` | `storage` | `custom`), `description`, optional `relative_path`, `position` (`col`, `row`), `tags`, `tech_stack`, `files`, `flags`, `features` (`id`, `title`, `pushed`, `external_id`), and `external_tasks` (`external_id`, `title`, `status`: `pending` | `in_progress` | `blocked` | `complete`).
- **`.rocket/components/<id>.md`**: Markdown specification body for component `<id>`, rendered in the Component Details panel and edited with `v`.
- **`.rocket/themes/*.ron`**: Optional workspace-local RON theme files.

## RON Themes

### Theme Search Order

When loading a theme named `<name>`, Rocket Factory searches in the following order:

1. `<workspace>/.rocket/themes/<name>.ron`
2. `~/.config/rocket-factory/themes/<name>.ron`
3. Built-in `crt_amber` (fallback when `<name>` is `"crt_amber"` and no override file exists)

Theme names may only contain ASCII alphanumerics, `_`, and `-`. Symlinks resolving outside the theme directory are rejected.

### Custom `.ron` Theme Example

Create `.rocket/themes/custom_amber.ron` (or `~/.config/rocket-factory/themes/custom_amber.ron`) with all 16 RGB palette fields:

```ron
(
    name: "custom_amber",
    palette: (
        background: (r: 10, g: 10, b: 8),
        surface: (r: 20, g: 18, b: 14),
        foreground: (r: 255, g: 176, b: 0),
        muted: (r: 153, g: 107, b: 20),
        accent: (r: 255, g: 196, b: 61),
        border_default: (r: 120, g: 82, b: 16),
        border_focused: (r: 255, g: 209, b: 102),
        status_gap: (r: 140, g: 100, b: 36),
        status_planned: (r: 214, g: 158, b: 46),
        status_in_progress: (r: 255, g: 196, b: 61),
        status_blocked: (r: 255, g: 82, b: 82),
        status_complete: (r: 64, g: 224, b: 112),
        md_heading: (r: 255, g: 218, b: 121),
        md_code: (r: 94, g: 212, b: 212),
        alert_warn: (r: 255, g: 159, b: 28),
        alert_error: (r: 255, g: 82, b: 82),
    ),
)
```

## External Adapters (`adapters.toml`)

Rocket Factory integrates with external tools by spawning configured commands and exchanging payloads over `stdin` and `stdout`.

### Configuration Path and Schema

By default, adapters are loaded from `~/.config/rocket-factory/adapters.toml` (overridable via `--adapters-file <PATH>`). Enable adapters for a workspace by listing their `name` values under `active_adapters` in `.rocket/project.toml`.

```toml
[[adapters]]
name = "example_tracker"
export_format = "json" # "toml_manifest" | "markdown_roadmap" | "json" (default: "json")

[adapters.push_component_cmd]
program = "/usr/local/bin/example-tracker-adapter"
args = ["push-component"]

[adapters.push_feature_cmd]
program = "/usr/local/bin/example-tracker-adapter"
args = ["push-feature"]

[adapters.pull_status_cmd]
program = "/usr/local/bin/example-tracker-adapter"
args = ["pull-status"]
```

Each command is executed with the workspace root as its working directory. Non-zero exit codes fail the operation and surface `stderr`.

### Command Stdin/Stdout Contracts

#### `push_component_cmd`

- **stdin**: Formatted according to the adapter's `export_format`:
  - `toml_manifest`: Single component TOML manifest (same as `rf export --format manifest --component <id>`).
  - `markdown_roadmap`: Full project Markdown roadmap (same as `rf export --format roadmap`).
  - `json`: Pretty-printed JSON representation of the target `Component`.
- **stdout**: Empty output, or a `PushResponse` JSON object (both fields optional):

```json
{
  "external_id": "CMP-101",
  "message": "synced"
}
```

#### `push_feature_cmd`

- **stdin**: `PushFeaturePayload` JSON object:

```json
{
  "component_id": "ui_view",
  "feature": {
    "id": "ui_view_f1",
    "title": "Render status header",
    "pushed": false,
    "external_id": null
  }
}
```

- **stdout**: Empty output, or a `PushResponse` JSON object:

```json
{
  "external_id": "TASK-204",
  "message": "created"
}
```

When multiple active adapters define `push_feature_cmd`, each unpushed feature is sent to every such adapter; if all succeed, the feature is marked `pushed = true` and records the first non-null `external_id` returned.

#### `pull_status_cmd`

- **stdin**: Full `Project` serialized as JSON (same as `rf export --format json`).
- **stdout**: `PullStatusResponse` JSON object:

```json
{
  "components": [
    {
      "component_id": "ui_view",
      "tasks": [
        {
          "external_id": "TASK-204",
          "title": "Render status header",
          "status": "complete"
        }
      ]
    }
  ]
}
```

Valid task `status` values are `"pending"`, `"in_progress"`, `"blocked"`, and `"complete"`. During reconciliation, each matched component's `external_tasks` are replaced with the returned `tasks`, and features are linked (`pushed = true`, `external_id` populated) by matching `external_id` or exact `title`.
