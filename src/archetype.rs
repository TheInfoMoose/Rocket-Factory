use crate::model::{Component, ComponentKind, FeatureItem, GridPos};

#[derive(Debug, Clone, Copy)]
pub struct ArchetypeSpec {
    pub kind: ComponentKind,
    pub label: &'static str,
    pub summary: &'static str,
    pub default_tags: &'static [&'static str],
    pub starter_features: &'static [&'static str],
    pub markdown_template: &'static str,
}

pub const ARCHETYPES: &[ArchetypeSpec] = &[
    ArchetypeSpec {
        kind: ComponentKind::UiView,
        label: "UI View",
        summary: "Interactive presentation layer with bounded layout regions and input handling.",
        default_tags: &["ui", "view"],
        starter_features: &[
            "Define layout and container bounds",
            "Wire state bindings and input handling",
        ],
        markdown_template: "## Layout & Regions\n\nDescribe the container hierarchy, proportions, and widgets.\n\n## State & Input\n\n- Bound state fields\n- Keybindings and focus transitions\n",
    },
    ArchetypeSpec {
        kind: ComponentKind::DataStream,
        label: "Data Stream",
        summary: "Structured ingestion, framing, and streaming data pipeline component.",
        default_tags: &["data", "stream"],
        starter_features: &[
            "Define payload schema and framing",
            "Implement producer/consumer pipeline",
        ],
        markdown_template: "## Schema & Framing\n\nSpecify message formats, boundaries, and serialization rules.\n\n## Pipeline Flow\n\n- Upstream producers\n- Backpressure and error handling\n",
    },
    ArchetypeSpec {
        kind: ComponentKind::ServiceCore,
        label: "Service Core",
        summary: "Core domain logic, state transitions, and internal API contracts.",
        default_tags: &["core", "service"],
        starter_features: &[
            "Define domain API and error contracts",
            "Implement core business logic",
        ],
        markdown_template: "## Domain Contracts\n\nDocument public types, invariants, and error conditions.\n\n## Responsibilities\n\n- Core operations\n- Collaborating components\n",
    },
    ArchetypeSpec {
        kind: ComponentKind::CliTool,
        label: "CLI Tool",
        summary: "Command-line interface with argument parsing, subcommands, and formatted output.",
        default_tags: &["cli"],
        starter_features: &[
            "Define argument parser and subcommands",
            "Implement exit codes and output formatting",
        ],
        markdown_template: "## Commands & Flags\n\nList supported subcommands, arguments, and environment variables.\n\n## Output & Exit Codes\n\n- Human-readable and machine-readable modes\n- Error reporting and exit status conventions\n",
    },
    ArchetypeSpec {
        kind: ComponentKind::Storage,
        label: "Storage",
        summary: "Durable persistence layer with schema management and atomic read/write guarantees.",
        default_tags: &["storage", "persistence"],
        starter_features: &[
            "Define persistence schema and migrations",
            "Implement atomic read/write layer",
        ],
        markdown_template: "## Persistence Schema\n\nDocument on-disk formats, directory layout, and versioning.\n\n## Durability & Safety\n\n- Atomic write strategy\n- Validation and recovery behavior\n",
    },
    ArchetypeSpec {
        kind: ComponentKind::Custom,
        label: "Custom",
        summary: "General-purpose component blueprint for specialized project responsibilities.",
        default_tags: &["custom"],
        starter_features: &["Define component responsibilities"],
        markdown_template: "## Overview\n\nDescribe the purpose, boundaries, and interfaces of this component.\n",
    },
];

pub fn archetype_for(kind: ComponentKind) -> &'static ArchetypeSpec {
    match kind {
        ComponentKind::UiView => &ARCHETYPES[0],
        ComponentKind::DataStream => &ARCHETYPES[1],
        ComponentKind::ServiceCore => &ARCHETYPES[2],
        ComponentKind::CliTool => &ARCHETYPES[3],
        ComponentKind::Storage => &ARCHETYPES[4],
        ComponentKind::Custom => &ARCHETYPES[5],
    }
}

pub fn scaffold_component(
    id: &str,
    name: &str,
    kind: ComponentKind,
    position: GridPos,
) -> Component {
    let spec = archetype_for(kind);
    let features = spec
        .starter_features
        .iter()
        .enumerate()
        .map(|(idx, title)| FeatureItem {
            id: format!("{id}_f{}", idx + 1),
            title: (*title).to_string(),
            pushed: false,
            external_id: None,
        })
        .collect();

    Component {
        id: id.to_string(),
        name: name.to_string(),
        kind,
        description: spec.summary.to_string(),
        relative_path: None,
        position,
        tags: spec
            .default_tags
            .iter()
            .map(|tag| (*tag).to_string())
            .collect(),
        tech_stack: Vec::new(),
        files: Vec::new(),
        flags: Vec::new(),
        features,
        external_tasks: Vec::new(),
        markdown_body: format!("# {name}\n\n{}", spec.markdown_template),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archetype_covers_all_kinds() {
        for kind in [
            ComponentKind::UiView,
            ComponentKind::DataStream,
            ComponentKind::ServiceCore,
            ComponentKind::CliTool,
            ComponentKind::Storage,
            ComponentKind::Custom,
        ] {
            let spec = archetype_for(kind);
            assert_eq!(spec.kind, kind);
            assert!(!spec.starter_features.is_empty());
        }
    }

    #[test]
    fn scaffold_ui_view_populates_features_and_markdown() {
        let comp = scaffold_component(
            "main_view",
            "Main View",
            ComponentKind::UiView,
            GridPos { col: 2, row: 1 },
        );
        assert_eq!(comp.id, "main_view");
        assert_eq!(comp.features.len(), 2);
        assert_eq!(comp.features[0].id, "main_view_f1");
        assert!(comp.markdown_body.starts_with("# Main View\n\n"));
        assert!(comp.tags.contains(&"ui".to_string()));
    }
}
