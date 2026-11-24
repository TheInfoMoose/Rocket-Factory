use std::fmt::Write;

use serde::Serialize;

use crate::error::{RbError, RbResult};
use crate::model::{ComponentKind, EdgeKind, GridPos, Project};

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
    pub parent_id: Option<String>,
    pub position: GridPos,
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
        .filter(|conn| conn.from == component_id && conn.kind != EdgeKind::Contains)
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
        parent_id: component.parent_id.clone(),
        position: component.position,
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

fn escape_markdown_cell(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '|' => escaped.push_str("\\|"),
            '`' => escaped.push_str("\\`"),
            '\r' | '\n' => escaped.push(' '),
            other => escaped.push(other),
        }
    }
    escaped
}

pub fn export_roadmap_markdown(project: &Project) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# {} — Blueprint Roadmap\n",
        escape_markdown_cell(&project.config.name)
    );
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
                escape_markdown_cell(&component.id),
                escape_markdown_cell(&feature.id),
                escape_markdown_cell(&component.name),
                escape_markdown_cell(&feature.title),
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
            let _ = writeln!(
                out,
                "- `{}` ({})",
                escape_markdown_cell(&component.id),
                escape_markdown_cell(&component.name),
            );
        }
    }

    out
}

pub fn export_project_json(project: &Project) -> RbResult<String> {
    let mut json = serde_json::to_string_pretty(project)?;
    json.push('\n');
    Ok(json)
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

    #[test]
    fn export_manifest_includes_hierarchy_and_excludes_contains() {
        let parent = sample_component("parent_svc", "Parent Svc", ComponentKind::ServiceCore);
        let mut child = sample_component("child_svc", "Child Svc", ComponentKind::Logic);
        child.parent_id = Some("parent_svc".to_string());
        child.position = GridPos { col: 2, row: 1 };
        let leaf = sample_component("leaf_svc", "Leaf Svc", ComponentKind::Data);
        let dep_target = sample_component("dep_svc", "Dep Svc", ComponentKind::Storage);

        let project = sample_project(
            "hierarchy",
            vec![parent, child, leaf, dep_target],
            vec![
                Connection {
                    from: "child_svc".to_string(),
                    to: "leaf_svc".to_string(),
                    kind: EdgeKind::Contains,
                    label: None,
                },
                Connection {
                    from: "child_svc".to_string(),
                    to: "dep_svc".to_string(),
                    kind: EdgeKind::DependsOn,
                    label: Some("reads".to_string()),
                },
            ],
        );

        let rendered = export_component_manifest_toml(&project, "child_svc").unwrap();
        assert!(rendered.contains("parent_id = \"parent_svc\""));
        assert!(rendered.contains("[position]"));
        assert!(rendered.contains("col = 2"));
        assert!(rendered.contains("row = 1"));
        assert!(rendered.contains("target = \"dep_svc\""));
        assert!(rendered.contains("kind = \"depends_on\""));
        assert!(!rendered.contains("target = \"leaf_svc\""));
    }

    #[test]
    fn export_roadmap_escapes_markdown_cells() {
        let mut comp = sample_component(
            "edge_node",
            "Alpha | `Beta`\nGamma",
            ComponentKind::Interface,
        );
        comp.features.push(FeatureItem {
            id: "f1".to_string(),
            title: "Parse `pipe | token`\r\nsafely".to_string(),
            pushed: false,
            external_id: None,
        });
        let project = sample_project("Demo | `v1`\nPlan", vec![comp], Vec::new());

        let md = export_roadmap_markdown(&project);
        assert!(md.contains("# Demo \\| \\`v1\\` Plan — Blueprint Roadmap"));

        let table_row = md
            .lines()
            .find(|line| line.starts_with("| 1 |"))
            .expect("missing feature table row");
        assert!(table_row.contains("Alpha \\| \\`Beta\\` Gamma"));
        assert!(table_row.contains("Parse \\`pipe \\| token\\`  safely"));

        let json_str = export_project_json(&project).unwrap();
        assert!(json_str.ends_with('\n'));
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["config"]["name"], "Demo | `v1`\nPlan");
    }
}
