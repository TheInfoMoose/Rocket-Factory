use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::error::{RbError, RbResult};
use crate::export::{export_component_manifest_toml, export_project_json, export_roadmap_markdown};
use crate::model::{Component, ExternalTaskRef, FeatureItem, Project};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    TomlManifest,
    MarkdownRoadmap,
    #[default]
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub name: String,
    #[serde(default)]
    pub export_format: ExportFormat,
    pub push_component_cmd: Option<CommandSpec>,
    pub push_feature_cmd: Option<CommandSpec>,
    pub pull_status_cmd: Option<CommandSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AdaptersFile {
    #[serde(default)]
    pub adapters: Vec<AdapterConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PulledComponentStatus {
    pub component_id: String,
    #[serde(default)]
    pub tasks: Vec<ExternalTaskRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PullStatusResponse {
    #[serde(default)]
    pub components: Vec<PulledComponentStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushFeaturePayload {
    pub component_id: String,
    pub feature: FeatureItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushResponse {
    pub external_id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReconcileSummary {
    pub components_updated: usize,
    pub tasks_synced: usize,
    pub features_linked: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FeaturePushSummary {
    pub features_pushed: usize,
    pub modified: bool,
}

pub fn load_adapters_file(path: &Path) -> RbResult<AdaptersFile> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AdaptersFile::default());
        }
        Err(source) => {
            return Err(RbError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    toml::from_str(&content).map_err(|source| RbError::TomlParse {
        path: path.to_path_buf(),
        source,
    })
}

pub fn resolve_active_adapters<'a>(
    file: &'a AdaptersFile,
    active_names: &[String],
) -> RbResult<Vec<&'a AdapterConfig>> {
    let mut resolved = Vec::with_capacity(active_names.len());
    for name in active_names {
        let adapter = file
            .adapters
            .iter()
            .find(|adapter| adapter.name == *name)
            .ok_or_else(|| RbError::NotFound(format!("adapter '{name}'")))?;
        resolved.push(adapter);
    }
    Ok(resolved)
}

pub fn reconcile_status(project: &mut Project, response: &PullStatusResponse) -> ReconcileSummary {
    let mut summary = ReconcileSummary::default();

    for entry in &response.components {
        let Some(component) = project.component_mut(&entry.component_id) else {
            continue;
        };

        component.external_tasks = entry.tasks.clone();
        summary.components_updated += 1;
        summary.tasks_synced += entry.tasks.len();

        for feature in &mut component.features {
            let matched_task = match feature.external_id.as_deref() {
                Some(ext_id) => entry.tasks.iter().find(|task| task.external_id == ext_id),
                None => entry.tasks.iter().find(|task| task.title == feature.title),
            };

            if let Some(task) = matched_task {
                feature.pushed = true;
                feature.external_id = Some(task.external_id.clone());
                summary.features_linked += 1;
            }
        }
    }

    summary
}

fn run_command_with_stdin(
    spec: &CommandSpec,
    workdir: &Path,
    stdin_payload: &str,
) -> RbResult<String> {
    let mut child = Command::new(&spec.program)
        .args(&spec.args)
        .current_dir(workdir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            RbError::Adapter(format!("failed to spawn command '{}': {err}", spec.program))
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(err) = stdin.write_all(stdin_payload.as_bytes()) {
            if err.kind() != std::io::ErrorKind::BrokenPipe {
                return Err(RbError::Adapter(format!(
                    "failed to write stdin to '{}': {err}",
                    spec.program
                )));
            }
        }
    }

    let output = child.wait_with_output().map_err(|err| {
        RbError::Adapter(format!(
            "failed to wait for command '{}': {err}",
            spec.program
        ))
    })?;

    if !output.status.success() {
        return Err(RbError::Adapter(format!(
            "command '{}' exited with {}: {}",
            spec.program,
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_push_response(adapter_name: &str, stdout: &str) -> RbResult<PushResponse> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        Ok(PushResponse {
            external_id: None,
            message: None,
        })
    } else {
        serde_json::from_str(trimmed).map_err(|err| {
            RbError::Adapter(format!(
                "failed to parse push response from adapter '{adapter_name}': {err}"
            ))
        })
    }
}

pub fn run_push_component(
    adapter: &AdapterConfig,
    workdir: &Path,
    project: &Project,
    component_id: &str,
) -> RbResult<PushResponse> {
    let spec = adapter.push_component_cmd.as_ref().ok_or_else(|| {
        RbError::Adapter(format!(
            "adapter '{}' has no push_component_cmd",
            adapter.name
        ))
    })?;

    let payload = match adapter.export_format {
        ExportFormat::TomlManifest => export_component_manifest_toml(project, component_id)?,
        ExportFormat::MarkdownRoadmap => export_roadmap_markdown(project),
        ExportFormat::Json => {
            let component = project
                .component(component_id)
                .ok_or_else(|| RbError::NotFound(component_id.to_string()))?;
            serde_json::to_string_pretty(component)?
        }
    };

    let stdout = run_command_with_stdin(spec, workdir, &payload)?;
    parse_push_response(&adapter.name, &stdout)
}

pub fn run_push_feature(
    adapter: &AdapterConfig,
    workdir: &Path,
    component_id: &str,
    feature: &FeatureItem,
) -> RbResult<PushResponse> {
    let spec = adapter.push_feature_cmd.as_ref().ok_or_else(|| {
        RbError::Adapter(format!(
            "adapter '{}' has no push_feature_cmd",
            adapter.name
        ))
    })?;

    let payload = serde_json::to_string_pretty(&PushFeaturePayload {
        component_id: component_id.to_string(),
        feature: feature.clone(),
    })?;

    let stdout = run_command_with_stdin(spec, workdir, &payload)?;
    parse_push_response(&adapter.name, &stdout)
}

pub fn push_unpushed_features(
    adapters: &[&AdapterConfig],
    workdir: &Path,
    component: &mut Component,
) -> RbResult<FeaturePushSummary> {
    let feature_adapters: Vec<&AdapterConfig> = adapters
        .iter()
        .copied()
        .filter(|active| active.push_feature_cmd.is_some())
        .collect();
    if feature_adapters.is_empty() {
        return Ok(FeaturePushSummary::default());
    }

    let unpushed_indices: Vec<usize> = component
        .features
        .iter()
        .enumerate()
        .filter_map(|(idx, feature)| (!feature.pushed).then_some(idx))
        .collect();
    if unpushed_indices.is_empty() {
        return Ok(FeaturePushSummary::default());
    }

    let mut features_pushed = 0usize;
    let mut recorded_external_ids: Vec<Option<String>> = vec![None; unpushed_indices.len()];

    for (slot, &feat_idx) in unpushed_indices.iter().enumerate() {
        if let Some(feature) = component.features.get(feat_idx) {
            for active in &feature_adapters {
                let resp = run_push_feature(active, workdir, &component.id, feature)?;
                features_pushed += 1;
                if recorded_external_ids[slot].is_none() {
                    recorded_external_ids[slot] = resp.external_id;
                }
            }
        }
    }

    let mut modified = false;
    for (&feat_idx, ext_id) in unpushed_indices.iter().zip(recorded_external_ids) {
        if let Some(feature) = component.features.get_mut(feat_idx) {
            feature.pushed = true;
            if let Some(ext_id) = ext_id {
                feature.external_id = Some(ext_id);
            }
            modified = true;
        }
    }

    Ok(FeaturePushSummary {
        features_pushed,
        modified,
    })
}

pub fn run_pull_status(
    adapter: &AdapterConfig,
    workdir: &Path,
    project: &Project,
) -> RbResult<PullStatusResponse> {
    let spec = adapter.pull_status_cmd.as_ref().ok_or_else(|| {
        RbError::Adapter(format!("adapter '{}' has no pull_status_cmd", adapter.name))
    })?;

    let payload = export_project_json(project)?;
    let stdout = run_command_with_stdin(spec, workdir, &payload)?;

    serde_json::from_str(stdout.trim()).map_err(|err| {
        RbError::Adapter(format!(
            "failed to parse pull status response from adapter '{}': {err}",
            adapter.name
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ComponentKind, GridConfig, GridPos, ProjectConfig, TaskStatus};
    use tempfile::tempdir;

    #[test]
    fn load_adapters_file_missing_returns_default() {
        let dir = tempdir().unwrap();
        let file = load_adapters_file(dir.path().join("missing.toml").as_path()).unwrap();
        assert!(file.adapters.is_empty());
    }

    #[test]
    fn load_and_resolve_active_adapters() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("adapters.toml");
        let toml_content = r#"
[[adapters]]
name = "tracker_a"
export_format = "toml_manifest"

[[adapters]]
name = "tracker_b"
export_format = "json"
"#;
        std::fs::write(&path, toml_content).unwrap();

        let file = load_adapters_file(&path).unwrap();
        let active = resolve_active_adapters(&file, &["tracker_b".to_string()]).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "tracker_b");

        let err = resolve_active_adapters(&file, &["unknown".to_string()]);
        assert!(matches!(err, Err(RbError::NotFound(_))));
    }

    #[test]
    fn reconcile_status_updates_tasks_and_links_features() {
        let mut project = Project {
            config: ProjectConfig {
                name: "Demo".to_string(),
                description: String::new(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig::default(),
                connections: Vec::new(),
            },
            components: vec![Component {
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
                features: vec![FeatureItem {
                    id: "f1".to_string(),
                    title: "Render card".to_string(),
                    pushed: false,
                    external_id: None,
                }],
                external_tasks: Vec::new(),
                markdown_body: String::new(),
            }],
        };

        let response = PullStatusResponse {
            components: vec![PulledComponentStatus {
                component_id: "view_box".to_string(),
                tasks: vec![ExternalTaskRef {
                    external_id: "EXT-1".to_string(),
                    title: "Render card".to_string(),
                    status: TaskStatus::Complete,
                }],
            }],
        };

        let summary = reconcile_status(&mut project, &response);
        assert_eq!(
            summary,
            ReconcileSummary {
                components_updated: 1,
                tasks_synced: 1,
                features_linked: 1,
            }
        );
        assert_eq!(
            project.component("view_box").unwrap().external_tasks.len(),
            1
        );
        assert!(project.component("view_box").unwrap().features[0].pushed);
    }

    fn sample_project() -> Project {
        Project {
            config: ProjectConfig {
                name: "Demo".to_string(),
                description: String::new(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig::default(),
                connections: Vec::new(),
            },
            components: vec![Component {
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
                features: vec![FeatureItem {
                    id: "f1".to_string(),
                    title: "Draw box".to_string(),
                    pushed: false,
                    external_id: None,
                }],
                external_tasks: Vec::new(),
                markdown_body: String::new(),
            }],
        }
    }

    #[test]
    fn run_push_component_pipes_toml_manifest_and_parses_response() {
        let dir = tempdir().unwrap();
        let project = sample_project();
        let adapter = AdapterConfig {
            name: "mock_tracker".to_string(),
            export_format: ExportFormat::TomlManifest,
            push_component_cmd: Some(CommandSpec {
                program: "sh".into(),
                args: vec![
                    "-c".into(),
                    "cat >/dev/null && printf '{\"external_id\":\"CMP-1\",\"message\":\"ok\"}'"
                        .into(),
                ],
            }),
            push_feature_cmd: None,
            pull_status_cmd: None,
        };

        let resp = run_push_component(&adapter, dir.path(), &project, "view_box").unwrap();
        assert_eq!(resp.external_id.as_deref(), Some("CMP-1"));
        assert_eq!(resp.message.as_deref(), Some("ok"));
    }

    #[test]
    fn run_pull_status_parses_json_tasks() {
        let dir = tempdir().unwrap();
        let project = sample_project();
        let adapter = AdapterConfig {
            name: "mock_tracker".to_string(),
            export_format: ExportFormat::Json,
            push_component_cmd: None,
            push_feature_cmd: None,
            pull_status_cmd: Some(CommandSpec {
                program: "sh".into(),
                args: vec![
                    "-c".into(),
                    r#"printf '{"components":[{"component_id":"view_box","tasks":[{"external_id":"T-1","title":"Draw box","status":"complete"}]}]}'"#.into(),
                ],
            }),
        };

        let resp = run_pull_status(&adapter, dir.path(), &project).unwrap();
        assert_eq!(resp.components.len(), 1);
        assert_eq!(resp.components[0].tasks[0].status, TaskStatus::Complete);
    }

    #[test]
    fn run_command_nonzero_exit_returns_adapter_error() {
        let dir = tempdir().unwrap();
        let project = sample_project();
        let adapter = AdapterConfig {
            name: "mock_tracker".to_string(),
            export_format: ExportFormat::Json,
            push_component_cmd: None,
            push_feature_cmd: None,
            pull_status_cmd: Some(CommandSpec {
                program: "sh".into(),
                args: vec!["-c".into(), "echo boom >&2; exit 2".into()],
            }),
        };

        assert!(matches!(
            run_pull_status(&adapter, dir.path(), &project),
            Err(RbError::Adapter(msg)) if msg.contains("boom")
        ));
    }

    #[test]
    fn push_unpushed_features_sends_to_all_adapters_and_records_first_id() {
        let dir = tempdir().unwrap();
        let log_a = dir.path().join("tracker_a.log");
        let log_b = dir.path().join("tracker_b.log");
        let mut project = sample_project();
        let comp = project.component_mut("view_box").unwrap();

        let adapter_a = AdapterConfig {
            name: "tracker_a".to_string(),
            export_format: ExportFormat::Json,
            push_component_cmd: None,
            push_feature_cmd: Some(CommandSpec {
                program: "sh".into(),
                args: vec![
                    "-c".into(),
                    format!(
                        "cat >> '{}' && printf '{{\"external_id\":\"A-1\"}}'",
                        log_a.display()
                    ),
                ],
            }),
            pull_status_cmd: None,
        };
        let adapter_b = AdapterConfig {
            name: "tracker_b".to_string(),
            export_format: ExportFormat::Json,
            push_component_cmd: None,
            push_feature_cmd: Some(CommandSpec {
                program: "sh".into(),
                args: vec![
                    "-c".into(),
                    format!(
                        "cat >> '{}' && printf '{{\"external_id\":\"B-2\"}}'",
                        log_b.display()
                    ),
                ],
            }),
            pull_status_cmd: None,
        };

        let summary = push_unpushed_features(&[&adapter_a, &adapter_b], dir.path(), comp).unwrap();
        assert_eq!(
            summary,
            FeaturePushSummary {
                features_pushed: 2,
                modified: true,
            }
        );
        assert!(comp.features[0].pushed);
        assert_eq!(comp.features[0].external_id.as_deref(), Some("A-1"));

        let payload_a = std::fs::read_to_string(&log_a).unwrap();
        let payload_b = std::fs::read_to_string(&log_b).unwrap();
        assert!(payload_a.contains("Draw box"));
        assert!(payload_b.contains("Draw box"));
    }

    #[test]
    fn push_unpushed_features_leaves_unpushed_on_second_adapter_error() {
        let dir = tempdir().unwrap();
        let mut project = sample_project();
        let comp = project.component_mut("view_box").unwrap();

        let adapter_ok = AdapterConfig {
            name: "tracker_ok".to_string(),
            export_format: ExportFormat::Json,
            push_component_cmd: None,
            push_feature_cmd: Some(CommandSpec {
                program: "sh".into(),
                args: vec![
                    "-c".into(),
                    "cat >/dev/null && printf '{\"external_id\":\"A-1\"}'".into(),
                ],
            }),
            pull_status_cmd: None,
        };
        let adapter_fail = AdapterConfig {
            name: "tracker_fail".to_string(),
            export_format: ExportFormat::Json,
            push_component_cmd: None,
            push_feature_cmd: Some(CommandSpec {
                program: "sh".into(),
                args: vec!["-c".into(), "echo second failed >&2; exit 2".into()],
            }),
            pull_status_cmd: None,
        };

        let err = push_unpushed_features(&[&adapter_ok, &adapter_fail], dir.path(), comp);
        assert!(matches!(
            err,
            Err(RbError::Adapter(msg)) if msg.contains("second failed")
        ));
        assert!(!comp.features[0].pushed);
        assert!(comp.features[0].external_id.is_none());
    }
}
