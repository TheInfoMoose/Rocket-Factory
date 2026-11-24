use std::fmt::Write;

use serde::Serialize;

use crate::error::{RbError, RbResult};
use crate::model::{ComponentKind, EdgeKind, Project};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportDependency {
    pub target: String,
    pub kind: EdgeKind,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComponentManifestExport {
    pub id: String,
    pub name: String,
    pub component_type: String,
    pub relative_path: Option<String>,
    pub description: String,
    pub tags: Vec<String>,
    pub tech_stack: Vec<String>,
    pub files: Vec<String>,
    pub flags: Vec<String>,
    pub dependencies: Vec<ExportDependency>,
}

pub fn map_component_type(kind: ComponentKind) -> &'static str {
    match kind {
        ComponentKind::Concept => "concept",
        ComponentKind::Interface => "interface",
        ComponentKind::Logic => "logic",
        ComponentKind::Data => "data",
        ComponentKind::Integration => "integration",
        ComponentKind::UiView => "ui_crate",
        ComponentKind::DataStream => "rust_crate",
        ComponentKind::ServiceCore => "rust_crate",
        ComponentKind::CliTool => "cli_crate",
        ComponentKind::Storage => "rust_crate",
        ComponentKind::Custom => "custom",
    }
}

pub fn export_component_manifest_toml(project: &Project, component_id: &str) -> RbResult<String> {
    let component = project
        .component(component_id)
        .ok_or_else(|| RbError::NotFound(format!("component `{component_id}`")))?;

    let dependencies = project
        .config
        .connections
        .iter()
        .filter(|conn| conn.from == component_id)
        .map(|conn| ExportDependency {
            target: conn.to.clone(),
            kind: conn.kind,
            label: conn.label.clone(),
        })
        .collect();

    let manifest = ComponentManifestExport {
        id: component.id.clone(),
        name: component.name.clone(),
        component_type: map_component_type(component.kind).to_string(),
        relative_path: component.relative_path.clone(),
        description: component.description.clone(),
        tags: component.tags.clone(),
        tech_stack: component.tech_stack.clone(),
        files: component.files.clone(),
        flags: component.flags.clone(),
        dependencies,
    };

    Ok(toml::to_string_pretty(&manifest)?)
}

pub fn export_roadmap_markdown(project: &Project) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# {} — Blueprint Roadmap\n", project.config.name);
    let _ = writeln!(out, "## Up Next\n");
    let _ = writeln!(out, "| # | Source Ref | Component | Feature | Status |");
    let _ = writeln!(out, "|---|---|---|---|---|");

    let mut row_idx = 1usize;
    for component in &project.components {
        for feature in &component.features {
            let status = if feature.pushed { "pushed" } else { "planned" };
            let _ = writeln!(
                out,
                "| {row_idx} | FEAT {}:{} | {} | {} | {status} |",
                component.id, feature.id, component.name, feature.title
            );
            row_idx += 1;
        }
    }

    let _ = writeln!(out, "\n## Components Without Tasks\n");
    let gaps: Vec<_> = project
        .components
        .iter()
        .filter(|c| c.features.is_empty() && c.external_tasks.is_empty())
        .collect();

    if gaps.is_empty() {
        let _ = writeln!(out, "_None_");
    } else {
        for component in gaps {
            let _ = writeln!(out, "- `{}` ({})", component.id, component.name);
        }
    }

    out
}

pub fn export_project_json(project: &Project) -> RbResult<String> {
    Ok(serde_json::to_string_pretty(project)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Component, Connection, FeatureItem, GridConfig, GridPos, ProjectConfig};

    fn sample_component(id: &str, name: &str, kind: ComponentKind) -> Component {
        Component {
            id: id.to_string(),
            name: name.to_string(),
            kind,
            parent_id: None,
            description: format!("{name} description"),
            relative_path: Some(format!("crates/{id}")),
            position: GridPos::default(),
            tags: Vec::new(),
            tech_stack: vec!["rust".to_string()],
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        }
    }

    fn sample_project(
        name: &str,
        components: Vec<Component>,
        connections: Vec<Connection>,
    ) -> Project {
        Project {
            config: ProjectConfig {
                name: name.to_string(),
                description: "Demo project".to_string(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig::default(),
                connections,
            },
            components,
        }
    }

    #[test]
    fn export_manifest_toml_includes_dependencies_and_type() {
        let project = sample_project(
            "demo",
            vec![
                sample_component("ui_box", "UI Box", ComponentKind::UiView),
                sample_component("core_bus", "Core Bus", ComponentKind::DataStream),
            ],
            vec![Connection {
                from: "ui_box".to_string(),
                to: "core_bus".to_string(),
                kind: EdgeKind::DataFlow,
                label: Some("events".to_string()),
            }],
        );

        let rendered = export_component_manifest_toml(&project, "ui_box").unwrap();
        assert!(rendered.contains("component_type = \"ui_crate\""));
        assert!(rendered.contains("id = \"ui_box\""));
        assert!(rendered.contains("target = \"core_bus\""));
    }

    #[test]
    fn export_roadmap_markdown_lists_features_and_gaps() {
        let mut core_bus = sample_component("core_bus", "Core Bus", ComponentKind::DataStream);
        core_bus.features.push(FeatureItem {
            id: "f1".to_string(),
            title: "Stream events".to_string(),
            pushed: false,
            external_id: None,
        });
        let view_box = sample_component("view_box", "View Box", ComponentKind::UiView);
        let project = sample_project("demo", vec![core_bus, view_box], Vec::new());

        let md = export_roadmap_markdown(&project);
        assert!(md.contains("FEAT core_bus:f1"));
        assert!(md.contains("Stream events"));

        let gaps_section = md
            .split("## Components Without Tasks")
            .nth(1)
            .expect("missing Components Without Tasks section");
        assert!(gaps_section.contains("view_box"));
        assert!(!gaps_section.contains("core_bus"));
    }

    #[test]
    fn export_project_json_valid_json() {
        let project = sample_project(
            "demo",
            vec![sample_component("ui_box", "UI Box", ComponentKind::UiView)],
            Vec::new(),
        );

        let json_str = export_project_json(&project).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(val["config"]["name"], "demo");
    }
}
