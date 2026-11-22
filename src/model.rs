use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct GridPos {
    pub col: u16,
    pub row: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    #[default]
    Concept,
    Interface,
    Logic,
    Data,
    Integration,
    UiView,
    DataStream,
    ServiceCore,
    CliTool,
    Storage,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    #[default]
    DependsOn,
    DataFlow,
    Renders,
    Invokes,
    Contains,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Pending,
    InProgress,
    Blocked,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardStatus {
    Gap,
    Planned,
    InProgress,
    Blocked,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalTaskRef {
    pub external_id: String,
    pub title: String,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureItem {
    pub id: String,
    pub title: String,
    pub pushed: bool,
    pub external_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub kind: ComponentKind,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub description: String,
    pub relative_path: Option<String>,
    pub position: GridPos,
    pub tags: Vec<String>,
    pub tech_stack: Vec<String>,
    pub files: Vec<String>,
    pub flags: Vec<String>,
    pub features: Vec<FeatureItem>,
    pub external_tasks: Vec<ExternalTaskRef>,
    #[serde(skip)]
    pub markdown_body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridConfig {
    pub cols: u16,
    pub rows: u16,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self { cols: 8, rows: 6 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub description: String,
    pub theme: String,
    pub active_adapters: Vec<String>,
    pub grid: GridConfig,
    pub connections: Vec<Connection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub config: ProjectConfig,
    pub components: Vec<Component>,
}

impl Project {
    pub fn component(&self, id: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.id == id)
    }

    pub fn component_mut(&mut self, id: &str) -> Option<&mut Component> {
        self.components.iter_mut().find(|c| c.id == id)
    }

    pub fn sub_components(&self, parent_id: &str) -> Vec<&Component> {
        self.components
            .iter()
            .filter(|c| c.parent_id.as_deref() == Some(parent_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_toml_roundtrip() {
        let cfg = ProjectConfig {
            name: "Rocket Factory".to_string(),
            description: "Retro-futuristic TUI planning workspace".to_string(),
            theme: "crt_amber".to_string(),
            active_adapters: vec!["tracker".to_string()],
            grid: GridConfig::default(),
            connections: vec![Connection {
                from: "stream_core".to_string(),
                to: "ui_view".to_string(),
                kind: EdgeKind::DataFlow,
                label: Some("events".into()),
            }],
        };

        let comp = Component {
            id: "ui_view".to_string(),
            name: "UI View".to_string(),
            kind: ComponentKind::UiView,
            parent_id: None,
            description: "Primary viewport".to_string(),
            relative_path: Some("src/ui.rs".to_string()),
            position: GridPos { col: 1, row: 2 },
            tags: vec!["frontend".to_string()],
            tech_stack: vec!["rust".to_string()],
            files: vec!["src/ui.rs".to_string()],
            flags: vec!["interactive".to_string()],
            features: vec![FeatureItem {
                id: "feat_1".to_string(),
                title: "Render status header".to_string(),
                pushed: true,
                external_id: Some("EXT-101".to_string()),
            }],
            external_tasks: vec![ExternalTaskRef {
                external_id: "EXT-101".to_string(),
                title: "Render status header".to_string(),
                status: TaskStatus::Complete,
            }],
            markdown_body: String::new(),
        };

        let cfg_toml = toml::to_string_pretty(&cfg).expect("serialize ProjectConfig");
        let parsed_cfg: ProjectConfig =
            toml::from_str(&cfg_toml).expect("deserialize ProjectConfig");
        assert_eq!(parsed_cfg, cfg);

        let comp_toml = toml::to_string_pretty(&comp).expect("serialize Component");
        let parsed_comp: Component = toml::from_str(&comp_toml).expect("deserialize Component");
        assert_eq!(parsed_comp, comp);
    }

    #[test]
    fn project_component_lookup() {
        let make_component = |id: &str, name: &str| Component {
            id: id.to_string(),
            name: name.to_string(),
            kind: ComponentKind::Custom,
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

        let project = Project {
            config: ProjectConfig {
                name: "Demo".to_string(),
                description: String::new(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig::default(),
                connections: Vec::new(),
            },
            components: vec![
                make_component("ui_view", "UI View"),
                make_component("stream_core", "Stream Core"),
            ],
        };

        assert!(project.component("ui_view").is_some());
        assert!(project.component("missing").is_none());
    }

    #[test]
    fn component_parent_id_toml_roundtrip_and_sub_components() {
        let mut child = Component {
            id: "child_a".to_string(),
            name: "Child A".to_string(),
            kind: ComponentKind::Concept,
            parent_id: Some("parent_a".to_string()),
            description: String::new(),
            relative_path: None,
            position: GridPos { col: 0, row: 1 },
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        };

        let toml_str = toml::to_string_pretty(&child).expect("serialize child component");
        assert!(toml_str.contains("parent_id = \"parent_a\""));
        assert!(toml_str.contains("kind = \"concept\""));
        let parsed: Component = toml::from_str(&toml_str).expect("deserialize child component");
        assert_eq!(parsed, child);

        let conn = Connection {
            from: "parent_a".to_string(),
            to: "child_a".to_string(),
            kind: EdgeKind::Contains,
            label: None,
        };
        let conn_toml = toml::to_string_pretty(&conn).expect("serialize contains edge");
        assert!(conn_toml.contains("kind = \"contains\""));
        let parsed_conn: Connection =
            toml::from_str(&conn_toml).expect("deserialize contains edge");
        assert_eq!(parsed_conn, conn);

        let parent = Component {
            id: "parent_a".to_string(),
            name: "Parent A".to_string(),
            kind: ComponentKind::Logic,
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
        child.parent_id = Some("parent_a".to_string());
        let project = Project {
            config: ProjectConfig {
                name: "Hierarchy".to_string(),
                description: String::new(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig::default(),
                connections: vec![conn],
            },
            components: vec![parent, child],
        };

        let subs = project.sub_components("parent_a");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].id, "child_a");
        assert!(project.sub_components("child_a").is_empty());
    }

    #[test]
    fn legacy_component_toml_without_parent_id_deserializes() {
        for (kind_str, expected_kind) in [
            ("ui_view", ComponentKind::UiView),
            ("data_stream", ComponentKind::DataStream),
            ("service_core", ComponentKind::ServiceCore),
            ("cli_tool", ComponentKind::CliTool),
            ("storage", ComponentKind::Storage),
            ("custom", ComponentKind::Custom),
        ] {
            let legacy_toml = format!(
                r#"
id = "legacy_node"
name = "Legacy Node"
kind = "{kind_str}"
description = "Legacy fixture"
tags = []
tech_stack = []
files = []
flags = []
features = []
external_tasks = []

[position]
col = 0
row = 0
"#
            );
            let parsed: Component =
                toml::from_str(&legacy_toml).expect("deserialize legacy component");
            assert_eq!(parsed.kind, expected_kind);
            assert_eq!(parsed.parent_id, None);
        }
    }
}
