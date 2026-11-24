use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::archetype::{scaffold_component, scaffold_sub_component};
use crate::error::{RbError, RbResult};
use crate::model::{ComponentKind, Connection, EdgeKind, FeatureItem};
use crate::storage::{
    delete_component, ensure_contained, save_component, save_project_config, validate_component_id,
    COMPONENTS_DIR, ROCKET_DIR,
};
use crate::tui::modal::{ModalAction, ModalState};

use super::grid::ensure_grid_contains;
use super::state::{App, Direction, FocusColumn, InputMode, SyncRequest};

impl App {
    pub fn reload_component_markdown(&mut self, component_id: &str) -> RbResult<()> {
        validate_component_id(component_id)?;

        let components_dir = self.root.join(ROCKET_DIR).join(COMPONENTS_DIR);
        let md_path = components_dir.join(format!("{component_id}.md"));
        let canon_md = ensure_contained(&components_dir, &md_path)?;

        let body = std::fs::read_to_string(&canon_md).map_err(|source| RbError::Io {
            path: canon_md,
            source,
        })?;

        let comp = self
            .project
            .component_mut(component_id)
            .ok_or_else(|| RbError::NotFound(component_id.to_string()))?;
        let comp_name = comp.name.clone();
        comp.markdown_body = body;

        self.refresh_audit();
        self.status_message = format!("Reloaded Markdown for {comp_name}");
        Ok(())
    }

    pub fn toggle_expand_selected(&mut self) {
        let Some((comp_id, comp_name)) = self
            .selected_component()
            .map(|c| (c.id.clone(), c.name.clone()))
        else {
            return;
        };

        if !self.has_sub_components(&comp_id) {
            return;
        }

        if self.expanded_components.contains(&comp_id) {
            self.expanded_components.remove(&comp_id);
            self.clamp_selection();
            self.status_message = format!("Collapsed {comp_name}");
        } else {
            let before_visible: HashSet<String> = self
                .visible_component_indices()
                .into_iter()
                .filter_map(|idx| self.project.components.get(idx).map(|c| c.id.clone()))
                .collect();

            self.expanded_components.insert(comp_id.clone());

            let newly_revealed: Vec<String> = self
                .visible_component_indices()
                .into_iter()
                .filter_map(|idx| self.project.components.get(idx).map(|c| c.id.clone()))
                .filter(|id| !before_visible.contains(id))
                .collect();

            let mut save_errors = Vec::new();
            for revealed_id in &newly_revealed {
                save_errors.extend(self.resolve_visible_collisions_for(revealed_id));
            }

            let child_count = self.project.sub_components(&comp_id).len();
            self.status_message = if save_errors.is_empty() {
                format!("Expanded {comp_name} ({child_count} sub-component(s))")
            } else {
                format!(
                    "Expanded {comp_name} ({child_count} sub-component(s)) (save warning: {})",
                    save_errors.join("; ")
                )
            };
        }
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
            format!("Connected: {source_id} depends on {target_id}")
        };

        let save_result = save_project_config(&self.root, &candidate_config);

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

    fn apply_modal_action(&mut self, mut modal: ModalState, action: ModalAction) {
        match action {
            ModalAction::None => {
                self.modal = Some(modal);
            }
            ModalAction::Cancel => {
                self.mode = InputMode::Navigate;
                self.status_message = "Mode: Navigate".to_string();
            }
            ModalAction::CreateComponent {
                id,
                name,
                kind,
                parent_id,
            } => {
                if self.project.component(&id).is_some() {
                    let err = format!("component id '{id}' already exists");
                    if let ModalState::NewComponent(ref mut form) = modal {
                        form.error = Some(err.clone());
                    }
                    self.status_message = err;
                    self.modal = Some(modal);
                    return;
                }

                let (comp, grid_expanded_config) = if let Some(ref pid) = parent_id {
                    let Some(parent_pos) = self.project.component(pid).map(|c| c.position) else {
                        let err = format!("parent component '{pid}' not found");
                        if let ModalState::NewComponent(ref mut form) = modal {
                            form.error = Some(err.clone());
                        }
                        self.status_message = err;
                        self.modal = Some(modal);
                        return;
                    };
                    let pos = self.find_proximity_grid_pos(parent_pos);
                    let mut candidate_config = self.project.config.clone();
                    let expanded = ensure_grid_contains(&mut candidate_config.grid, pos);
                    let comp = scaffold_sub_component(&id, &name, kind, pos, pid);
                    (comp, expanded.then_some(candidate_config))
                } else {
                    let Some(pos) = self.first_unoccupied_grid_pos() else {
                        let grid = self.effective_grid();
                        let err = format!(
                            "grid ({}x{}) is full; no free cell available",
                            grid.cols, grid.rows
                        );
                        if let ModalState::NewComponent(ref mut form) = modal {
                            form.error = Some(err.clone());
                        }
                        self.status_message = err;
                        self.modal = Some(modal);
                        return;
                    };
                    let mut candidate_config = self.project.config.clone();
                    let expanded = ensure_grid_contains(&mut candidate_config.grid, pos);
                    (
                        scaffold_component(&id, &name, kind, pos),
                        expanded.then_some(candidate_config),
                    )
                };

                if let Err(err) = save_component(&self.root, &comp) {
                    let msg = format!("Failed to save {name}: {err}");
                    if let ModalState::NewComponent(ref mut form) = modal {
                        form.error = Some(msg.clone());
                    }
                    self.status_message = msg;
                    self.modal = Some(modal);
                    return;
                }
                if let Some(ref cfg) = grid_expanded_config {
                    if let Err(err) = save_project_config(&self.root, cfg) {
                        let msg = match delete_component(&self.root, &comp.id) {
                            Ok(()) => format!("Failed to save grid config: {err}"),
                            Err(rb_err) => {
                                format!(
                                    "Failed to save grid config: {err} (rollback failed: {rb_err})"
                                )
                            }
                        };
                        if let ModalState::NewComponent(ref mut form) = modal {
                            form.error = Some(msg.clone());
                        }
                        self.status_message = msg;
                        self.modal = Some(modal);
                        return;
                    }
                }

                if let Some(cfg) = grid_expanded_config {
                    self.project.config = cfg;
                }
                self.project.components.push(comp);
                let new_comp_idx = self.project.components.len() - 1;
                let save_errors = if let Some(ref pid) = parent_id {
                    self.expanded_components.insert(pid.clone());
                    self.resolve_visible_collisions_for(&id)
                } else {
                    Vec::new()
                };
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
                let base_msg = if parent_id.is_some() {
                    format!("Created sub-component {name} ({id})")
                } else {
                    format!("Created component {name} ({id})")
                };
                self.status_message = if save_errors.is_empty() {
                    base_msg
                } else {
                    format!("{base_msg} (save warning: {})", save_errors.join("; "))
                };
            }
            ModalAction::UpdateComponent {
                component_id,
                name,
                kind,
                description,
                relative_path,
                tags,
                add_feature,
                add_sub_component,
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
                candidate.kind = kind;
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

                let (new_sub_comp, grid_expanded_config) =
                    if let Some(ref sub_name) = add_sub_component {
                        let sub_id = self.derive_unique_sub_component_id(&component_id, sub_name);
                        let sub_pos = self.find_proximity_grid_pos(candidate.position);
                        let mut candidate_config = self.project.config.clone();
                        let expanded = ensure_grid_contains(&mut candidate_config.grid, sub_pos);
                        let sub_comp = scaffold_sub_component(
                            &sub_id,
                            sub_name,
                            ComponentKind::Concept,
                            sub_pos,
                            &component_id,
                        );
                        (Some(sub_comp), expanded.then_some(candidate_config))
                    } else {
                        (None, None)
                    };

                let save_result = save_component(&self.root, &candidate).and_then(|()| {
                    if let Some(ref sub_comp) = new_sub_comp {
                        save_component(&self.root, sub_comp)?;
                    }
                    if let Some(ref cfg) = grid_expanded_config {
                        save_project_config(&self.root, cfg)?;
                    }
                    Ok(())
                });

                match save_result {
                    Ok(()) => {
                        self.project.components[comp_idx] = candidate;
                        if let Some(cfg) = grid_expanded_config {
                            self.project.config = cfg;
                        }
                        if let Some(sub_comp) = new_sub_comp {
                            let sub_id = sub_comp.id.clone();
                            let sub_name = sub_comp.name.clone();
                            self.project.components.push(sub_comp);
                            let save_errors = if self.expanded_components.contains(&component_id) {
                                self.resolve_visible_collisions_for(&sub_id)
                            } else {
                                Vec::new()
                            };
                            let base_msg = format!(
                                "Updated component {name} and added sub-component {sub_name} ({sub_id})"
                            );
                            self.status_message = if save_errors.is_empty() {
                                base_msg
                            } else {
                                format!("{base_msg} (save warning: {})", save_errors.join("; "))
                            };
                        } else {
                            self.status_message = format!("Updated component {name}");
                        }
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

                let prev_kind = conn.kind;
                if kind == EdgeKind::Contains && self.is_descendant_of(&from, &to) {
                    let msg =
                        format!("Cannot set Contains: '{from}' is already a descendant of '{to}'");
                    if let ModalState::EditConnection(ref mut form) = modal {
                        form.error = Some(msg.clone());
                    }
                    self.status_message = msg;
                    self.modal = Some(modal);
                    self.mode = InputMode::Modal;
                    return;
                }

                conn.kind = kind;
                conn.label = label;

                let updated_to_comp = if kind == EdgeKind::Contains {
                    self.project.component(&to).map(|c| {
                        let mut updated = c.clone();
                        updated.parent_id = Some(from.clone());
                        updated
                    })
                } else if prev_kind == EdgeKind::Contains {
                    self.project.component(&to).and_then(|c| {
                        if c.parent_id.as_deref() == Some(from.as_str()) {
                            let mut updated = c.clone();
                            updated.parent_id = None;
                            Some(updated)
                        } else {
                            None
                        }
                    })
                } else {
                    None
                };

                if let Err(err) = save_project_config(&self.root, &candidate_config) {
                    let msg = format!("Failed to save connections: {err}");
                    if let ModalState::EditConnection(ref mut form) = modal {
                        form.error = Some(msg.clone());
                    }
                    self.status_message = msg;
                    self.modal = Some(modal);
                    self.mode = InputMode::Modal;
                    return;
                }
                if let Some(ref updated_comp) = updated_to_comp {
                    if let Err(err) = save_component(&self.root, updated_comp) {
                        let msg = match save_project_config(&self.root, &self.project.config) {
                            Ok(()) => format!("Failed to save component {to}: {err}"),
                            Err(rollback_err) => format!(
                                "Failed to save component {to}: {err} (rollback failed: {rollback_err})"
                            ),
                        };
                        if let ModalState::EditConnection(ref mut form) = modal {
                            form.error = Some(msg.clone());
                        }
                        self.status_message = msg;
                        self.modal = Some(modal);
                        self.mode = InputMode::Modal;
                        return;
                    }
                }

                self.project.config = candidate_config;
                if let Some(updated_comp) = updated_to_comp {
                    if let Some(slot) = self.project.component_mut(&to) {
                        *slot = updated_comp;
                    }
                }
                let save_errors =
                    if kind == EdgeKind::Contains && self.expanded_components.contains(&from) {
                        self.resolve_visible_collisions_for(&to)
                    } else {
                        Vec::new()
                    };
                self.clamp_selection();
                self.status_message = if save_errors.is_empty() {
                    format!("Updated connection {from} -> {to}")
                } else {
                    format!(
                        "Updated connection {from} -> {to} (save warning: {})",
                        save_errors.join("; ")
                    )
                };
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

                if let Err(err) = save_project_config(&self.root, &candidate_config) {
                    let msg = format!("Failed to save theme '{name}': {err}");
                    if let ModalState::ThemeSwitcher(ref mut form) = modal {
                        form.error = Some(msg.clone());
                    }
                    self.status_message = msg;
                    self.modal = Some(modal);
                    return;
                }

                self.project.config = candidate_config;
                self.theme = resolved_theme;
                self.mode = InputMode::Navigate;
                self.status_message = format!("Theme switched to {name}");
            }
            ModalAction::DeleteComponent { id } => {
                let mut candidate_config = self.project.config.clone();
                candidate_config
                    .connections
                    .retain(|c| c.from != id && c.to != id);
                let connections_changed =
                    candidate_config.connections.len() != self.project.config.connections.len();

                if connections_changed {
                    if let Err(err) = save_project_config(&self.root, &candidate_config) {
                        let msg = format!("Failed to save connections before deleting {id}: {err}");
                        if let ModalState::ConfirmDelete(ref mut form) = modal {
                            form.error = Some(msg.clone());
                        }
                        self.status_message = msg;
                        self.modal = Some(modal);
                        self.mode = InputMode::Modal;
                        return;
                    }
                }

                if let Err(err) = delete_component(&self.root, &id) {
                    if connections_changed {
                        let _ = save_project_config(&self.root, &self.project.config);
                    }
                    let msg = format!("Failed to delete component {id}: {err}");
                    if let ModalState::ConfirmDelete(ref mut form) = modal {
                        form.error = Some(msg.clone());
                    }
                    self.status_message = msg;
                    self.modal = Some(modal);
                    self.mode = InputMode::Modal;
                    return;
                }

                self.project.components.retain(|c| c.id != id);
                let mut detach_errors = Vec::new();
                for child in &mut self.project.components {
                    if child.parent_id.as_deref() == Some(id.as_str()) {
                        child.parent_id = None;
                        if let Err(err) = save_component(&self.root, child) {
                            detach_errors.push(format!("failed to detach {}: {err}", child.id));
                        }
                    }
                }
                self.expanded_components.remove(&id);
                self.project.config = candidate_config;
                self.connect_source_id = None;
                if self.pending_editor_component.as_deref() == Some(id.as_str()) {
                    self.pending_editor_component = None;
                }
                self.clamp_selection();
                self.detail_scroll = 0;
                self.refresh_audit();
                self.mode = InputMode::Navigate;
                self.status_message = if detach_errors.is_empty() {
                    format!("Deleted component {id}")
                } else {
                    format!(
                        "Deleted component {id} (warning: {})",
                        detach_errors.join("; ")
                    )
                };
            }
            ModalAction::DeleteConnection { from, to } => {
                let mut candidate_config = self.project.config.clone();
                let Some(pos) = candidate_config
                    .connections
                    .iter()
                    .position(|c| c.from == from && c.to == to)
                else {
                    self.mode = InputMode::Navigate;
                    self.status_message = format!("Connection {from} -> {to} not found");
                    return;
                };
                candidate_config.connections.remove(pos);

                if let Err(err) = save_project_config(&self.root, &candidate_config) {
                    let msg = format!("Failed to save connections: {err}");
                    if let ModalState::ConfirmDelete(ref mut form) = modal {
                        form.error = Some(msg.clone());
                    }
                    self.status_message = msg;
                    self.modal = Some(modal);
                    self.mode = InputMode::Modal;
                    return;
                }

                self.project.config = candidate_config;
                self.connect_source_id = None;
                self.refresh_audit();
                self.mode = InputMode::Navigate;
                self.status_message = format!("Deleted connection {from} -> {to}");
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
            KeyCode::Char('+') | KeyCode::Char('=') => self.zoom_in(),
            KeyCode::Char('-') => self.zoom_out(),
            KeyCode::Char('f') => self.toggle_gaps_filter(),
            KeyCode::Char('n') => {
                self.connect_source_id = None;
                self.modal = Some(ModalState::new_component());
                self.mode = InputMode::Modal;
                self.status_message = "New Component".to_string();
            }
            KeyCode::Char('a') if self.mode == InputMode::Navigate => {
                if let Some(comp) = self.selected_component() {
                    let parent_id = comp.id.clone();
                    let parent_name = comp.name.clone();
                    self.connect_source_id = None;
                    self.modal = Some(ModalState::new_sub_component(parent_id));
                    self.mode = InputMode::Modal;
                    self.status_message = format!("New Sub-Component under {parent_name}");
                } else {
                    self.status_message = "No parent component selected".to_string();
                }
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
            KeyCode::Char('d') => {
                if let Some(comp) = self.selected_component() {
                    let comp_id = comp.id.clone();
                    let comp_name = comp.name.clone();
                    let affected_connections = self
                        .project
                        .config
                        .connections
                        .iter()
                        .filter(|conn| conn.from == comp_id || conn.to == comp_id)
                        .count();
                    self.modal = Some(ModalState::confirm_delete_component(
                        comp_id.clone(),
                        comp_name,
                        affected_connections,
                    ));
                    self.connect_source_id = None;
                    self.mode = InputMode::Modal;
                    self.status_message = format!("Confirm delete component {comp_id}");
                } else {
                    self.status_message = "No component selected to delete".to_string();
                }
            }
            KeyCode::Char('D') => {
                if let Some(comp_id) = self.selected_component().map(|c| c.id.clone()) {
                    let target_conn = self
                        .project
                        .config
                        .connections
                        .iter()
                        .find(|conn| conn.from == comp_id)
                        .or_else(|| {
                            self.project
                                .config
                                .connections
                                .iter()
                                .find(|conn| conn.to == comp_id)
                        });
                    if let Some(conn) = target_conn {
                        let from = conn.from.clone();
                        let to = conn.to.clone();
                        self.modal = Some(ModalState::confirm_delete_connection(
                            from.clone(),
                            to.clone(),
                        ));
                        self.connect_source_id = None;
                        self.mode = InputMode::Modal;
                        self.status_message = format!("Confirm delete connection {from} -> {to}");
                    } else {
                        self.status_message =
                            "No connection on selected component to delete".to_string();
                    }
                } else {
                    self.status_message =
                        "No connection on selected component to delete".to_string();
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
                    self.status_message = format!(
                        "Connect: select dependency for {name} ({name} depends on <target>), Enter to confirm"
                    );
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
            KeyCode::Char(' ') | KeyCode::Enter if self.mode == InputMode::Navigate => {
                self.toggle_expand_selected();
            }
            KeyCode::Left | KeyCode::Char('h') => self.handle_direction_key(Direction::Left),
            KeyCode::Right | KeyCode::Char('l') => self.handle_direction_key(Direction::Right),
            KeyCode::Up | KeyCode::Char('k') => self.handle_direction_key(Direction::Up),
            KeyCode::Down | KeyCode::Char('j') => self.handle_direction_key(Direction::Down),
            _ => {}
        }
    }
}
