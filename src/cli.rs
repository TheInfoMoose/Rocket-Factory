use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::adapter;
use crate::audit::{self, FindingSeverity};
use crate::error::{RbError, RbResult};
use crate::export;
use crate::storage;

#[derive(Debug, clap::Parser)]
#[command(
    name = "rktb",
    about = "Rocket Bench — retro-futuristic architecture planning TUI & CLI"
)]
pub struct Cli {
    #[arg(long, global = true, default_value = ".")]
    pub path: PathBuf,
    #[arg(long, global = true)]
    pub adapters_file: Option<PathBuf>,
    #[arg(long, global = true, default_value_t = false)]
    pub auto_pull: bool,
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

pub fn default_adapters_path_from_home(home: Option<&std::ffi::OsStr>) -> Option<PathBuf> {
    home.filter(|h| !h.is_empty())
        .map(|h| PathBuf::from(h).join(".config/rocket-bench/adapters.toml"))
}

pub fn default_adapters_path() -> Option<PathBuf> {
    default_adapters_path_from_home(std::env::var_os("HOME").as_deref())
}

pub fn execute_command(root: &Path, adapters_path: &Path, command: Command) -> RbResult<CliOutput> {
    match command {
        Command::Init { name, description } => {
            storage::init_workspace(root, &name, &description)?;
            Ok(CliOutput {
                stdout: format!(
                    "Initialized Rocket Bench workspace '{name}' in {}\n",
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
                        RbError::InvalidArgument(
                            "export --format manifest requires --component <id>".into(),
                        )
                    })?;
                    export::export_component_manifest_toml(&project, &id)?
                }
                ExportKind::Roadmap => {
                    if component.is_some() {
                        return Err(RbError::InvalidArgument(
                            "export --component is only supported with --format manifest".into(),
                        ));
                    }
                    export::export_roadmap_markdown(&project)
                }
                ExportKind::Json => {
                    if component.is_some() {
                        return Err(RbError::InvalidArgument(
                            "export --component is only supported with --format manifest".into(),
                        ));
                    }
                    export::export_project_json(&project)?
                }
            };

            Ok(CliOutput {
                stdout,
                exit_code: 0,
            })
        }
        Command::Push { component, adapter } => {
            let mut project = storage::load_project(root)?;
            let target_ids: Vec<String> = match component {
                Some(id) => vec![id],
                None => project.components.iter().map(|c| c.id.clone()).collect(),
            };

            let report = adapter::sync_push_project(
                &mut project,
                root,
                adapters_path,
                &target_ids,
                adapter.as_deref(),
            )?;

            let mut stdout = format!(
                "Pushed {} component(s) and {} feature(s) across {} adapter(s).\n",
                report.components_pushed, report.features_pushed, report.adapters_count
            );
            if let Some(details) = report.format_responses() {
                stdout.push_str("Adapter responses: ");
                stdout.push_str(&details);
                stdout.push('\n');
            }

            Ok(CliOutput {
                stdout,
                exit_code: 0,
            })
        }
        Command::Pull { adapter } => {
            let mut project = storage::load_project(root)?;
            let report =
                adapter::sync_pull_project(&mut project, root, adapters_path, adapter.as_deref())?;

            Ok(CliOutput {
                stdout: format!(
                    "Pulled status from {} adapter(s): {} component(s) updated, {} task(s) synced, {} feature(s) linked.\n",
                    report.adapters_count,
                    report.components_updated,
                    report.tasks_synced,
                    report.features_linked
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
        assert!(init_out
            .stdout
            .contains("Initialized Rocket Bench workspace 'apollo'"));

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

    #[test]
    fn default_adapters_path_uses_rocket_bench_config_dir() {
        let path = default_adapters_path();
        match std::env::var_os("HOME") {
            Some(home) if !home.is_empty() => {
                assert_eq!(
                    path,
                    Some(PathBuf::from(home).join(".config/rocket-bench/adapters.toml"))
                );
            }
            _ => {
                assert_eq!(path, None);
            }
        }
    }

    #[test]
    fn default_adapters_path_from_home_rejects_missing_or_empty_home() {
        use std::ffi::OsStr;

        assert_eq!(default_adapters_path_from_home(None), None);
        assert_eq!(default_adapters_path_from_home(Some(OsStr::new(""))), None);
        assert_eq!(
            default_adapters_path_from_home(Some(OsStr::new("/home/testuser"))),
            Some(PathBuf::from(
                "/home/testuser/.config/rocket-bench/adapters.toml"
            ))
        );
    }

    #[test]
    fn execute_command_pull_errors_when_adapters_path_empty() {
        let dir = tempfile::tempdir().expect("create tempdir");
        execute_command(
            dir.path(),
            Path::new(""),
            Command::Init {
                name: "apollo".into(),
                description: "Plan".into(),
            },
        )
        .expect("init workspace");

        let err = execute_command(dir.path(), Path::new(""), Command::Pull { adapter: None })
            .expect_err("pull without adapters path should fail");
        match err {
            RbError::InvalidArgument(msg) => assert!(
                msg.contains("--adapters-file"),
                "expected --adapters-file in error message, got: {msg}"
            ),
            other => panic!("expected RbError::InvalidArgument, got: {other:?}"),
        }
    }

    #[test]
    fn export_rejects_component_with_roadmap_or_json() {
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

        let roadmap_err = execute_command(
            dir.path(),
            &adapters,
            Command::Export {
                format: ExportKind::Roadmap,
                component: Some("core".into()),
            },
        )
        .expect_err("roadmap export with --component should fail");
        assert!(
            matches!(roadmap_err, RbError::InvalidArgument(_)),
            "expected RbError::InvalidArgument, got: {roadmap_err:?}"
        );

        let json_err = execute_command(
            dir.path(),
            &adapters,
            Command::Export {
                format: ExportKind::Json,
                component: Some("core".into()),
            },
        )
        .expect_err("json export with --component should fail");
        assert!(
            matches!(json_err, RbError::InvalidArgument(_)),
            "expected RbError::InvalidArgument, got: {json_err:?}"
        );
    }

    #[test]
    fn export_manifest_requires_component_returns_invalid_argument() {
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

        let err = execute_command(
            dir.path(),
            &adapters,
            Command::Export {
                format: ExportKind::Manifest,
                component: None,
            },
        )
        .expect_err("manifest export without --component should fail");
        assert!(
            matches!(err, RbError::InvalidArgument(_)),
            "expected RbError::InvalidArgument, got: {err:?}"
        );
    }

    #[test]
    fn cli_pull_push_adapter_override_and_nonstrict_audit() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let adapters_path = dir.path().join("adapters.toml");
        let log_a = dir.path().join("tracker_a.log");
        let log_b = dir.path().join("tracker_b.log");

        execute_command(
            dir.path(),
            &adapters_path,
            Command::Init {
                name: "apollo".into(),
                description: "Adapter override and non-strict audit test".into(),
            },
        )
        .expect("init workspace");

        let mut hud = scaffold_component(
            "hud",
            "HUD",
            ComponentKind::UiView,
            GridPos { col: 0, row: 0 },
        );
        hud.features.push(FeatureItem {
            id: "hud_f1".to_string(),
            title: "Render status header".to_string(),
            pushed: false,
            external_id: None,
        });
        storage::save_component(dir.path(), &hud).expect("save hud component");

        let view_box = Component {
            id: "view_box".to_string(),
            name: "View Box".to_string(),
            kind: ComponentKind::UiView,
            parent_id: None,
            description: String::new(),
            relative_path: None,
            position: GridPos { col: 1, row: 0 },
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        };
        storage::save_component(dir.path(), &view_box).expect("save view_box");

        let mut project = storage::load_project(dir.path()).expect("load project");
        project.config.active_adapters = vec!["tracker_a".to_string()];
        storage::save_project_config(dir.path(), &project.config).expect("save project config");

        let pull_json = r#"{"components":[{"component_id":"hud","tasks":[{"external_id":"B-303","title":"Render status header","status":"in_progress"}]}]}"#;
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
                    pull_status_cmd: Some(adapter::CommandSpec {
                        program: "sh".to_string(),
                        args: vec![
                            "-c".to_string(),
                            format!("echo pulled >> '{}' && printf '{{\"components\":[]}}'", log_a.display()),
                        ],
                    }),
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
                                "cat >> '{}' && printf '{{\"external_id\":\"B-303\",\"message\":\"queued\"}}'",
                                log_b.display()
                            ),
                        ],
                    }),
                    pull_status_cmd: Some(adapter::CommandSpec {
                        program: "sh".to_string(),
                        args: vec!["-c".to_string(), format!("printf '%s' '{pull_json}'")],
                    }),
                },
            ],
        };
        std::fs::write(
            &adapters_path,
            toml::to_string_pretty(&adapters_file).expect("serialize adapters.toml"),
        )
        .expect("write adapters.toml");

        let push_out = execute_command(
            dir.path(),
            &adapters_path,
            Command::Push {
                component: Some("hud".to_string()),
                adapter: Some("tracker_b".to_string()),
            },
        )
        .expect("push with adapter override");
        assert_eq!(push_out.exit_code, 0);
        assert!(push_out
            .stdout
            .contains("Pushed 0 component(s) and 1 feature(s) across 1 adapter(s)."));
        assert!(push_out.stdout.contains("Adapter responses: B-303: queued"));
        assert!(!log_a.exists());
        assert!(std::fs::read_to_string(&log_b)
            .expect("read tracker_b log")
            .contains("hud_f1"));

        let pull_out = execute_command(
            dir.path(),
            &adapters_path,
            Command::Pull {
                adapter: Some("tracker_b".to_string()),
            },
        )
        .expect("pull with adapter override");
        assert_eq!(pull_out.exit_code, 0);
        assert!(pull_out.stdout.contains(
            "Pulled status from 1 adapter(s): 1 component(s) updated, 1 task(s) synced, 1 feature(s) linked."
        ));
        assert!(!log_a.exists());

        let audit_out =
            execute_command(dir.path(), &adapters_path, Command::Audit { strict: false })
                .expect("non-strict audit workspace");
        assert_eq!(audit_out.exit_code, 0);
        assert!(audit_out.stdout.contains("[ERROR]"));
        assert!(audit_out.stdout.contains("view_box"));
    }
}
