use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::archetype::{archetype_for, ArchetypeSpec, ARCHETYPES};
use crate::model::{Component, ComponentKind, EdgeKind};
use crate::storage::validate_component_id;

pub const EDGE_KINDS: &[EdgeKind] = &[
    EdgeKind::DependsOn,
    EdgeKind::DataFlow,
    EdgeKind::Renders,
    EdgeKind::Invokes,
    EdgeKind::Contains,
    EdgeKind::Custom,
];

pub fn edge_kind_label(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::DependsOn => "depends_on",
        EdgeKind::DataFlow => "data_flow",
        EdgeKind::Renders => "renders",
        EdgeKind::Invokes => "invokes",
        EdgeKind::Contains => "contains",
        EdgeKind::Custom => "custom",
    }
}

pub fn edge_relationship_preview(from: &str, to: &str, kind: EdgeKind) -> String {
    match kind {
        EdgeKind::DependsOn => format!("{from} depends on {to}"),
        EdgeKind::DataFlow => format!("{from} sends data to {to}"),
        EdgeKind::Renders => format!("{from} renders {to}"),
        EdgeKind::Invokes => format!("{from} invokes {to}"),
        EdgeKind::Contains => format!("{from} contains {to} (sub-component)"),
        EdgeKind::Custom => format!("{from} -> {to}"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewComponentForm {
    pub id: String,
    pub name: String,
    pub archetype_index: usize,
    pub parent_id: Option<String>,
    pub active_field: usize,
    pub error: Option<String>,
}

impl NewComponentForm {
    pub const FIELD_COUNT: usize = 3;

    pub fn selected_archetype(&self) -> &'static ArchetypeSpec {
        ARCHETYPES
            .get(self.archetype_index)
            .unwrap_or(&ARCHETYPES[0])
    }

    fn active_text_mut(&mut self) -> Option<&mut String> {
        match self.active_field {
            0 => Some(&mut self.id),
            1 => Some(&mut self.name),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditComponentForm {
    pub component_id: String,
    pub name: String,
    pub archetype_index: usize,
    pub description: String,
    pub relative_path: String,
    pub tags: String,
    pub new_feature: String,
    pub new_sub_component: String,
    pub active_field: usize,
    pub error: Option<String>,
}

impl EditComponentForm {
    pub const FIELD_COUNT: usize = 7;

    pub fn selected_archetype(&self) -> &'static ArchetypeSpec {
        ARCHETYPES
            .get(self.archetype_index)
            .unwrap_or(&ARCHETYPES[0])
    }

    fn active_text_mut(&mut self) -> Option<&mut String> {
        match self.active_field {
            0 => Some(&mut self.name),
            2 => Some(&mut self.description),
            3 => Some(&mut self.relative_path),
            4 => Some(&mut self.tags),
            5 => Some(&mut self.new_feature),
            6 => Some(&mut self.new_sub_component),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditConnectionForm {
    pub from: String,
    pub to: String,
    pub kind_index: usize,
    pub label: String,
    pub active_field: usize,
    pub error: Option<String>,
}

impl EditConnectionForm {
    pub const FIELD_COUNT: usize = 2;

    pub fn selected_kind(&self) -> EdgeKind {
        EDGE_KINDS
            .get(self.kind_index)
            .copied()
            .unwrap_or(EdgeKind::DependsOn)
    }

    fn active_text_mut(&mut self) -> Option<&mut String> {
        match self.active_field {
            1 => Some(&mut self.label),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeSwitcherForm {
    pub themes: Vec<String>,
    pub selected_index: usize,
    pub error: Option<String>,
}

impl ThemeSwitcherForm {
    pub fn selected_theme(&self) -> Option<&str> {
        self.themes.get(self.selected_index).map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmDeleteTarget {
    Component {
        id: String,
        name: String,
        affected_connections: usize,
    },
    Connection {
        from: String,
        to: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmDeleteForm {
    pub target: ConfirmDeleteTarget,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalState {
    NewComponent(NewComponentForm),
    EditComponent(EditComponentForm),
    EditConnection(EditConnectionForm),
    Help,
    ThemeSwitcher(ThemeSwitcherForm),
    ConfirmDelete(ConfirmDeleteForm),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalAction {
    None,
    Cancel,
    CreateComponent {
        id: String,
        name: String,
        kind: ComponentKind,
        parent_id: Option<String>,
    },
    UpdateComponent {
        component_id: String,
        name: String,
        kind: ComponentKind,
        description: String,
        relative_path: Option<String>,
        tags: Vec<String>,
        add_feature: Option<String>,
        add_sub_component: Option<String>,
    },
    UpdateConnection {
        from: String,
        to: String,
        kind: EdgeKind,
        label: Option<String>,
    },
    SelectTheme {
        name: String,
    },
    DeleteComponent {
        id: String,
    },
    DeleteConnection {
        from: String,
        to: String,
    },
}

fn cycle_next(index: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (index + 1) % len
    }
}

fn cycle_prev(index: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else if index == 0 || index >= len {
        len - 1
    } else {
        index - 1
    }
}

impl ModalState {
    pub fn new_component() -> Self {
        Self::NewComponent(NewComponentForm {
            id: String::new(),
            name: String::new(),
            archetype_index: 0,
            parent_id: None,
            active_field: 0,
            error: None,
        })
    }

    pub fn new_sub_component(parent_id: impl Into<String>) -> Self {
        Self::NewComponent(NewComponentForm {
            id: String::new(),
            name: String::new(),
            archetype_index: 0,
            parent_id: Some(parent_id.into()),
            active_field: 0,
            error: None,
        })
    }

    pub fn edit_component(comp: &Component) -> Self {
        let target_kind = archetype_for(comp.kind).kind;
        let archetype_index = ARCHETYPES
            .iter()
            .position(|spec| spec.kind == target_kind)
            .unwrap_or(0);
        Self::EditComponent(EditComponentForm {
            component_id: comp.id.clone(),
            name: comp.name.clone(),
            archetype_index,
            description: comp.description.clone(),
            relative_path: comp.relative_path.clone().unwrap_or_default(),
            tags: comp.tags.join(", "),
            new_feature: String::new(),
            new_sub_component: String::new(),
            active_field: 0,
            error: None,
        })
    }

    pub fn edit_connection(from: &str, to: &str, kind: EdgeKind, label: Option<&str>) -> Self {
        let kind_index = EDGE_KINDS.iter().position(|&k| k == kind).unwrap_or(0);
        Self::EditConnection(EditConnectionForm {
            from: from.to_string(),
            to: to.to_string(),
            kind_index,
            label: label.unwrap_or_default().to_string(),
            active_field: 0,
            error: None,
        })
    }

    pub fn help() -> Self {
        Self::Help
    }

    pub fn theme_switcher(themes: Vec<String>, current_theme: &str) -> Self {
        let selected_index = themes.iter().position(|t| t == current_theme).unwrap_or(0);
        Self::ThemeSwitcher(ThemeSwitcherForm {
            themes,
            selected_index,
            error: None,
        })
    }

    pub fn confirm_delete_component(
        id: impl Into<String>,
        name: impl Into<String>,
        affected_connections: usize,
    ) -> Self {
        Self::ConfirmDelete(ConfirmDeleteForm {
            target: ConfirmDeleteTarget::Component {
                id: id.into(),
                name: name.into(),
                affected_connections,
            },
            error: None,
        })
    }

    pub fn confirm_delete_connection(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::ConfirmDelete(ConfirmDeleteForm {
            target: ConfirmDeleteTarget::Connection {
                from: from.into(),
                to: to.into(),
            },
            error: None,
        })
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::NewComponent(form) => {
                if form.parent_id.is_some() {
                    "New Sub-Component"
                } else {
                    "New Component"
                }
            }
            Self::EditComponent(_) => "Edit Component",
            Self::EditConnection(_) => "Edit Connection",
            Self::Help => "Keyboard Help",
            Self::ThemeSwitcher(_) => "Switch Theme",
            Self::ConfirmDelete(form) => match form.target {
                ConfirmDeleteTarget::Component { .. } => "Delete Component",
                ConfirmDeleteTarget::Connection { .. } => "Delete Connection",
            },
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ModalAction {
        if key.code == KeyCode::Esc {
            return ModalAction::Cancel;
        }

        match self {
            Self::NewComponent(form) => handle_new_component_key(form, key),
            Self::EditComponent(form) => handle_edit_component_key(form, key),
            Self::EditConnection(form) => handle_edit_connection_key(form, key),
            Self::Help => handle_help_key(key),
            Self::ThemeSwitcher(form) => handle_theme_switcher_key(form, key),
            Self::ConfirmDelete(form) => handle_confirm_delete_key(form, key),
        }
    }
}

fn is_printable_char(key: KeyEvent) -> Option<char> {
    match key.code {
        KeyCode::Char(c)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(c)
        }
        _ => None,
    }
}

fn handle_new_component_key(form: &mut NewComponentForm, key: KeyEvent) -> ModalAction {
    match key.code {
        KeyCode::Tab | KeyCode::Down => {
            form.active_field = cycle_next(form.active_field, NewComponentForm::FIELD_COUNT);
            ModalAction::None
        }
        KeyCode::BackTab | KeyCode::Up => {
            form.active_field = cycle_prev(form.active_field, NewComponentForm::FIELD_COUNT);
            ModalAction::None
        }
        KeyCode::Left if form.active_field == 2 => {
            form.archetype_index = cycle_prev(form.archetype_index, ARCHETYPES.len());
            form.error = None;
            ModalAction::None
        }
        KeyCode::Right if form.active_field == 2 => {
            form.archetype_index = cycle_next(form.archetype_index, ARCHETYPES.len());
            form.error = None;
            ModalAction::None
        }
        KeyCode::Backspace => {
            if let Some(buf) = form.active_text_mut() {
                buf.pop();
                form.error = None;
            }
            ModalAction::None
        }
        KeyCode::Enter => {
            let id = form.id.trim();
            if let Err(err) = validate_component_id(id) {
                form.error = Some(err.to_string());
                return ModalAction::None;
            }
            let name = form.name.trim();
            if name.is_empty() {
                form.error = Some("component name cannot be empty".to_string());
                return ModalAction::None;
            }
            form.error = None;
            ModalAction::CreateComponent {
                id: id.to_string(),
                name: name.to_string(),
                kind: form.selected_archetype().kind,
                parent_id: form.parent_id.clone(),
            }
        }
        _ => {
            if let Some(c) = is_printable_char(key) {
                if let Some(buf) = form.active_text_mut() {
                    buf.push(c);
                    form.error = None;
                }
            }
            ModalAction::None
        }
    }
}

fn handle_edit_component_key(form: &mut EditComponentForm, key: KeyEvent) -> ModalAction {
    match key.code {
        KeyCode::Tab | KeyCode::Down => {
            form.active_field = cycle_next(form.active_field, EditComponentForm::FIELD_COUNT);
            ModalAction::None
        }
        KeyCode::BackTab | KeyCode::Up => {
            form.active_field = cycle_prev(form.active_field, EditComponentForm::FIELD_COUNT);
            ModalAction::None
        }
        KeyCode::Left if form.active_field == 1 => {
            form.archetype_index = cycle_prev(form.archetype_index, ARCHETYPES.len());
            form.error = None;
            ModalAction::None
        }
        KeyCode::Right if form.active_field == 1 => {
            form.archetype_index = cycle_next(form.archetype_index, ARCHETYPES.len());
            form.error = None;
            ModalAction::None
        }
        KeyCode::Backspace => {
            if let Some(buf) = form.active_text_mut() {
                buf.pop();
                form.error = None;
            }
            ModalAction::None
        }
        KeyCode::Enter => {
            let name = form.name.trim();
            if name.is_empty() {
                form.error = Some("component name cannot be empty".to_string());
                return ModalAction::None;
            }
            let description = form.description.trim().to_string();
            let trimmed_path = form.relative_path.trim();
            let relative_path = if trimmed_path.is_empty() {
                None
            } else {
                Some(trimmed_path.to_string())
            };
            let tags = form
                .tags
                .split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(str::to_string)
                .collect();
            let trimmed_feature = form.new_feature.trim();
            let add_feature = if trimmed_feature.is_empty() {
                None
            } else {
                Some(trimmed_feature.to_string())
            };
            let trimmed_sub = form.new_sub_component.trim();
            let add_sub_component = if trimmed_sub.is_empty() {
                None
            } else {
                Some(trimmed_sub.to_string())
            };
            form.error = None;
            ModalAction::UpdateComponent {
                component_id: form.component_id.clone(),
                name: name.to_string(),
                kind: form.selected_archetype().kind,
                description,
                relative_path,
                tags,
                add_feature,
                add_sub_component,
            }
        }
        _ => {
            if let Some(c) = is_printable_char(key) {
                if let Some(buf) = form.active_text_mut() {
                    buf.push(c);
                    form.error = None;
                }
            }
            ModalAction::None
        }
    }
}

fn handle_edit_connection_key(form: &mut EditConnectionForm, key: KeyEvent) -> ModalAction {
    match key.code {
        KeyCode::Tab | KeyCode::Down => {
            form.active_field = cycle_next(form.active_field, EditConnectionForm::FIELD_COUNT);
            ModalAction::None
        }
        KeyCode::BackTab | KeyCode::Up => {
            form.active_field = cycle_prev(form.active_field, EditConnectionForm::FIELD_COUNT);
            ModalAction::None
        }
        KeyCode::Left if form.active_field == 0 => {
            form.kind_index = cycle_prev(form.kind_index, EDGE_KINDS.len());
            form.error = None;
            ModalAction::None
        }
        KeyCode::Right if form.active_field == 0 => {
            form.kind_index = cycle_next(form.kind_index, EDGE_KINDS.len());
            form.error = None;
            ModalAction::None
        }
        KeyCode::Backspace => {
            if let Some(buf) = form.active_text_mut() {
                buf.pop();
                form.error = None;
            }
            ModalAction::None
        }
        KeyCode::Enter => {
            let trimmed_label = form.label.trim();
            let label = if trimmed_label.is_empty() {
                None
            } else {
                Some(trimmed_label.to_string())
            };
            form.error = None;
            ModalAction::UpdateConnection {
                from: form.from.clone(),
                to: form.to.clone(),
                kind: form.selected_kind(),
                label,
            }
        }
        _ => {
            if let Some(c) = is_printable_char(key) {
                if let Some(buf) = form.active_text_mut() {
                    buf.push(c);
                    form.error = None;
                }
            }
            ModalAction::None
        }
    }
}

fn handle_help_key(key: KeyEvent) -> ModalAction {
    match key.code {
        KeyCode::Enter | KeyCode::Char('?') | KeyCode::Char('q') => ModalAction::Cancel,
        _ => ModalAction::None,
    }
}

fn handle_theme_switcher_key(form: &mut ThemeSwitcherForm, key: KeyEvent) -> ModalAction {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            form.selected_index = cycle_prev(form.selected_index, form.themes.len());
            form.error = None;
            ModalAction::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            form.selected_index = cycle_next(form.selected_index, form.themes.len());
            form.error = None;
            ModalAction::None
        }
        KeyCode::Enter => match form.selected_theme() {
            Some(name) => ModalAction::SelectTheme {
                name: name.to_string(),
            },
            None => ModalAction::Cancel,
        },
        _ => ModalAction::None,
    }
}

fn handle_confirm_delete_key(form: &mut ConfirmDeleteForm, key: KeyEvent) -> ModalAction {
    match key.code {
        KeyCode::Enter | KeyCode::Char('y' | 'Y') => {
            form.error = None;
            match &form.target {
                ConfirmDeleteTarget::Component { id, .. } => {
                    ModalAction::DeleteComponent { id: id.clone() }
                }
                ConfirmDeleteTarget::Connection { from, to } => ModalAction::DeleteConnection {
                    from: from.clone(),
                    to: to.clone(),
                },
            }
        }
        KeyCode::Esc | KeyCode::Char('n' | 'N') => ModalAction::Cancel,
        _ => ModalAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::GridPos;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn type_text(modal: &mut ModalState, text: &str) {
        for ch in text.chars() {
            assert_eq!(modal.handle_key(key(KeyCode::Char(ch))), ModalAction::None);
        }
    }

    #[test]
    fn new_component_rejects_invalid_then_submits_valid() {
        let mut modal = ModalState::new_component();
        assert_eq!(modal.title(), "New Component");

        // Empty submission rejected
        assert_eq!(modal.handle_key(key(KeyCode::Enter)), ModalAction::None);
        let ModalState::NewComponent(ref form) = modal else {
            panic!("expected NewComponent");
        };
        assert!(form.error.is_some());

        // Invalid uppercase ID rejected
        type_text(&mut modal, "BadId");
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        type_text(&mut modal, "Telemetry");
        assert_eq!(modal.handle_key(key(KeyCode::Enter)), ModalAction::None);
        let ModalState::NewComponent(ref form) = modal else {
            panic!("expected NewComponent");
        };
        assert!(form.error.is_some());

        // Fix ID to valid value, cycle archetype to Interface, and submit
        assert_eq!(modal.handle_key(key(KeyCode::BackTab)), ModalAction::None);
        for _ in 0..5 {
            assert_eq!(modal.handle_key(key(KeyCode::Backspace)), ModalAction::None);
        }
        type_text(&mut modal, "telemetry_bus");
        assert_eq!(modal.handle_key(key(KeyCode::Down)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Down)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Right)), ModalAction::None);

        let action = modal.handle_key(key(KeyCode::Enter));
        assert_eq!(
            action,
            ModalAction::CreateComponent {
                id: "telemetry_bus".to_string(),
                name: "Telemetry".to_string(),
                kind: ComponentKind::Interface,
                parent_id: None,
            }
        );
    }

    #[test]
    fn new_sub_component_sets_title_and_emits_parent_id() {
        let mut modal = ModalState::new_sub_component("core_bus");
        assert_eq!(modal.title(), "New Sub-Component");

        type_text(&mut modal, "bus_worker");
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        type_text(&mut modal, "Bus Worker");
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Right)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Right)), ModalAction::None);

        let action = modal.handle_key(key(KeyCode::Enter));
        assert_eq!(
            action,
            ModalAction::CreateComponent {
                id: "bus_worker".to_string(),
                name: "Bus Worker".to_string(),
                kind: ComponentKind::Logic,
                parent_id: Some("core_bus".to_string()),
            }
        );
    }

    #[test]
    fn edit_component_parses_tags_and_optional_feature() {
        let comp = Component {
            id: "ui_view".to_string(),
            name: "UI View".to_string(),
            kind: ComponentKind::UiView,
            parent_id: None,
            description: "Primary screen".to_string(),
            relative_path: Some("src/tui/ui.rs".to_string()),
            position: GridPos { col: 0, row: 0 },
            tags: vec!["ui".to_string(), "view".to_string()],
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        };

        let mut modal = ModalState::edit_component(&comp);
        assert_eq!(modal.title(), "Edit Component");

        // Submit without adding a feature -> add_feature and add_sub_component are None
        let initial_action = modal.handle_key(key(KeyCode::Enter));
        assert_eq!(
            initial_action,
            ModalAction::UpdateComponent {
                component_id: "ui_view".to_string(),
                name: "UI View".to_string(),
                kind: ComponentKind::Interface,
                description: "Primary screen".to_string(),
                relative_path: Some("src/tui/ui.rs".to_string()),
                tags: vec!["ui".to_string(), "view".to_string()],
                add_feature: None,
                add_sub_component: None,
            }
        );

        // Move to archetype (field 1), cycle right (Interface -> Logic), then left (Logic -> Interface), then right (Logic)
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Right)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Left)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Right)), ModalAction::None);

        // Move to tags (field 4): Tab past description (2), path (3), to tags (4)
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        type_text(&mut modal, ",  modal , , layout ");

        // Move to new_feature (field 5) and type a feature title
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        type_text(&mut modal, "Render modal overlay");

        // Move to new_sub_component (field 6) and type a sub-component name
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        type_text(&mut modal, "  Modal Renderer  ");

        let updated = modal.handle_key(key(KeyCode::Enter));
        assert_eq!(
            updated,
            ModalAction::UpdateComponent {
                component_id: "ui_view".to_string(),
                name: "UI View".to_string(),
                kind: ComponentKind::Logic,
                description: "Primary screen".to_string(),
                relative_path: Some("src/tui/ui.rs".to_string()),
                tags: vec![
                    "ui".to_string(),
                    "view".to_string(),
                    "modal".to_string(),
                    "layout".to_string(),
                ],
                add_feature: Some("Render modal overlay".to_string()),
                add_sub_component: Some("Modal Renderer".to_string()),
            }
        );
    }

    #[test]
    fn edge_relationship_preview_formats_all_variants() {
        assert_eq!(
            edge_relationship_preview("api", "db", EdgeKind::DependsOn),
            "api depends on db"
        );
        assert_eq!(
            edge_relationship_preview("ingest", "queue", EdgeKind::DataFlow),
            "ingest sends data to queue"
        );
        assert_eq!(
            edge_relationship_preview("shell", "canvas", EdgeKind::Renders),
            "shell renders canvas"
        );
        assert_eq!(
            edge_relationship_preview("cli", "runner", EdgeKind::Invokes),
            "cli invokes runner"
        );
        assert_eq!(
            edge_relationship_preview("core", "worker", EdgeKind::Contains),
            "core contains worker (sub-component)"
        );
        assert_eq!(
            edge_relationship_preview("alpha", "beta", EdgeKind::Custom),
            "alpha -> beta"
        );
    }

    #[test]
    fn edit_connection_cycles_kind_updates_label_and_cancels_on_esc() {
        let mut modal =
            ModalState::edit_connection("stream_core", "ui_view", EdgeKind::DependsOn, Some("old"));
        assert_eq!(modal.title(), "Edit Connection");

        // Cycle left wraps to Custom, then right wraps back to DependsOn, then right -> DataFlow
        assert_eq!(modal.handle_key(key(KeyCode::Left)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Right)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Right)), ModalAction::None);

        // Move to label field, replace "old" with "  events  "
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        for _ in 0..3 {
            assert_eq!(modal.handle_key(key(KeyCode::Backspace)), ModalAction::None);
        }
        type_text(&mut modal, "  events  ");

        let action = modal.handle_key(key(KeyCode::Enter));
        assert_eq!(
            action,
            ModalAction::UpdateConnection {
                from: "stream_core".to_string(),
                to: "ui_view".to_string(),
                kind: EdgeKind::DataFlow,
                label: Some("events".to_string()),
            }
        );

        // Esc cancels
        assert_eq!(modal.handle_key(key(KeyCode::Esc)), ModalAction::Cancel);
    }

    #[test]
    fn help_modal_dismisses_on_esc_question_enter_and_q() {
        let mut modal = ModalState::help();
        assert_eq!(modal.title(), "Keyboard Help");

        assert_eq!(modal.handle_key(key(KeyCode::Char('a'))), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Esc)), ModalAction::Cancel);
        assert_eq!(
            modal.handle_key(key(KeyCode::Char('?'))),
            ModalAction::Cancel
        );
        assert_eq!(modal.handle_key(key(KeyCode::Enter)), ModalAction::Cancel);
        assert_eq!(
            modal.handle_key(key(KeyCode::Char('q'))),
            ModalAction::Cancel
        );
    }

    #[test]
    fn theme_switcher_navigates_wraps_and_selects_theme() {
        let themes = vec![
            "amber_alt".to_string(),
            "crt_amber".to_string(),
            "phosphor_green".to_string(),
        ];
        let mut modal = ModalState::theme_switcher(themes, "crt_amber");
        assert_eq!(modal.title(), "Switch Theme");

        let ModalState::ThemeSwitcher(ref form) = modal else {
            panic!("expected ThemeSwitcher");
        };
        assert_eq!(form.selected_index, 1);
        assert_eq!(form.selected_theme(), Some("crt_amber"));

        // Down -> phosphor_green, Down -> wraps to amber_alt
        assert_eq!(modal.handle_key(key(KeyCode::Down)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Char('j'))), ModalAction::None);
        assert_eq!(
            modal.handle_key(key(KeyCode::Enter)),
            ModalAction::SelectTheme {
                name: "amber_alt".to_string()
            }
        );

        // Up from 0 wraps to last (phosphor_green)
        assert_eq!(modal.handle_key(key(KeyCode::Char('k'))), ModalAction::None);
        assert_eq!(
            modal.handle_key(key(KeyCode::Enter)),
            ModalAction::SelectTheme {
                name: "phosphor_green".to_string()
            }
        );

        // Esc cancels
        assert_eq!(modal.handle_key(key(KeyCode::Esc)), ModalAction::Cancel);

        // Empty theme list cancels on Enter
        let mut empty_modal = ModalState::theme_switcher(Vec::new(), "crt_amber");
        assert_eq!(
            empty_modal.handle_key(key(KeyCode::Enter)),
            ModalAction::Cancel
        );
    }

    #[test]
    fn confirm_delete_component_emits_delete_cancel_and_none() {
        let mut modal = ModalState::confirm_delete_component("stream_core", "Stream Core", 2);
        assert_eq!(modal.title(), "Delete Component");

        if let ModalState::ConfirmDelete(ref mut form) = modal {
            form.error = Some("previous error".to_string());
        }

        // Unrelated keys emit None
        assert_eq!(modal.handle_key(key(KeyCode::Char('x'))), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);

        // Enter / y / Y clear error and emit DeleteComponent
        assert_eq!(
            modal.handle_key(key(KeyCode::Enter)),
            ModalAction::DeleteComponent {
                id: "stream_core".to_string(),
            }
        );
        let ModalState::ConfirmDelete(ref form) = modal else {
            panic!("expected ConfirmDelete");
        };
        assert!(form.error.is_none());

        assert_eq!(
            modal.handle_key(key(KeyCode::Char('y'))),
            ModalAction::DeleteComponent {
                id: "stream_core".to_string(),
            }
        );
        assert_eq!(
            modal.handle_key(key(KeyCode::Char('Y'))),
            ModalAction::DeleteComponent {
                id: "stream_core".to_string(),
            }
        );

        // n / N / Esc emit Cancel
        assert_eq!(
            modal.handle_key(key(KeyCode::Char('n'))),
            ModalAction::Cancel
        );
        assert_eq!(
            modal.handle_key(key(KeyCode::Char('N'))),
            ModalAction::Cancel
        );
        assert_eq!(modal.handle_key(key(KeyCode::Esc)), ModalAction::Cancel);
    }

    #[test]
    fn confirm_delete_connection_emits_delete_cancel_and_none() {
        let mut modal = ModalState::confirm_delete_connection("view_box", "data_stream");
        assert_eq!(modal.title(), "Delete Connection");

        assert_eq!(modal.handle_key(key(KeyCode::Char('z'))), ModalAction::None);
        assert_eq!(
            modal.handle_key(key(KeyCode::Enter)),
            ModalAction::DeleteConnection {
                from: "view_box".to_string(),
                to: "data_stream".to_string(),
            }
        );
        assert_eq!(
            modal.handle_key(key(KeyCode::Char('y'))),
            ModalAction::DeleteConnection {
                from: "view_box".to_string(),
                to: "data_stream".to_string(),
            }
        );
        assert_eq!(
            modal.handle_key(key(KeyCode::Char('n'))),
            ModalAction::Cancel
        );
        assert_eq!(modal.handle_key(key(KeyCode::Esc)), ModalAction::Cancel);
    }
}
