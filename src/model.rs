use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct GridPos {
    pub col: u16,
    pub row: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    UiView,
    DataStream,
    ServiceCore,
    CliTool,
    Storage,
    #[default]
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
}
