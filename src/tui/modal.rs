use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::archetype::{ArchetypeSpec, ARCHETYPES};
use crate::model::{Component, ComponentKind, EdgeKind};
use crate::storage::validate_component_id;

pub const EDGE_KINDS: &[EdgeKind] = &[
    EdgeKind::DependsOn,
    EdgeKind::DataFlow,
    EdgeKind::Renders,
    EdgeKind::Invokes,
    EdgeKind::Custom,
];

pub fn edge_kind_label(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::DependsOn => "depends_on",
        EdgeKind::DataFlow => "data_flow",
        EdgeKind::Renders => "renders",
        EdgeKind::Invokes => "invokes",
        EdgeKind::Custom => "custom",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewComponentForm {
    pub id: String,
    pub name: String,
    pub archetype_index: usize,
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
    pub description: String,
    pub relative_path: String,
    pub tags: String,
    pub new_feature: String,
    pub active_field: usize,
    pub error: Option<String>,
}

impl EditComponentForm {
    pub const FIELD_COUNT: usize = 5;

    fn active_text_mut(&mut self) -> Option<&mut String> {
        match self.active_field {
            0 => Some(&mut self.name),
            1 => Some(&mut self.description),
            2 => Some(&mut self.relative_path),
            3 => Some(&mut self.tags),
            4 => Some(&mut self.new_feature),
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
pub enum ModalState {
    NewComponent(NewComponentForm),
    EditComponent(EditComponentForm),
    EditConnection(EditConnectionForm),
    Help,
    ThemeSwitcher(ThemeSwitcherForm),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalAction {
    None,
    Cancel,
    CreateComponent {
        id: String,
        name: String,
        kind: ComponentKind,
    },
    UpdateComponent {
        component_id: String,
        name: String,
        description: String,
        relative_path: Option<String>,
        tags: Vec<String>,
        add_feature: Option<String>,
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
            active_field: 0,
            error: None,
        })
    }

    pub fn edit_component(comp: &Component) -> Self {
        Self::EditComponent(EditComponentForm {
            component_id: comp.id.clone(),
            name: comp.name.clone(),
            description: comp.description.clone(),
            relative_path: comp.relative_path.clone().unwrap_or_default(),
            tags: comp.tags.join(", "),
            new_feature: String::new(),
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

    pub fn title(&self) -> &'static str {
        match self {
            Self::NewComponent(_) => "New Component",
            Self::EditComponent(_) => "Edit Component",
            Self::EditConnection(_) => "Edit Connection",
            Self::Help => "Keyboard Help",
            Self::ThemeSwitcher(_) => "Switch Theme",
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
            form.error = None;
            ModalAction::UpdateComponent {
                component_id: form.component_id.clone(),
                name: name.to_string(),
                description,
                relative_path,
                tags,
                add_feature,
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

        // Fix ID to valid value, cycle archetype to DataStream, and submit
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
                kind: ComponentKind::DataStream,
            }
        );
    }

    #[test]
    fn edit_component_parses_tags_and_optional_feature() {
        let comp = Component {
            id: "ui_view".to_string(),
            name: "UI View".to_string(),
            kind: ComponentKind::UiView,
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

        // Submit without adding a feature -> add_feature is None
        let initial_action = modal.handle_key(key(KeyCode::Enter));
        assert_eq!(
            initial_action,
            ModalAction::UpdateComponent {
                component_id: "ui_view".to_string(),
                name: "UI View".to_string(),
                description: "Primary screen".to_string(),
                relative_path: Some("src/tui/ui.rs".to_string()),
                tags: vec!["ui".to_string(), "view".to_string()],
                add_feature: None,
            }
        );

        // Move to tags (field 3), append messy comma-separated tags
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        type_text(&mut modal, ",  modal , , layout ");

        // Move to new_feature (field 4) and type a feature title
        assert_eq!(modal.handle_key(key(KeyCode::Tab)), ModalAction::None);
        type_text(&mut modal, "Render modal overlay");

        let updated = modal.handle_key(key(KeyCode::Enter));
        assert_eq!(
            updated,
            ModalAction::UpdateComponent {
                component_id: "ui_view".to_string(),
                name: "UI View".to_string(),
                description: "Primary screen".to_string(),
                relative_path: Some("src/tui/ui.rs".to_string()),
                tags: vec![
                    "ui".to_string(),
                    "view".to_string(),
                    "modal".to_string(),
                    "layout".to_string(),
                ],
                add_feature: Some("Render modal overlay".to_string()),
            }
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
}
