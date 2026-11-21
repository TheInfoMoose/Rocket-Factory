use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::adapter;
use crate::audit::{self, FindingSeverity};
use crate::error::{RfError, RfResult};
use crate::export;
use crate::storage;

#[derive(Debug, clap::Parser)]
#[command(
    name = "rf",
    about = "Rocket Factory — retro-futuristic architecture planning TUI & CLI"
)]
pub struct Cli {
    #[arg(long, global = true, default_value = ".")]
    pub path: PathBuf,
    #[arg(long, global = true)]
    pub adapters_file: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ExportKind {
    Manifest,
    Roadmap,
    Json,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    Init {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "")]
        description: String,
    },
    Audit {
        #[arg(long)]
        strict: bool,
    },
    Export {
        #[arg(long, value_enum)]
        format: ExportKind,
        #[arg(long)]
        component: Option<String>,
    },
    Push {
        #[arg(long)]
        component: Option<String>,
        #[arg(long)]
        adapter: Option<String>,
    },
    Pull {
        #[arg(long)]
        adapter: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    pub stdout: String,
    pub exit_code: i32,
}

pub fn default_adapters_path() -> PathBuf {
    match std::env::var_os("HOME") {
        Some(home) if !home.is_empty() => {
            PathBuf::from(home).join(".config/rocket-factory/adapters.toml")
        }
        _ => PathBuf::from(".rocket/adapters.toml"),
    }
}

pub fn execute_command(root: &Path, adapters_path: &Path, command: Command) -> RfResult<CliOutput> {
    match command {
        Command::Init { name, description } => {
            storage::init_workspace(root, &name, &description)?;
            Ok(CliOutput {
                stdout: format!(
                    "Initialized Rocket Factory workspace '{name}' in {}\n",
                    root.display()
                ),
                exit_code: 0,
            })
        }
        Command::Audit { strict } => {
            let project = storage::load_project(root)?;
            let findings = audit::audit_project(&project);

            let stdout = if findings.is_empty() {
                "No audit findings.\n".to_string()
            } else {
                let mut out = String::new();
                for finding in &findings {
                    let prefix = match finding.severity {
                        FindingSeverity::Error => "[ERROR]",
                        FindingSeverity::Warning => "[WARN]",
                    };
                    let _ = writeln!(out, "{prefix} {}", finding.message);
                }
                out
            };

            let has_error = findings
                .iter()
                .any(|finding| finding.severity == FindingSeverity::Error);
            let exit_code = if strict && has_error { 1 } else { 0 };

            Ok(CliOutput { stdout, exit_code })
        }
        Command::Export { format, component } => {
            let project = storage::load_project(root)?;
            let stdout = match format {
                ExportKind::Manifest => {
                    let id = component.ok_or_else(|| {
                        RfError::NotFound(
                            "export --format manifest requires --component <id>".into(),
                        )
                    })?;
                    export::export_component_manifest_toml(&project, &id)?
                }
                ExportKind::Roadmap => export::export_roadmap_markdown(&project),
                ExportKind::Json => export::export_project_json(&project)?,
            };

            Ok(CliOutput {
                stdout,
                exit_code: 0,
            })
        }
        Command::Push { component, adapter } => {
            let mut project = storage::load_project(root)?;
            let adapters_file = adapter::load_adapters_file(adapters_path)?;

            let adapter_names: Vec<String> = match adapter {
                Some(name) => vec![name],
                None => project.config.active_adapters.clone(),
            };
            let active_adapters = adapter::resolve_active_adapters(&adapters_file, &adapter_names)?;

            let target_ids: Vec<String> = match component {
                Some(id) => {
                    if project.component(&id).is_none() {
                        return Err(RfError::NotFound(format!("component '{id}'")));
                    }
                    vec![id]
                }
                None => project.components.iter().map(|c| c.id.clone()).collect(),
            };

            let mut components_pushed = 0usize;
            let mut features_pushed = 0usize;

            for active in &active_adapters {
                for comp_id in &target_ids {
                    if active.push_component_cmd.is_some() {
                        adapter::run_push_component(active, root, &project, comp_id)?;
                        components_pushed += 1;
                    }

                    if active.push_feature_cmd.is_some() {
                        let mut modified = false;
                        if let Some(comp) = project.component_mut(comp_id) {
                            for feature in &mut comp.features {
                                if !feature.pushed {
                                    let resp =
                                        adapter::run_push_feature(active, root, comp_id, feature)?;
                                    feature.pushed = true;
                                    if let Some(ext_id) = resp.external_id {
                                        feature.external_id = Some(ext_id);
                                    }
                                    modified = true;
                                    features_pushed += 1;
                                }
                            }
                        }
                        if modified {
                            if let Some(comp) = project.component(comp_id) {
                                storage::save_component(root, comp)?;
                            }
                        }
                    }
                }
            }

            Ok(CliOutput {
                stdout: format!(
                    "Pushed {components_pushed} component(s) and {features_pushed} feature(s) across {} adapter(s).\n",
                    active_adapters.len()
                ),
                exit_code: 0,
            })
        }
        Command::Pull { adapter } => {
            let mut project = storage::load_project(root)?;
            let adapters_file = adapter::load_adapters_file(adapters_path)?;

            let adapter_names: Vec<String> = match adapter {
                Some(name) => vec![name],
                None => project.config.active_adapters.clone(),
            };
            let active_adapters = adapter::resolve_active_adapters(&adapters_file, &adapter_names)?;

            let mut components_updated = 0usize;
            let mut tasks_synced = 0usize;
            let mut features_linked = 0usize;

            for active in &active_adapters {
                if active.pull_status_cmd.is_some() {
                    let resp = adapter::run_pull_status(active, root, &project)?;
                    let summary = adapter::reconcile_status(&mut project, &resp);
                    components_updated += summary.components_updated;
                    tasks_synced += summary.tasks_synced;
                    features_linked += summary.features_linked;

                    for entry in &resp.components {
                        if let Some(comp) = project.component(&entry.component_id) {
                            storage::save_component(root, comp)?;
                        }
                    }
                }
            }

            Ok(CliOutput {
                stdout: format!(
                    "Pulled status from {} adapter(s): {components_updated} component(s) updated, {tasks_synced} task(s) synced, {features_linked} feature(s) linked.\n",
                    active_adapters.len()
                ),
                exit_code: 0,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archetype::scaffold_component;
    use crate::model::{Component, ComponentKind, GridPos};

    #[test]
    fn cli_init_and_export_roadmap() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let adapters = dir.path().join("adapters.toml");

        let init_out = execute_command(
            dir.path(),
            &adapters,
            Command::Init {
                name: "apollo".into(),
                description: "Plan".into(),
            },
        )
        .expect("init workspace");
        assert_eq!(init_out.exit_code, 0);

        let hud = scaffold_component("hud", "HUD", ComponentKind::UiView, GridPos::default());
        storage::save_component(dir.path(), &hud).expect("save hud component");

        let out = execute_command(
            dir.path(),
            &adapters,
            Command::Export {
                format: ExportKind::Roadmap,
                component: None,
            },
        )
        .expect("export roadmap");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("# apollo — Blueprint Roadmap"));
        assert!(out.stdout.contains("hud_f1"));
    }

    #[test]
    fn cli_audit_strict_exits_nonzero_on_gap() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let adapters = dir.path().join("adapters.toml");

        execute_command(
            dir.path(),
            &adapters,
            Command::Init {
                name: "apollo".into(),
                description: "Plan".into(),
            },
        )
        .expect("init workspace");

        let view_box = Component {
            id: "view_box".to_string(),
            name: "View Box".to_string(),
            kind: ComponentKind::UiView,
            description: String::new(),
            relative_path: None,
            position: GridPos::default(),
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        };
        storage::save_component(dir.path(), &view_box).expect("save view_box");

        let out = execute_command(dir.path(), &adapters, Command::Audit { strict: true })
            .expect("audit workspace");
        assert_eq!(out.exit_code, 1);
        assert!(out.stdout.contains("view_box"));
    }
}
