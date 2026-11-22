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

            for comp_id in &target_ids {
                for active in &active_adapters {
                    if active.push_component_cmd.is_some() {
                        adapter::run_push_component(active, root, &project, comp_id)?;
                        components_pushed += 1;
                    }
                }

                if let Some(comp) = project.component_mut(comp_id) {
                    let summary = adapter::push_unpushed_features(&active_adapters, root, comp)?;
                    features_pushed += summary.features_pushed;
                    if summary.modified {
                        storage::save_component(root, comp)?;
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
    use crate::model::{Component, ComponentKind, FeatureItem, GridPos};

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

        let mut hud = scaffold_component("hud", "HUD", ComponentKind::UiView, GridPos::default());
        hud.features.push(FeatureItem {
            id: "hud_f1".to_string(),
            title: "Render status header".to_string(),
            pushed: false,
            external_id: None,
        });
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
            parent_id: None,
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

    #[test]
    fn cli_push_sends_unpushed_features_to_all_active_adapters() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let adapters_path = dir.path().join("adapters.toml");
        let log_a = dir.path().join("tracker_a.log");
        let log_b = dir.path().join("tracker_b.log");

        execute_command(
            dir.path(),
            &adapters_path,
            Command::Init {
                name: "apollo".into(),
                description: "Multi-adapter CLI push test".into(),
            },
        )
        .expect("init workspace");

        let mut comp = scaffold_component("hud", "HUD", ComponentKind::UiView, GridPos::default());
        comp.features.push(FeatureItem {
            id: "hud_f1".to_string(),
            title: "Render status header".to_string(),
            pushed: false,
            external_id: None,
        });
        storage::save_component(dir.path(), &comp).expect("save hud component");

        let mut project = storage::load_project(dir.path()).expect("load project");
        project.config.active_adapters = vec!["tracker_a".to_string(), "tracker_b".to_string()];
        storage::save_project_config(dir.path(), &project.config).expect("save project config");

        let adapters_file = adapter::AdaptersFile {
            adapters: vec![
                adapter::AdapterConfig {
                    name: "tracker_a".to_string(),
                    export_format: adapter::ExportFormat::Json,
                    push_component_cmd: None,
                    push_feature_cmd: Some(adapter::CommandSpec {
                        program: "sh".to_string(),
                        args: vec![
                            "-c".to_string(),
                            format!(
                                "cat >> '{}' && printf '{{\"external_id\":\"A-101\"}}'",
                                log_a.display()
                            ),
                        ],
                    }),
                    pull_status_cmd: None,
                },
                adapter::AdapterConfig {
                    name: "tracker_b".to_string(),
                    export_format: adapter::ExportFormat::Json,
                    push_component_cmd: None,
                    push_feature_cmd: Some(adapter::CommandSpec {
                        program: "sh".to_string(),
                        args: vec![
                            "-c".to_string(),
                            format!(
                                "cat >> '{}' && printf '{{\"external_id\":\"B-202\"}}'",
                                log_b.display()
                            ),
                        ],
                    }),
                    pull_status_cmd: None,
                },
            ],
        };
        std::fs::write(
            &adapters_path,
            toml::to_string_pretty(&adapters_file).expect("serialize adapters.toml"),
        )
        .expect("write adapters.toml");

        let out = execute_command(
            dir.path(),
            &adapters_path,
            Command::Push {
                component: Some("hud".to_string()),
                adapter: None,
            },
        )
        .expect("push across two adapters");
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("2 feature(s) across 2 adapter(s)"));

        let payload_a = std::fs::read_to_string(&log_a).expect("read tracker_a log");
        let payload_b = std::fs::read_to_string(&log_b).expect("read tracker_b log");
        assert!(payload_a.contains("hud_f1"));
        assert!(payload_b.contains("hud_f1"));

        let reloaded = storage::load_project(dir.path()).expect("reload project");
        let hud = reloaded.component("hud").expect("hud exists");
        assert!(hud.features[0].pushed);
        assert_eq!(hud.features[0].external_id.as_deref(), Some("A-101"));
    }
}
