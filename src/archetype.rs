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
        kind: ComponentKind::Concept,
        label: "Concept",
        summary: "",
        default_tags: &[],
        starter_features: &[],
        markdown_template: "## Notes\n\n",
    },
    ArchetypeSpec {
        kind: ComponentKind::Interface,
        label: "Interface",
        summary: "User-facing view, CLI, or API surface.",
        default_tags: &["interface"],
        starter_features: &[],
        markdown_template: "## Interface\n\nDescribe views, commands, or endpoints.\n",
    },
    ArchetypeSpec {
        kind: ComponentKind::Logic,
        label: "Logic",
        summary: "Core domain rules, workflows, and state transitions.",
        default_tags: &["logic"],
        starter_features: &[],
        markdown_template: "## Logic\n\nDescribe domain behavior, invariants, and rules.\n",
    },
    ArchetypeSpec {
        kind: ComponentKind::Data,
        label: "Data",
        summary: "Schemas, streams, caches, or durable storage.",
        default_tags: &["data"],
        starter_features: &[],
        markdown_template: "## Data\n\nDescribe schemas, streams, or persistence format.\n",
    },
    ArchetypeSpec {
        kind: ComponentKind::Integration,
        label: "Integration",
        summary: "External service connector, adapter, or protocol bridge.",
        default_tags: &["integration"],
        starter_features: &[],
        markdown_template:
            "## Integration\n\nDescribe external boundaries, protocols, and failure modes.\n",
    },
    ArchetypeSpec {
        kind: ComponentKind::Custom,
        label: "Custom",
        summary: "General-purpose component blueprint.",
        default_tags: &[],
        starter_features: &[],
        markdown_template: "## Overview\n\nDescribe purpose and boundaries.\n",
    },
];

pub fn archetype_for(kind: ComponentKind) -> &'static ArchetypeSpec {
    match kind {
        ComponentKind::Concept => &ARCHETYPES[0],
        ComponentKind::Interface | ComponentKind::UiView | ComponentKind::CliTool => &ARCHETYPES[1],
        ComponentKind::Logic | ComponentKind::ServiceCore => &ARCHETYPES[2],
        ComponentKind::Data | ComponentKind::DataStream | ComponentKind::Storage => &ARCHETYPES[3],
        ComponentKind::Integration => &ARCHETYPES[4],
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
        parent_id: None,
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

pub fn scaffold_sub_component(
    id: &str,
    name: &str,
    kind: ComponentKind,
    position: GridPos,
    parent_id: &str,
) -> Component {
    let mut comp = scaffold_component(id, name, kind, position);
    comp.parent_id = Some(parent_id.to_string());
    comp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archetype_covers_all_broad_and_legacy_kinds() {
        for kind in [
            ComponentKind::Concept,
            ComponentKind::Interface,
            ComponentKind::Logic,
            ComponentKind::Data,
            ComponentKind::Integration,
            ComponentKind::Custom,
        ] {
            let spec = archetype_for(kind);
            assert_eq!(spec.kind, kind);
            assert!(spec.starter_features.is_empty());
        }

        assert_eq!(
            archetype_for(ComponentKind::UiView).kind,
            ComponentKind::Interface
        );
        assert_eq!(
            archetype_for(ComponentKind::CliTool).kind,
            ComponentKind::Interface
        );
        assert_eq!(
            archetype_for(ComponentKind::ServiceCore).kind,
            ComponentKind::Logic
        );
        assert_eq!(
            archetype_for(ComponentKind::DataStream).kind,
            ComponentKind::Data
        );
        assert_eq!(
            archetype_for(ComponentKind::Storage).kind,
            ComponentKind::Data
        );
    }

    #[test]
    fn scaffold_concept_is_minimal_and_blank() {
        let comp = scaffold_component(
            "draft_idea",
            "Draft Idea",
            ComponentKind::Concept,
            GridPos { col: 2, row: 1 },
        );
        assert_eq!(comp.id, "draft_idea");
        assert_eq!(comp.kind, ComponentKind::Concept);
        assert_eq!(comp.parent_id, None);
        assert!(comp.description.is_empty());
        assert!(comp.tags.is_empty());
        assert!(comp.features.is_empty());
        assert_eq!(comp.markdown_body, "# Draft Idea\n\n## Notes\n\n");
    }

    #[test]
    fn scaffold_sub_component_sets_parent_id() {
        let comp = scaffold_sub_component(
            "child_logic",
            "Child Logic",
            ComponentKind::Logic,
            GridPos { col: 1, row: 0 },
            "parent_core",
        );
        assert_eq!(comp.id, "child_logic");
        assert_eq!(comp.kind, ComponentKind::Logic);
        assert_eq!(comp.parent_id.as_deref(), Some("parent_core"));
        assert!(comp.tags.contains(&"logic".to_string()));
        assert!(comp.features.is_empty());
    }
}
