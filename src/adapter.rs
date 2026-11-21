use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{RfError, RfResult};
use crate::model::{ExternalTaskRef, FeatureItem, Project};

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

pub fn load_adapters_file(path: &Path) -> RfResult<AdaptersFile> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AdaptersFile::default());
        }
        Err(source) => {
            return Err(RfError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    toml::from_str(&content).map_err(|source| RfError::TomlParse {
        path: path.to_path_buf(),
        source,
    })
}

pub fn resolve_active_adapters<'a>(
    file: &'a AdaptersFile,
    active_names: &[String],
) -> RfResult<Vec<&'a AdapterConfig>> {
    let mut resolved = Vec::with_capacity(active_names.len());
    for name in active_names {
        let adapter = file
            .adapters
            .iter()
            .find(|adapter| adapter.name == *name)
            .ok_or_else(|| RfError::NotFound(format!("adapter '{name}'")))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Component, ComponentKind, GridConfig, GridPos, ProjectConfig, TaskStatus};
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
        assert!(matches!(err, Err(RfError::NotFound(_))));
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
}
