use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::adapter::{
    load_adapters_file, push_unpushed_features, reconcile_status, resolve_active_adapters,
    run_pull_status, run_push_component,
};
use crate::archetype::scaffold_component;
use crate::audit::{audit_project, compute_card_status, AuditFinding};
use crate::error::{RfError, RfResult};
use crate::model::{CardStatus, Component, Connection, EdgeKind, FeatureItem, GridPos, Project};
use crate::storage::{
    save_component, save_project_config, validate_component_id, COMPONENTS_DIR, ROCKET_DIR,
};
use crate::theme::Theme;
use crate::tui::canvas::CanvasViewport;
use crate::tui::modal::{ModalAction, ModalState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusColumn {
    List,
    Canvas,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Navigate,
    MoveCard,
    Connect,
    Modal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFilter {
    All,
    GapsOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncRequest {
    Pull,
    PushSelected,
}

#[derive(Debug, Clone)]
pub struct App {
    pub root: PathBuf,
    pub project: Project,
    pub theme: Theme,
    pub theme_dirs: Vec<PathBuf>,
    pub focus: FocusColumn,
    pub mode: InputMode,
    pub filter: ListFilter,
    pub selected_index: usize,
    pub detail_scroll: u16,
    pub status_message: String,
    pub should_quit: bool,
    pub audit_findings: Vec<AuditFinding>,
    pub viewport: CanvasViewport,
    pub connect_source_id: Option<String>,
    pub modal: Option<ModalState>,
    pub pending_editor_component: Option<String>,
    pub pending_sync: Option<SyncRequest>,
}

impl App {
    pub fn new(root: PathBuf, project: Project, theme: Theme) -> Self {
        let audit_findings = audit_project(&project);
        let theme_dirs = crate::tui::terminal::default_theme_dirs(&root);
        Self {
            root,
            project,
            theme,
            theme_dirs,
            focus: FocusColumn::List,
            mode: InputMode::Navigate,
            filter: ListFilter::All,
            selected_index: 0,
            detail_scroll: 0,
            status_message: "Ready".to_string(),
            should_quit: false,
            audit_findings,
            viewport: CanvasViewport::default(),
            connect_source_id: None,
            modal: None,
            pending_editor_component: None,
            pending_sync: None,
        }
    }

    pub fn refresh_audit(&mut self) {
        self.audit_findings = audit_project(&self.project);
    }

    pub fn reload_component_markdown(&mut self, component_id: &str) -> RfResult<()> {
        validate_component_id(component_id)?;

        let components_dir = self.root.join(ROCKET_DIR).join(COMPONENTS_DIR);
        let md_path = components_dir.join(format!("{component_id}.md"));

        let canon_dir = std::fs::canonicalize(&components_dir).map_err(|source| RfError::Io {
            path: components_dir,
            source,
        })?;
        let canon_md = std::fs::canonicalize(&md_path).map_err(|source| RfError::Io {
            path: md_path,
            source,
        })?;
        if !canon_md.starts_with(&canon_dir) {
            return Err(RfError::PathEscape(canon_md));
        }

        let body = std::fs::read_to_string(&canon_md).map_err(|source| RfError::Io {
            path: canon_md,
            source,
        })?;

        let comp = self
            .project
            .component_mut(component_id)
            .ok_or_else(|| RfError::NotFound(component_id.to_string()))?;
        let comp_name = comp.name.clone();
        comp.markdown_body = body;

        self.refresh_audit();
        self.status_message = format!("Reloaded Markdown for {comp_name}");
        Ok(())
    }

    pub fn sync_pull(&mut self, adapters_path: &Path) -> RfResult<String> {
        if self.project.config.active_adapters.is_empty() {
            let msg = "No active adapters configured".to_string();
            self.status_message = msg.clone();
            return Ok(msg);
        }

        let adapters_file = load_adapters_file(adapters_path)?;
        let active_adapters =
            resolve_active_adapters(&adapters_file, &self.project.config.active_adapters)?;

        let mut candidate = self.project.clone();
        let mut components_updated = 0usize;
        let mut tasks_synced = 0usize;
        let mut features_linked = 0usize;
        let mut updated_ids: Vec<String> = Vec::new();

        for active in &active_adapters {
            if active.pull_status_cmd.is_some() {
                let resp = run_pull_status(active, &self.root, &candidate)?;
                let summary = reconcile_status(&mut candidate, &resp);
                components_updated += summary.components_updated;
                tasks_synced += summary.tasks_synced;
                features_linked += summary.features_linked;

                for entry in &resp.components {
                    if candidate.component(&entry.component_id).is_some()
                        && !updated_ids.contains(&entry.component_id)
                    {
                        updated_ids.push(entry.component_id.clone());
                    }
                }
            }
        }

        if self.root.join(ROCKET_DIR).exists() {
            for comp_id in &updated_ids {
                if let Some(comp) = candidate.component(comp_id) {
                    save_component(&self.root, comp)?;
                }
            }
        }

        self.project = candidate;
        self.refresh_audit();
        self.clamp_selection();

        let summary_msg = format!(
            "Synced {} adapter(s): {components_updated} component(s) updated, {tasks_synced} task(s) synced, {features_linked} feature(s) linked",
            active_adapters.len()
        );
        self.status_message = summary_msg.clone();
        Ok(summary_msg)
    }

    pub fn sync_push_selected(&mut self, adapters_path: &Path) -> RfResult<String> {
        let Some(comp_id) = self.selected_component().map(|c| c.id.clone()) else {
            let msg = "No component selected to push".to_string();
            self.status_message = msg.clone();
            return Ok(msg);
        };

        if self.project.config.active_adapters.is_empty() {
            let msg = "No active adapters configured".to_string();
            self.status_message = msg.clone();
            return Ok(msg);
        }

        validate_component_id(&comp_id)?;

        let adapters_file = load_adapters_file(adapters_path)?;
        let active_adapters =
            resolve_active_adapters(&adapters_file, &self.project.config.active_adapters)?;

        let mut candidate = self.project.clone();
        let mut components_pushed = 0usize;
        let mut features_pushed = 0usize;
        let mut modified = false;

        for active in &active_adapters {
            if active.push_component_cmd.is_some() {
                run_push_component(active, &self.root, &candidate, &comp_id)?;
                components_pushed += 1;
            }
        }

        if let Some(comp) = candidate.component_mut(&comp_id) {
            let summary = push_unpushed_features(&active_adapters, &self.root, comp)?;
            features_pushed += summary.features_pushed;
            modified = summary.modified;
        }

        if modified && self.root.join(ROCKET_DIR).exists() {
            if let Some(comp) = candidate.component(&comp_id) {
                save_component(&self.root, comp)?;
            }
        }

        self.project = candidate;
        self.refresh_audit();
        self.clamp_selection();

        let summary_msg = format!(
            "Pushed {comp_id}: {components_pushed} component(s), {features_pushed} feature(s) across {} adapter(s)",
            active_adapters.len()
        );
        self.status_message = summary_msg.clone();
        Ok(summary_msg)
    }

    pub fn gap_count(&self) -> usize {
        self.project
            .components
            .iter()
            .filter(|c| compute_card_status(c) == CardStatus::Gap)
            .count()
    }

    pub fn visible_component_indices(&self) -> Vec<usize> {
        match self.filter {
            ListFilter::All => (0..self.project.components.len()).collect(),
            ListFilter::GapsOnly => self
                .project
                .components
                .iter()
                .enumerate()
                .filter_map(|(i, c)| (compute_card_status(c) == CardStatus::Gap).then_some(i))
                .collect(),
        }
    }

    pub fn selected_component(&self) -> Option<&Component> {
        let indices = self.visible_component_indices();
        indices
            .get(self.selected_index)
            .and_then(|&idx| self.project.components.get(idx))
    }

    pub fn clamp_selection(&mut self) {
        let len = self.visible_component_indices().len();
        if len == 0 {
            self.selected_index = 0;
        } else {
            self.selected_index = self.selected_index.min(len - 1);
        }
    }

    pub fn select_next(&mut self) {
        let visible = self.visible_component_indices();
        if visible.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % visible.len();
        self.detail_scroll = 0;
    }

    pub fn select_prev(&mut self) {
        let visible = self.visible_component_indices();
        if visible.is_empty() {
            return;
        }
        self.selected_index = if self.selected_index == 0 || self.selected_index >= visible.len() {
            visible.len() - 1
        } else {
            self.selected_index - 1
        };
        self.detail_scroll = 0;
    }

    pub fn cycle_focus_next(&mut self) {
        self.focus = match self.focus {
            FocusColumn::List => FocusColumn::Canvas,
            FocusColumn::Canvas => FocusColumn::Detail,
            FocusColumn::Detail => FocusColumn::List,
        };
    }

    pub fn cycle_focus_prev(&mut self) {
        self.focus = match self.focus {
            FocusColumn::List => FocusColumn::Detail,
            FocusColumn::Detail => FocusColumn::Canvas,
            FocusColumn::Canvas => FocusColumn::List,
        };
    }

    pub fn toggle_gaps_filter(&mut self) {
        self.filter = match self.filter {
            ListFilter::All => ListFilter::GapsOnly,
            ListFilter::GapsOnly => ListFilter::All,
        };
        self.clamp_selection();
        self.detail_scroll = 0;
        self.status_message = match self.filter {
            ListFilter::GapsOnly => "Filter: Gaps Only".to_string(),
            ListFilter::All => "Filter: All Components".to_string(),
        };
    }

    fn select_in_direction(&mut self, dir: Direction) {
        let visible = self.visible_component_indices();
        let Some(&current_comp_idx) = visible.get(self.selected_index) else {
            return;
        };
        let Some(current) = self.project.components.get(current_comp_idx) else {
            return;
        };
        let origin = current.position;

        let best = visible
            .iter()
            .enumerate()
            .filter_map(|(vis_idx, &comp_idx)| {
                if vis_idx == self.selected_index {
                    return None;
                }
                let cand = self.project.components.get(comp_idx)?;
                let pos = cand.position;
                let in_dir = match dir {
                    Direction::Left => pos.col < origin.col,
                    Direction::Right => pos.col > origin.col,
                    Direction::Up => pos.row < origin.row,
                    Direction::Down => pos.row > origin.row,
                };
                if !in_dir {
                    return None;
                }
                let dist = u32::from(origin.col.abs_diff(pos.col))
                    + u32::from(origin.row.abs_diff(pos.row));
                Some((vis_idx, dist))
            })
            .min_by_key(|&(vis_idx, dist)| (dist, vis_idx));

        if let Some((vis_idx, _)) = best {
            self.selected_index = vis_idx;
            self.detail_scroll = 0;
        }
    }

    fn move_selected_card(&mut self, dir: Direction) {
        let visible = self.visible_component_indices();
        let Some(&comp_idx) = visible.get(self.selected_index) else {
            return;
        };
        let Some(comp) = self.project.components.get(comp_idx) else {
            return;
        };

        let max_col = self.project.config.grid.cols.saturating_sub(1);
        let max_row = self.project.config.grid.rows.saturating_sub(1);
        let current_pos = comp.position;

        let target_pos = match dir {
            Direction::Left => GridPos {
                col: current_pos.col.saturating_sub(1),
                row: current_pos.row,
            },
            Direction::Right => GridPos {
                col: current_pos.col.saturating_add(1).min(max_col),
                row: current_pos.row,
            },
            Direction::Up => GridPos {
                col: current_pos.col,
                row: current_pos.row.saturating_sub(1),
            },
            Direction::Down => GridPos {
                col: current_pos.col,
                row: current_pos.row.saturating_add(1).min(max_row),
            },
        };

        if target_pos == current_pos {
            return;
        }

        if let Some(occupant) = self
            .project
            .components
            .iter()
            .enumerate()
            .find(|(idx, c)| *idx != comp_idx && c.position == target_pos)
            .map(|(_, c)| c.name.clone())
        {
            self.status_message = format!(
                "Cell ({}, {}) is occupied by {}",
                target_pos.col, target_pos.row, occupant
            );
            return;
        }

        let comp_name = self.project.components[comp_idx].name.clone();
        let mut candidate = self.project.components[comp_idx].clone();
        candidate.position = target_pos;

        if self.root.join(ROCKET_DIR).exists() {
            if let Err(err) = save_component(&self.root, &candidate) {
                self.refresh_audit();
                self.status_message = format!("Failed to save {comp_name}: {err}");
                return;
            }
        }

        self.project.components[comp_idx] = candidate;
        self.refresh_audit();
        self.status_message = format!(
            "Moved {comp_name} to ({}, {})",
            target_pos.col, target_pos.row
        );
    }

    fn handle_direction_key(&mut self, dir: Direction) {
        if self.mode == InputMode::MoveCard {
            self.move_selected_card(dir);
            return;
        }

        match self.focus {
            FocusColumn::Canvas => self.select_in_direction(dir),
            FocusColumn::Detail => match dir {
                Direction::Down => self.detail_scroll = self.detail_scroll.saturating_add(1),
                Direction::Up => self.detail_scroll = self.detail_scroll.saturating_sub(1),
                Direction::Left | Direction::Right => {}
            },
            FocusColumn::List => match dir {
                Direction::Down => self.select_next(),
                Direction::Up => self.select_prev(),
                Direction::Left | Direction::Right => {}
            },
        }
    }

    fn confirm_connect(&mut self) {
        let Some(source_id) = self.connect_source_id.clone() else {
            self.mode = InputMode::Navigate;
            return;
        };
        let Some(target_id) = self.selected_component().map(|c| c.id.clone()) else {
            self.connect_source_id = None;
            self.mode = InputMode::Navigate;
            self.status_message = "No target component selected".to_string();
            return;
        };

        if source_id == target_id {
            self.connect_source_id = None;
            self.mode = InputMode::Navigate;
            self.status_message = "Cannot connect a component to itself".to_string();
            return;
        }

        let mut candidate_config = self.project.config.clone();
        let mut msg = if let Some(pos) = candidate_config
            .connections
            .iter()
            .position(|conn| conn.from == source_id && conn.to == target_id)
        {
            candidate_config.connections.remove(pos);
            format!("Removed connection {source_id} -> {target_id}")
        } else {
            candidate_config.connections.push(Connection {
                from: source_id.clone(),
                to: target_id.clone(),
                kind: EdgeKind::DependsOn,
                label: None,
            });
            format!("Connected {source_id} -> {target_id}")
        };

        let save_result = if self.root.join(ROCKET_DIR).exists() {
            save_project_config(&self.root, &candidate_config)
        } else {
            Ok(())
        };

        match save_result {
            Ok(()) => {
                self.project.config = candidate_config;
            }
            Err(err) => {
                msg = format!("Failed to save connections: {err}");
            }
        }

        self.refresh_audit();
        self.connect_source_id = None;
        self.mode = InputMode::Navigate;
        self.status_message = msg;
    }

    fn first_unoccupied_grid_pos(&self) -> Option<GridPos> {
        for row in 0..self.project.config.grid.rows {
            for col in 0..self.project.config.grid.cols {
                let candidate = GridPos { col, row };
                if !self
                    .project
                    .components
                    .iter()
                    .any(|comp| comp.position == candidate)
                {
                    return Some(candidate);
                }
            }
        }
        None
    }

    fn apply_modal_action(&mut self, mut modal: ModalState, action: ModalAction) {
        match action {
            ModalAction::None => {
                self.modal = Some(modal);
            }
            ModalAction::Cancel => {
                self.mode = InputMode::Navigate;
                self.status_message = "Mode: Navigate".to_string();
            }
            ModalAction::CreateComponent { id, name, kind } => {
                if self.project.component(&id).is_some() {
                    let err = format!("component id '{id}' already exists");
                    if let ModalState::NewComponent(ref mut form) = modal {
                        form.error = Some(err.clone());
                    }
                    self.status_message = err;
                    self.modal = Some(modal);
                    return;
                }

                let Some(pos) = self.first_unoccupied_grid_pos() else {
                    let err = format!(
                        "grid ({}x{}) is full; no free cell available",
                        self.project.config.grid.cols, self.project.config.grid.rows
                    );
                    if let ModalState::NewComponent(ref mut form) = modal {
                        form.error = Some(err.clone());
                    }
                    self.status_message = err;
                    self.modal = Some(modal);
                    return;
                };
                let comp = scaffold_component(&id, &name, kind, pos);

                if self.root.join(ROCKET_DIR).exists() {
                    if let Err(err) = save_component(&self.root, &comp) {
                        let msg = format!("Failed to save {name}: {err}");
                        if let ModalState::NewComponent(ref mut form) = modal {
                            form.error = Some(msg.clone());
                        }
                        self.status_message = msg;
                        self.modal = Some(modal);
                        return;
                    }
                }

                self.project.components.push(comp);
                let new_comp_idx = self.project.components.len() - 1;
                if let Some(vis_idx) = self
                    .visible_component_indices()
                    .iter()
                    .position(|&idx| idx == new_comp_idx)
                {
                    self.selected_index = vis_idx;
                    self.detail_scroll = 0;
                }
                self.refresh_audit();
                self.mode = InputMode::Navigate;
                self.status_message = format!("Created component {name} ({id})");
            }
            ModalAction::UpdateComponent {
                component_id,
                name,
                description,
                relative_path,
                tags,
                add_feature,
            } => {
                let Some(comp_idx) = self
                    .project
                    .components
                    .iter()
                    .position(|c| c.id == component_id)
                else {
                    self.mode = InputMode::Navigate;
                    self.status_message = format!("Component '{component_id}' not found");
                    return;
                };

                let mut candidate = self.project.components[comp_idx].clone();
                candidate.name = name.clone();
                candidate.description = description;
                candidate.relative_path = relative_path;
                candidate.tags = tags;

                if let Some(title) = add_feature {
                    let mut next_num = candidate.features.len() + 1;
                    let mut feat_id = format!("{component_id}_f{next_num}");
                    while candidate.features.iter().any(|f| f.id == feat_id) {
                        next_num += 1;
                        feat_id = format!("{component_id}_f{next_num}");
                    }
                    candidate.features.push(FeatureItem {
                        id: feat_id,
                        title,
                        pushed: false,
                        external_id: None,
                    });
                }

                let save_result = if self.root.join(ROCKET_DIR).exists() {
                    save_component(&self.root, &candidate)
                } else {
                    Ok(())
                };

                match save_result {
                    Ok(()) => {
                        self.project.components[comp_idx] = candidate;
                        self.status_message = format!("Updated component {name}");
                    }
                    Err(err) => {
                        let msg = format!("Failed to save {name}: {err}");
                        if let ModalState::EditComponent(ref mut form) = modal {
                            form.error = Some(msg.clone());
                        }
                        self.status_message = msg;
                        self.modal = Some(modal);
                        self.mode = InputMode::Modal;
                        return;
                    }
                }
                self.refresh_audit();
                self.mode = InputMode::Navigate;
            }
            ModalAction::UpdateConnection {
                from,
                to,
                kind,
                label,
            } => {
                let mut candidate_config = self.project.config.clone();
                let Some(conn) = candidate_config
                    .connections
                    .iter_mut()
                    .find(|c| c.from == from && c.to == to)
                else {
                    self.mode = InputMode::Navigate;
                    self.status_message = format!("Connection {from} -> {to} not found");
                    return;
                };

                conn.kind = kind;
                conn.label = label;

                let save_result = if self.root.join(ROCKET_DIR).exists() {
                    save_project_config(&self.root, &candidate_config)
                } else {
                    Ok(())
                };

                match save_result {
                    Ok(()) => {
                        self.project.config = candidate_config;
                        self.status_message = format!("Updated connection {from} -> {to}");
                    }
                    Err(err) => {
                        let msg = format!("Failed to save connections: {err}");
                        if let ModalState::EditConnection(ref mut form) = modal {
                            form.error = Some(msg.clone());
                        }
                        self.status_message = msg;
                        self.modal = Some(modal);
                        self.mode = InputMode::Modal;
                        return;
                    }
                }
                self.refresh_audit();
                self.mode = InputMode::Navigate;
            }
            ModalAction::SelectTheme { name } => {
                let resolved_theme = match crate::theme::resolve_theme(&name, &self.theme_dirs) {
                    Ok(theme) => theme,
                    Err(err) => {
                        let msg = format!("Failed to load theme '{name}': {err}");
                        if let ModalState::ThemeSwitcher(ref mut form) = modal {
                            form.error = Some(msg.clone());
                        }
                        self.status_message = msg;
                        self.modal = Some(modal);
                        return;
                    }
                };

                let mut candidate_config = self.project.config.clone();
                candidate_config.theme = name.clone();

                if self.root.join(ROCKET_DIR).exists() {
                    if let Err(err) = save_project_config(&self.root, &candidate_config) {
                        let msg = format!("Failed to save theme '{name}': {err}");
                        if let ModalState::ThemeSwitcher(ref mut form) = modal {
                            form.error = Some(msg.clone());
                        }
                        self.status_message = msg;
                        self.modal = Some(modal);
                        return;
                    }
                }

                self.project.config = candidate_config;
                self.theme = resolved_theme;
                self.mode = InputMode::Navigate;
                self.status_message = format!("Theme switched to {name}");
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        if let Some(mut modal) = self.modal.take() {
            let action = modal.handle_key(key);
            self.apply_modal_action(modal, action);
            return;
        }

        match key.code {
            KeyCode::Char('q') if self.mode != InputMode::Modal => {
                self.should_quit = true;
            }
            KeyCode::Esc => {
                self.connect_source_id = None;
                self.mode = InputMode::Navigate;
                self.status_message = "Mode: Navigate".to_string();
            }
            KeyCode::Tab => self.cycle_focus_next(),
            KeyCode::BackTab => self.cycle_focus_prev(),
            KeyCode::Char('f') => self.toggle_gaps_filter(),
            KeyCode::Char('n') => {
                self.connect_source_id = None;
                self.modal = Some(ModalState::new_component());
                self.mode = InputMode::Modal;
                self.status_message = "New Component".to_string();
            }
            KeyCode::Char('e') => {
                if let Some(comp) = self.selected_component() {
                    let name = comp.name.clone();
                    self.modal = Some(ModalState::edit_component(comp));
                    self.connect_source_id = None;
                    self.mode = InputMode::Modal;
                    self.status_message = format!("Editing {name}");
                } else {
                    self.status_message = "No component selected to edit".to_string();
                }
            }
            KeyCode::Char('E') => {
                if let Some(comp) = self.selected_component() {
                    let comp_id = comp.id.clone();
                    let comp_name = comp.name.clone();
                    if let Some(conn) = self
                        .project
                        .config
                        .connections
                        .iter()
                        .find(|c| c.from == comp_id)
                    {
                        self.modal = Some(ModalState::edit_connection(
                            &conn.from,
                            &conn.to,
                            conn.kind,
                            conn.label.as_deref(),
                        ));
                        self.connect_source_id = None;
                        self.mode = InputMode::Modal;
                        self.status_message =
                            format!("Editing connection {} -> {}", conn.from, conn.to);
                    } else {
                        self.status_message =
                            format!("No outgoing connection from {comp_name} to edit");
                    }
                } else {
                    self.status_message = "No component selected".to_string();
                }
            }
            KeyCode::Char('m') => {
                if self.mode == InputMode::MoveCard {
                    self.mode = InputMode::Navigate;
                    self.status_message = "Mode: Navigate".to_string();
                } else {
                    self.connect_source_id = None;
                    self.mode = InputMode::MoveCard;
                    self.status_message = "Mode: Move Card".to_string();
                }
            }
            KeyCode::Char('c') => {
                if let Some(comp) = self.selected_component() {
                    let id = comp.id.clone();
                    let name = comp.name.clone();
                    self.connect_source_id = Some(id);
                    self.mode = InputMode::Connect;
                    self.status_message =
                        format!("Connect from {name}: select target and press Enter");
                } else {
                    self.status_message = "No component selected to connect".to_string();
                }
            }
            KeyCode::Char('v') => {
                if let Some(comp) = self.selected_component() {
                    let id = comp.id.clone();
                    let name = comp.name.clone();
                    self.connect_source_id = None;
                    self.mode = InputMode::Navigate;
                    self.pending_editor_component = Some(id);
                    self.status_message = format!("Opening {name} in $EDITOR...");
                } else {
                    self.status_message = "No component selected to open in $EDITOR".to_string();
                }
            }
            KeyCode::Char('r') | KeyCode::Char('S') => {
                self.connect_source_id = None;
                self.mode = InputMode::Navigate;
                self.pending_sync = Some(SyncRequest::Pull);
                self.status_message = "Syncing status from active adapters...".to_string();
            }
            KeyCode::Char('p') => {
                self.connect_source_id = None;
                self.mode = InputMode::Navigate;
                if let Some(comp) = self.selected_component() {
                    let name = comp.name.clone();
                    self.pending_sync = Some(SyncRequest::PushSelected);
                    self.status_message = format!("Pushing {name} to active adapters...");
                } else {
                    self.status_message = "No component selected to push".to_string();
                }
            }
            KeyCode::Char('?') => {
                self.connect_source_id = None;
                self.modal = Some(ModalState::help());
                self.mode = InputMode::Modal;
                self.status_message = "Keyboard Help".to_string();
            }
            KeyCode::Char('t') => {
                self.connect_source_id = None;
                let themes = crate::theme::list_available_themes(&self.theme_dirs);
                self.modal = Some(ModalState::theme_switcher(
                    themes,
                    &self.project.config.theme,
                ));
                self.mode = InputMode::Modal;
                self.status_message = "Switch Theme".to_string();
            }
            KeyCode::Enter if self.mode == InputMode::Connect => {
                self.confirm_connect();
            }
            KeyCode::Left | KeyCode::Char('h') => self.handle_direction_key(Direction::Left),
            KeyCode::Right | KeyCode::Char('l') => self.handle_direction_key(Direction::Right),
            KeyCode::Up | KeyCode::Char('k') => self.handle_direction_key(Direction::Up),
            KeyCode::Down | KeyCode::Char('j') => self.handle_direction_key(Direction::Down),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ComponentKind, ExternalTaskRef, GridConfig, GridPos, ProjectConfig, TaskStatus,
    };
    use crate::theme::default_theme;

    fn make_component(id: &str) -> Component {
        Component {
            id: id.to_string(),
            name: id.to_string(),
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
        }
    }

    fn make_app(components: Vec<Component>) -> App {
        let project = Project {
            config: ProjectConfig {
                name: "Test Project".to_string(),
                description: "App state test".to_string(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig::default(),
                connections: Vec::new(),
            },
            components,
        };
        let theme = default_theme().expect("default theme");
        App::new(PathBuf::from("/tmp/test-project"), project, theme)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn app_cycles_focus_and_quits_on_q() {
        let mut app = make_app(vec![make_component("comp_a"), make_component("comp_b")]);
        assert_eq!(app.focus, FocusColumn::List);

        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus, FocusColumn::Canvas);

        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus, FocusColumn::Detail);

        app.handle_key(key(KeyCode::BackTab));
        assert_eq!(app.focus, FocusColumn::Canvas);

        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn app_gaps_filter_narrows_visible_components() {
        let mut tracked = make_component("tracked");
        tracked.external_tasks.push(ExternalTaskRef {
            external_id: "TASK-1".to_string(),
            title: "Completed task".to_string(),
            status: TaskStatus::Complete,
        });
        let view_box = make_component("view_box");

        let mut app = make_app(vec![tracked, view_box]);
        assert_eq!(app.gap_count(), 1);
        assert_eq!(app.visible_component_indices(), vec![0, 1]);

        app.handle_key(key(KeyCode::Char('f')));
        assert_eq!(app.visible_component_indices(), vec![1]);
        assert_eq!(app.selected_component().unwrap().id, "view_box");
    }

    #[test]
    fn app_detail_focus_scrolls_instead_of_changing_selection() {
        let mut app = make_app(vec![make_component("comp_a"), make_component("comp_b")]);
        app.focus = FocusColumn::Detail;

        app.handle_key(key(KeyCode::Char('j')));
        app.handle_key(key(KeyCode::Char('j')));
        app.handle_key(key(KeyCode::Char('k')));

        assert_eq!(app.detail_scroll, 1);
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn app_canvas_directional_selection_moves_to_nearest_in_direction() {
        let mut a = make_component("alpha");
        a.position = GridPos { col: 0, row: 0 };
        let mut b = make_component("beta");
        b.position = GridPos { col: 2, row: 0 };
        let mut c = make_component("gamma");
        c.position = GridPos { col: 1, row: 0 };
        let mut d = make_component("delta");
        d.position = GridPos { col: 1, row: 2 };

        let mut app = make_app(vec![a, b, c, d]);
        app.focus = FocusColumn::Canvas;
        app.detail_scroll = 3;

        // Right from (0,0): nearest is gamma at (1,0), not beta at (2,0)
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.selected_component().unwrap().id, "gamma");
        assert_eq!(app.detail_scroll, 0);

        // Down from (1,0): delta at (1,2)
        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.selected_component().unwrap().id, "delta");

        // Down again: no component below, selection unchanged
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.selected_component().unwrap().id, "delta");

        // Left from (1,2): alpha at (0,0)
        app.handle_key(key(KeyCode::Char('h')));
        assert_eq!(app.selected_component().unwrap().id, "alpha");

        // Up from (0,0): none above, selection unchanged
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.selected_component().unwrap().id, "alpha");
    }

    #[test]
    fn app_move_card_moves_clamps_blocks_collisions_and_persists() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "Move Test", "Move card persistence").expect("init workspace");

        let mut a = make_component("alpha");
        a.position = GridPos { col: 0, row: 0 };
        let mut b = make_component("beta");
        b.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &a).expect("save alpha");
        save_component(dir.path(), &b).expect("save beta");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Toggle MoveCard on
        app.handle_key(key(KeyCode::Char('m')));
        assert_eq!(app.mode, InputMode::MoveCard);

        // Attempt to move alpha right into beta's occupied cell (1, 0)
        app.handle_key(key(KeyCode::Right));
        assert_eq!(
            app.selected_component().unwrap().position,
            GridPos { col: 0, row: 0 }
        );
        assert!(app.status_message.contains("occupied"));

        // Clamp at (0, 0) when moving left/up
        app.handle_key(key(KeyCode::Left));
        app.handle_key(key(KeyCode::Up));
        assert_eq!(
            app.selected_component().unwrap().position,
            GridPos { col: 0, row: 0 }
        );

        // Move alpha down into unoccupied (0, 1)
        app.handle_key(key(KeyCode::Down));
        assert_eq!(
            app.selected_component().unwrap().position,
            GridPos { col: 0, row: 1 }
        );

        // Verify persisted on disk
        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(
            reloaded.component("alpha").unwrap().position,
            GridPos { col: 0, row: 1 }
        );

        // Toggle MoveCard off with 'm'
        app.handle_key(key(KeyCode::Char('m')));
        assert_eq!(app.mode, InputMode::Navigate);
    }

    #[test]
    fn app_connect_mode_adds_and_removes_edge_and_rejects_self_connection() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "Connect Test", "Edge toggle persistence")
            .expect("init workspace");

        let mut a = make_component("alpha");
        a.position = GridPos { col: 0, row: 0 };
        let mut b = make_component("beta");
        b.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &a).expect("save alpha");
        save_component(dir.path(), &b).expect("save beta");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        app.focus = FocusColumn::Canvas;

        // Self-connection is rejected
        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.mode, InputMode::Connect);
        assert_eq!(app.connect_source_id.as_deref(), Some("alpha"));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.project.config.connections.is_empty());
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.connect_source_id.is_none());

        // Esc cancels Connect mode and clears connect_source_id
        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.connect_source_id.as_deref(), Some("alpha"));
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.connect_source_id.is_none());

        // Add edge alpha -> beta
        app.handle_key(key(KeyCode::Char('c')));
        app.handle_key(key(KeyCode::Right));
        assert_eq!(app.selected_component().unwrap().id, "beta");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.connect_source_id.is_none());
        assert_eq!(app.project.config.connections.len(), 1);
        assert_eq!(app.project.config.connections[0].from, "alpha");
        assert_eq!(app.project.config.connections[0].to, "beta");
        assert_eq!(app.project.config.connections[0].kind, EdgeKind::DependsOn);

        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(reloaded.config.connections.len(), 1);

        // Toggle same edge alpha -> beta removes it
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.selected_component().unwrap().id, "alpha");
        app.handle_key(key(KeyCode::Char('c')));
        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Enter));

        assert!(app.project.config.connections.is_empty());
        let reloaded_after = load_project(dir.path()).expect("reload project");
        assert!(reloaded_after.config.connections.is_empty());
    }

    fn type_keys(app: &mut App, text: &str) {
        for ch in text.chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
    }

    #[test]
    fn app_new_component_modal_scaffolds_persists_selects_and_rejects_duplicate() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "Modal Test", "New component modal test")
            .expect("init workspace");

        let mut existing = make_component("alpha");
        existing.position = GridPos { col: 0, row: 0 };
        save_component(dir.path(), &existing).expect("save alpha");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Open New Component modal with 'n'
        app.handle_key(key(KeyCode::Char('n')));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());

        // Type ID, tab to Name, type Name, tab to Archetype, cycle right to DataStream, submit
        type_keys(&mut app, "telemetry_bus");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Telemetry Bus");
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Enter));

        // Modal closes back to Navigate, component is added, scaffolded, selected, and persisted
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(app.project.components.len(), 2);
        let selected = app.selected_component().expect("new component selected");
        assert_eq!(selected.id, "telemetry_bus");
        assert_eq!(selected.name, "Telemetry Bus");
        assert_eq!(selected.kind, ComponentKind::DataStream);
        assert_eq!(selected.position, GridPos { col: 1, row: 0 });
        assert!(!selected.features.is_empty());

        let reloaded = load_project(dir.path()).expect("reload project");
        assert!(reloaded.component("telemetry_bus").is_some());

        // Duplicate ID is rejected cleanly and keeps modal open with error
        app.handle_key(key(KeyCode::Char('n')));
        type_keys(&mut app, "telemetry_bus");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Duplicate");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        assert_eq!(app.project.components.len(), 2);
        if let Some(ModalState::NewComponent(ref form)) = app.modal {
            assert!(form
                .error
                .as_deref()
                .unwrap_or("")
                .contains("already exists"));
        } else {
            panic!("expected NewComponent modal to remain open");
        }

        // Esc cancels back to Navigate
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
    }

    #[test]
    fn app_edit_component_and_connection_modals_update_and_persist() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "Edit Modal Test", "Edit component and edge")
            .expect("init workspace");

        let mut a = make_component("alpha");
        a.position = GridPos { col: 0, row: 0 };
        let mut b = make_component("beta");
        b.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &a).expect("save alpha");
        save_component(dir.path(), &b).expect("save beta");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // 'E' with no outgoing connection sets a helpful status message
        app.handle_key(key(KeyCode::Char('E')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert!(app.status_message.contains("No outgoing connection"));

        // Edit selected component ('alpha') with 'e' and add a feature
        app.handle_key(key(KeyCode::Char('e')));
        assert_eq!(app.mode, InputMode::Modal);
        app.handle_key(key(KeyCode::Tab)); // description
        type_keys(&mut app, "Core alpha service");
        app.handle_key(key(KeyCode::Tab)); // path
        type_keys(&mut app, "src/alpha.rs");
        app.handle_key(key(KeyCode::Tab)); // tags
        type_keys(&mut app, "core, api");
        app.handle_key(key(KeyCode::Tab)); // add feature
        type_keys(&mut app, "Expose health endpoint");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        let alpha = app.project.component("alpha").expect("alpha exists");
        assert_eq!(alpha.description, "Core alpha service");
        assert_eq!(alpha.relative_path.as_deref(), Some("src/alpha.rs"));
        assert_eq!(alpha.tags, vec!["core", "api"]);
        assert_eq!(alpha.features.len(), 1);
        assert_eq!(alpha.features[0].title, "Expose health endpoint");

        // Add an outgoing connection and edit it with 'E'
        app.project.config.connections.push(Connection {
            from: "alpha".to_string(),
            to: "beta".to_string(),
            kind: EdgeKind::DependsOn,
            label: None,
        });
        app.handle_key(key(KeyCode::Char('E')));
        assert_eq!(app.mode, InputMode::Modal);
        app.handle_key(key(KeyCode::Right)); // DependsOn -> DataFlow
        app.handle_key(key(KeyCode::Tab)); // label
        type_keys(&mut app, "events");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(app.project.config.connections[0].kind, EdgeKind::DataFlow);
        assert_eq!(
            app.project.config.connections[0].label.as_deref(),
            Some("events")
        );

        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(reloaded.component("alpha").unwrap().features.len(), 1);
        assert_eq!(reloaded.config.connections[0].kind, EdgeKind::DataFlow);
        assert_eq!(
            reloaded.config.connections[0].label.as_deref(),
            Some("events")
        );
    }

    #[test]
    fn app_v_key_queues_editor_and_reloads_markdown_from_disk() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "Editor Test", "Markdown reload test").expect("init workspace");

        let mut comp = make_component("alpha");
        comp.markdown_body = "# Alpha\nInitial spec\n".to_string();
        save_component(dir.path(), &comp).expect("save alpha");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        assert!(app.pending_editor_component.is_none());

        let md_path = dir
            .path()
            .join(ROCKET_DIR)
            .join(COMPONENTS_DIR)
            .join("alpha.md");
        let updated_md = "# Alpha\nUpdated spec from external editor\n";
        std::fs::write(&md_path, updated_md).expect("write updated markdown");

        app.handle_key(key(KeyCode::Char('v')));
        assert_eq!(app.pending_editor_component.as_deref(), Some("alpha"));

        let comp_id = app
            .pending_editor_component
            .take()
            .expect("pending editor component id");
        app.reload_component_markdown(&comp_id)
            .expect("reload component markdown");

        assert_eq!(
            app.project
                .component("alpha")
                .expect("alpha exists")
                .markdown_body,
            updated_md
        );
        assert!(app.status_message.contains("Reloaded"));
    }

    #[test]
    fn app_mutations_rollback_in_memory_state_on_disk_save_failure() {
        use crate::storage::{init_workspace, load_project, save_component, PROJECT_FILE};

        let dir = tempfile::tempdir().expect("create tempdir");
        init_workspace(dir.path(), "Rollback Test", "Transactional save failure")
            .expect("init workspace");

        let mut a = make_component("alpha");
        a.position = GridPos { col: 0, row: 0 };
        a.description = "Original alpha".to_string();
        let mut b = make_component("beta");
        b.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &a).expect("save alpha");
        save_component(dir.path(), &b).expect("save beta");

        let mut project = load_project(dir.path()).expect("load project");
        project.config.connections.push(Connection {
            from: "alpha".to_string(),
            to: "beta".to_string(),
            kind: EdgeKind::DependsOn,
            label: Some("initial".to_string()),
        });
        save_project_config(dir.path(), &project.config).expect("save initial connections");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Break component saves by replacing .rocket/components with a regular file
        let components_path = dir.path().join(ROCKET_DIR).join(COMPONENTS_DIR);
        std::fs::remove_dir_all(&components_path).expect("remove components dir");
        std::fs::write(&components_path, "not a directory").expect("create blocking file");

        // 1. move_selected_card rolls back position on save failure
        app.handle_key(key(KeyCode::Char('m')));
        app.handle_key(key(KeyCode::Down));
        assert_eq!(
            app.project.component("alpha").unwrap().position,
            GridPos { col: 0, row: 0 }
        );
        assert!(app.status_message.contains("Failed to save"));
        app.handle_key(key(KeyCode::Char('m'))); // exit MoveCard

        // 2. UpdateComponent rolls back fields and features on save failure
        app.handle_key(key(KeyCode::Char('e')));
        app.handle_key(key(KeyCode::Tab)); // description
        type_keys(&mut app, " mutated");
        app.handle_key(key(KeyCode::Tab)); // path
        app.handle_key(key(KeyCode::Tab)); // tags
        app.handle_key(key(KeyCode::Tab)); // add feature
        type_keys(&mut app, "Unpersisted feature");
        app.handle_key(key(KeyCode::Enter));

        let alpha = app.project.component("alpha").unwrap();
        assert_eq!(alpha.description, "Original alpha");
        assert!(alpha.features.is_empty());
        assert!(app.status_message.contains("Failed to save"));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        if let Some(ModalState::EditComponent(ref form)) = app.modal {
            assert!(form.error.is_some());
            assert!(form
                .error
                .as_deref()
                .unwrap_or("")
                .contains("Failed to save"));
        } else {
            panic!("expected EditComponent modal to remain open on save failure");
        }
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());

        // Break project.toml saves by replacing .rocket/project.toml with a directory
        let project_file = dir.path().join(ROCKET_DIR).join(PROJECT_FILE);
        std::fs::remove_file(&project_file).expect("remove project.toml");
        std::fs::create_dir(&project_file).expect("replace project.toml with dir");

        // 3. confirm_connect rolls back connection removal on save failure
        app.focus = FocusColumn::Canvas;
        app.handle_key(key(KeyCode::Char('c')));
        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.project.config.connections.len(), 1);
        assert_eq!(app.project.config.connections[0].from, "alpha");
        assert_eq!(app.project.config.connections[0].to, "beta");
        assert!(app.status_message.contains("Failed to save connections"));

        // 4. UpdateConnection rolls back kind/label on save failure and keeps modal open
        app.handle_key(key(KeyCode::Left)); // back to alpha
        app.handle_key(key(KeyCode::Char('E')));
        app.handle_key(key(KeyCode::Right)); // DependsOn -> DataFlow
        app.handle_key(key(KeyCode::Tab)); // label
        type_keys(&mut app, "_mut");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.project.config.connections[0].kind, EdgeKind::DependsOn);
        assert_eq!(
            app.project.config.connections[0].label.as_deref(),
            Some("initial")
        );
        assert!(app.status_message.contains("Failed to save connections"));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        if let Some(ModalState::EditConnection(ref form)) = app.modal {
            assert!(form.error.is_some());
            assert!(form
                .error
                .as_deref()
                .unwrap_or("")
                .contains("Failed to save connections"));
        } else {
            panic!("expected EditConnection modal to remain open on save failure");
        }
    }

    #[test]
    fn app_create_component_rejects_when_grid_is_full() {
        let mut comp = make_component("only_card");
        comp.position = GridPos { col: 0, row: 0 };
        let mut app = make_app(vec![comp]);
        app.project.config.grid = GridConfig { cols: 1, rows: 1 };

        app.handle_key(key(KeyCode::Char('n')));
        type_keys(&mut app, "second_card");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Second Card");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        assert_eq!(app.project.components.len(), 1);
        assert_eq!(app.project.components[0].id, "only_card");
        assert_eq!(
            app.project.components[0].position,
            GridPos { col: 0, row: 0 }
        );
        assert!(app.status_message.contains("full"));
        if let Some(ModalState::NewComponent(ref form)) = app.modal {
            assert!(form.error.as_deref().unwrap_or("").contains("full"));
        } else {
            panic!("expected NewComponent modal to remain open");
        }
    }

    #[test]
    fn app_v_key_clears_connect_mode_before_queueing_editor() {
        let mut app = make_app(vec![make_component("alpha"), make_component("beta")]);
        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.mode, InputMode::Connect);
        assert_eq!(app.connect_source_id.as_deref(), Some("alpha"));

        app.handle_key(key(KeyCode::Char('v')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.connect_source_id.is_none());
        assert_eq!(app.pending_editor_component.as_deref(), Some("alpha"));
    }

    #[test]
    fn app_sync_keys_queue_expected_requests() {
        let mut app = make_app(vec![make_component("alpha"), make_component("beta")]);
        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.mode, InputMode::Connect);

        app.handle_key(key(KeyCode::Char('r')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.connect_source_id.is_none());
        assert_eq!(app.pending_sync, Some(SyncRequest::Pull));
        assert!(app.status_message.contains("Syncing"));

        app.pending_sync = None;
        app.handle_key(key(KeyCode::Char('S')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert_eq!(app.pending_sync, Some(SyncRequest::Pull));

        app.pending_sync = None;
        app.handle_key(key(KeyCode::Char('p')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert_eq!(app.pending_sync, Some(SyncRequest::PushSelected));
        assert!(app.status_message.contains("Pushing"));

        let mut empty_app = make_app(Vec::new());
        empty_app.handle_key(key(KeyCode::Char('p')));
        assert_eq!(empty_app.pending_sync, None);
        assert_eq!(empty_app.status_message, "No component selected to push");
    }

    #[test]
    fn sync_pull_updates_external_tasks_flips_status_and_persists() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut project = crate::storage::init_workspace(dir.path(), "Sync Pull", "Live pull test")
            .expect("init workspace");
        project.config.active_adapters = vec!["mock_tracker".to_string()];
        crate::storage::save_project_config(dir.path(), &project.config)
            .expect("save project config");

        let mut comp = make_component("auth_svc");
        comp.features.push(FeatureItem {
            id: "auth_svc_f1".to_string(),
            title: "Login endpoint".to_string(),
            pushed: false,
            external_id: None,
        });
        save_component(dir.path(), &comp).expect("save initial component");
        project.components.push(comp);

        let adapters_path = dir.path().join("adapters.toml");
        let adapters_toml = r#"
[[adapters]]
name = "mock_tracker"
export_format = "json"

[adapters.pull_status_cmd]
program = "sh"
args = ["-c", 'printf "%s" "{\"components\":[{\"component_id\":\"auth_svc\",\"tasks\":[{\"external_id\":\"TASK-101\",\"title\":\"Login endpoint\",\"status\":\"complete\"}]}]}"']
"#;
        std::fs::write(&adapters_path, adapters_toml).expect("write adapters.toml");

        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        assert_eq!(
            compute_card_status(app.project.component("auth_svc").expect("comp")),
            CardStatus::Planned
        );

        let summary = app.sync_pull(&adapters_path).expect("sync_pull succeeds");
        assert!(summary.contains("1 component(s) updated"));
        assert!(summary.contains("1 task(s) synced"));
        assert!(summary.contains("1 feature(s) linked"));
        assert_eq!(app.status_message, summary);

        let updated = app.project.component("auth_svc").expect("updated comp");
        assert_eq!(updated.external_tasks.len(), 1);
        assert_eq!(updated.external_tasks[0].external_id, "TASK-101");
        assert_eq!(updated.external_tasks[0].status, TaskStatus::Complete);
        assert_eq!(compute_card_status(updated), CardStatus::Complete);
        assert!(updated.features[0].pushed);
        assert_eq!(updated.features[0].external_id.as_deref(), Some("TASK-101"));

        let reloaded = crate::storage::load_project(dir.path()).expect("reload project");
        let disk_comp = reloaded.component("auth_svc").expect("disk comp");
        assert_eq!(disk_comp.external_tasks.len(), 1);
        assert_eq!(disk_comp.external_tasks[0].external_id, "TASK-101");
        assert_eq!(compute_card_status(disk_comp), CardStatus::Complete);
    }

    #[test]
    fn sync_push_selected_marks_features_pushed_and_persists() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut project = crate::storage::init_workspace(dir.path(), "Sync Push", "Live push test")
            .expect("init workspace");
        project.config.active_adapters = vec!["mock_tracker".to_string()];
        crate::storage::save_project_config(dir.path(), &project.config)
            .expect("save project config");

        let mut comp = make_component("billing_api");
        comp.features.push(FeatureItem {
            id: "billing_api_f1".to_string(),
            title: "Invoice webhook".to_string(),
            pushed: false,
            external_id: None,
        });
        save_component(dir.path(), &comp).expect("save initial component");
        project.components.push(comp);

        let adapters_path = dir.path().join("adapters.toml");
        let adapters_toml = r#"
[[adapters]]
name = "mock_tracker"
export_format = "json"

[adapters.push_component_cmd]
program = "sh"
args = ["-c", 'printf "%s" "{\"external_id\":\"COMP-1\"}"']

[adapters.push_feature_cmd]
program = "sh"
args = ["-c", 'printf "%s" "{\"external_id\":\"FEAT-42\"}"']
"#;
        std::fs::write(&adapters_path, adapters_toml).expect("write adapters.toml");

        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        let summary = app
            .sync_push_selected(&adapters_path)
            .expect("sync_push_selected succeeds");
        assert!(summary.contains("1 component(s)"));
        assert!(summary.contains("1 feature(s)"));
        assert_eq!(app.status_message, summary);

        let updated = app.project.component("billing_api").expect("updated comp");
        assert!(updated.features[0].pushed);
        assert_eq!(updated.features[0].external_id.as_deref(), Some("FEAT-42"));

        let reloaded = crate::storage::load_project(dir.path()).expect("reload project");
        let disk_comp = reloaded.component("billing_api").expect("disk comp");
        assert!(disk_comp.features[0].pushed);
        assert_eq!(
            disk_comp.features[0].external_id.as_deref(),
            Some("FEAT-42")
        );
    }

    #[test]
    fn sync_push_selected_pushes_unpushed_features_to_all_active_adapters() {
        use crate::adapter::{AdapterConfig, AdaptersFile, CommandSpec, ExportFormat};

        let dir = tempfile::tempdir().expect("tempdir");
        let log_a = dir.path().join("tracker_a.log");
        let log_b = dir.path().join("tracker_b.log");

        let mut project =
            crate::storage::init_workspace(dir.path(), "Multi Push", "Multi-adapter TUI push test")
                .expect("init workspace");
        project.config.active_adapters = vec!["tracker_a".to_string(), "tracker_b".to_string()];
        crate::storage::save_project_config(dir.path(), &project.config)
            .expect("save project config");

        let mut comp = make_component("billing_api");
        comp.features.push(FeatureItem {
            id: "billing_api_f1".to_string(),
            title: "Invoice webhook".to_string(),
            pushed: false,
            external_id: None,
        });
        save_component(dir.path(), &comp).expect("save initial component");
        project.components.push(comp);

        let adapters_path = dir.path().join("adapters.toml");
        let adapters_file = AdaptersFile {
            adapters: vec![
                AdapterConfig {
                    name: "tracker_a".to_string(),
                    export_format: ExportFormat::Json,
                    push_component_cmd: None,
                    push_feature_cmd: Some(CommandSpec {
                        program: "sh".to_string(),
                        args: vec![
                            "-c".to_string(),
                            format!(
                                "cat >> '{}' && printf '{{\"external_id\":\"A-42\"}}'",
                                log_a.display()
                            ),
                        ],
                    }),
                    pull_status_cmd: None,
                },
                AdapterConfig {
                    name: "tracker_b".to_string(),
                    export_format: ExportFormat::Json,
                    push_component_cmd: None,
                    push_feature_cmd: Some(CommandSpec {
                        program: "sh".to_string(),
                        args: vec![
                            "-c".to_string(),
                            format!(
                                "cat >> '{}' && printf '{{\"external_id\":\"B-99\"}}'",
                                log_b.display()
                            ),
                        ],
                    }),
                    pull_status_cmd: None,
                },
            ],
        };
        std::fs::write(
            &adapters_path,
            toml::to_string_pretty(&adapters_file).expect("serialize adapters.toml"),
        )
        .expect("write adapters.toml");

        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        let summary = app
            .sync_push_selected(&adapters_path)
            .expect("sync_push_selected succeeds");
        assert!(summary.contains("2 feature(s) across 2 adapter(s)"));
        assert_eq!(app.status_message, summary);

        let payload_a = std::fs::read_to_string(&log_a).expect("read tracker_a log");
        let payload_b = std::fs::read_to_string(&log_b).expect("read tracker_b log");
        assert!(payload_a.contains("billing_api_f1"));
        assert!(payload_b.contains("billing_api_f1"));

        let updated = app.project.component("billing_api").expect("updated comp");
        assert!(updated.features[0].pushed);
        assert_eq!(updated.features[0].external_id.as_deref(), Some("A-42"));

        let reloaded = crate::storage::load_project(dir.path()).expect("reload project");
        let disk_comp = reloaded.component("billing_api").expect("disk comp");
        assert!(disk_comp.features[0].pushed);
        assert_eq!(disk_comp.features[0].external_id.as_deref(), Some("A-42"));
    }

    #[test]
    fn app_theme_switcher_applies_persists_and_rolls_back_on_error() {
        use crate::storage::{init_workspace, load_project, PROJECT_FILE};
        use crate::theme::DEFAULT_CRT_AMBER_RON;

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Theme Test", "Theme switcher persistence")
            .expect("init workspace");

        let themes_dir = dir.path().join("custom_themes");
        std::fs::create_dir_all(&themes_dir).expect("create themes dir");
        let amber_alt_ron = DEFAULT_CRT_AMBER_RON.replace("\"crt_amber\"", "\"amber_alt\"");
        std::fs::write(themes_dir.join("amber_alt.ron"), amber_alt_ron)
            .expect("write amber_alt.ron");
        std::fs::write(themes_dir.join("broken_theme.ron"), "not valid ron (")
            .expect("write broken_theme.ron");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        app.theme_dirs = vec![themes_dir];

        // Open '?' help overlay and close it with '?'
        app.handle_key(key(KeyCode::Char('?')));
        assert_eq!(app.mode, InputMode::Modal);
        assert_eq!(app.modal, Some(ModalState::Help));
        app.handle_key(key(KeyCode::Char('?')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());

        // Open 't' theme switcher: sorted list is ["amber_alt", "broken_theme", "crt_amber"]
        // Pre-selected index should be 2 ("crt_amber")
        app.handle_key(key(KeyCode::Char('t')));
        assert_eq!(app.mode, InputMode::Modal);
        let Some(ModalState::ThemeSwitcher(ref form)) = app.modal else {
            panic!("expected ThemeSwitcher modal");
        };
        assert_eq!(form.selected_theme(), Some("crt_amber"));

        // Move Up to "broken_theme" and press Enter -> stays open with error
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Modal);
        assert_eq!(app.theme.name, "crt_amber");
        assert_eq!(app.project.config.theme, "crt_amber");
        let Some(ModalState::ThemeSwitcher(ref form)) = app.modal else {
            panic!("expected ThemeSwitcher modal to remain open on parse error");
        };
        assert!(form.error.is_some());

        // Move Up to "amber_alt" and press Enter -> applies and persists
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(app.theme.name, "amber_alt");
        assert_eq!(app.project.config.theme, "amber_alt");
        assert_eq!(app.status_message, "Theme switched to amber_alt");

        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(reloaded.config.theme, "amber_alt");

        // Break project.toml to verify rollback on save failure
        let project_file = dir.path().join(ROCKET_DIR).join(PROJECT_FILE);
        std::fs::remove_file(&project_file).expect("remove project.toml");
        std::fs::create_dir(&project_file).expect("replace project.toml with dir");

        app.handle_key(key(KeyCode::Char('t')));
        // Currently on "amber_alt" (index 0); move Up wraps to "crt_amber" (index 2)
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Modal);
        assert_eq!(app.theme.name, "amber_alt");
        assert_eq!(app.project.config.theme, "amber_alt");
        assert!(app.status_message.contains("Failed to save theme"));
    }
}
