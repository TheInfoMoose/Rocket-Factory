use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::adapter::{sync_pull_project, sync_push_project};
use crate::archetype::{scaffold_component, scaffold_sub_component};
use crate::audit::{audit_project, compute_card_status, AuditFinding};
use crate::error::{RbError, RbResult};
use crate::model::{
    CardStatus, Component, ComponentKind, Connection, EdgeKind, FeatureItem, GridConfig, GridPos,
    Project,
};
use crate::storage::{
    delete_component, save_component, save_project_config, validate_component_id, COMPONENTS_DIR,
    ROCKET_DIR,
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
pub(crate) enum SyncRequest {
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
    pub last_canvas_capacity: Option<(u16, u16)>,
    pub connect_source_id: Option<String>,
    pub modal: Option<ModalState>,
    pub pending_editor_component: Option<String>,
    pub(crate) pending_sync: Option<SyncRequest>,
    pub expanded_components: HashSet<String>,
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
            last_canvas_capacity: None,
            connect_source_id: None,
            modal: None,
            pending_editor_component: None,
            pending_sync: None,
            expanded_components: HashSet::new(),
        }
    }

    pub fn refresh_audit(&mut self) {
        self.audit_findings = audit_project(&self.project);
    }

    pub fn effective_grid(&self) -> GridConfig {
        let (vis_cols, vis_rows) = self.last_canvas_capacity.unwrap_or((0, 0));
        GridConfig {
            cols: self.project.config.grid.cols.max(vis_cols),
            rows: self.project.config.grid.rows.max(vis_rows),
        }
    }

    pub fn zoom_in(&mut self) {
        self.viewport.zoom = self.viewport.zoom.zoom_in();
        self.status_message = format!("Zoom: {}", self.viewport.zoom.label());
    }

    pub fn zoom_out(&mut self) {
        self.viewport.zoom = self.viewport.zoom.zoom_out();
        self.status_message = format!("Zoom: {}", self.viewport.zoom.label());
    }

    pub fn reload_component_markdown(&mut self, component_id: &str) -> RbResult<()> {
        validate_component_id(component_id)?;

        let components_dir = self.root.join(ROCKET_DIR).join(COMPONENTS_DIR);
        let md_path = components_dir.join(format!("{component_id}.md"));

        let canon_dir = std::fs::canonicalize(&components_dir).map_err(|source| RbError::Io {
            path: components_dir,
            source,
        })?;
        let canon_md = std::fs::canonicalize(&md_path).map_err(|source| RbError::Io {
            path: md_path,
            source,
        })?;
        if !canon_md.starts_with(&canon_dir) {
            return Err(RbError::PathEscape(canon_md));
        }

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

    pub fn sync_pull(&mut self, adapters_path: &Path) -> RbResult<String> {
        if self.project.config.active_adapters.is_empty() {
            let msg = "No active adapters configured".to_string();
            self.status_message = msg.clone();
            return Ok(msg);
        }

        let report = sync_pull_project(&mut self.project, &self.root, adapters_path, None)?;
        self.refresh_audit();
        self.clamp_selection();

        let summary_msg = format!(
            "Synced {} adapter(s): {} component(s) updated, {} task(s) synced, {} feature(s) linked",
            report.adapters_count,
            report.components_updated,
            report.tasks_synced,
            report.features_linked
        );
        self.status_message = summary_msg.clone();
        Ok(summary_msg)
    }

    pub fn sync_push_selected(&mut self, adapters_path: &Path) -> RbResult<String> {
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

        let report = sync_push_project(
            &mut self.project,
            &self.root,
            adapters_path,
            std::slice::from_ref(&comp_id),
            None,
        )?;
        self.refresh_audit();
        self.clamp_selection();

        let mut summary_msg = format!(
            "Pushed {comp_id}: {} component(s), {} feature(s) across {} adapter(s)",
            report.components_pushed, report.features_pushed, report.adapters_count
        );
        if let Some(details) = report.format_responses() {
            summary_msg.push_str(&format!(" [{details}]"));
        }
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

    pub fn has_sub_components(&self, component_id: &str) -> bool {
        !self.project.sub_components(component_id).is_empty()
    }

    pub fn is_expanded(&self, component_id: &str) -> bool {
        self.expanded_components.contains(component_id)
    }

    fn matches_filter(&self, comp: &Component) -> bool {
        match self.filter {
            ListFilter::All => true,
            ListFilter::GapsOnly => compute_card_status(comp) == CardStatus::Gap,
        }
    }

    fn push_visible_component(
        &self,
        idx: usize,
        out: &mut Vec<usize>,
        visited: &mut HashSet<usize>,
    ) {
        if !visited.insert(idx) {
            return;
        }
        let Some(comp) = self.project.components.get(idx) else {
            return;
        };
        if self.matches_filter(comp) {
            out.push(idx);
        }
        if self.expanded_components.contains(&comp.id) {
            let parent_id = comp.id.as_str();
            for (child_idx, child) in self.project.components.iter().enumerate() {
                if child.parent_id.as_deref() == Some(parent_id) {
                    self.push_visible_component(child_idx, out, visited);
                }
            }
        }
    }

    pub fn visible_component_indices(&self) -> Vec<usize> {
        let existing_ids: HashSet<&str> = self
            .project
            .components
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        let mut out = Vec::new();
        let mut visited = HashSet::new();

        for (idx, comp) in self.project.components.iter().enumerate() {
            let is_top_level = match comp.parent_id.as_deref() {
                None => true,
                Some(pid) => !existing_ids.contains(pid),
            };
            if is_top_level {
                self.push_visible_component(idx, &mut out, &mut visited);
            }
        }

        out
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

        let grid = self.effective_grid();
        let max_col = grid.cols.saturating_sub(1);
        let max_row = grid.rows.saturating_sub(1);
        let current_pos = comp.position;

        let target_pos = match dir {
            Direction::Left => GridPos {
                col: current_pos.col.saturating_sub(1),
                row: current_pos.row,
            },
            Direction::Right => GridPos {
                col: current_pos
                    .col
                    .saturating_add(1)
                    .min(max_col.max(current_pos.col)),
                row: current_pos.row,
            },
            Direction::Up => GridPos {
                col: current_pos.col,
                row: current_pos.row.saturating_sub(1),
            },
            Direction::Down => GridPos {
                col: current_pos.col,
                row: current_pos
                    .row
                    .saturating_add(1)
                    .min(max_row.max(current_pos.row)),
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
        let previous = self.project.components[comp_idx].clone();
        let mut candidate = previous.clone();
        candidate.position = target_pos;

        let mut candidate_config = self.project.config.clone();
        let mut expanded = false;
        if target_pos.col >= candidate_config.grid.cols {
            candidate_config.grid.cols = target_pos.col.saturating_add(1);
            expanded = true;
        }
        if target_pos.row >= candidate_config.grid.rows {
            candidate_config.grid.rows = target_pos.row.saturating_add(1);
            expanded = true;
        }

        if let Err(err) = save_component(&self.root, &candidate) {
            self.refresh_audit();
            self.status_message = format!("Failed to save {comp_name}: {err}");
            return;
        }
        if expanded {
            if let Err(err) = save_project_config(&self.root, &candidate_config) {
                let msg = match save_component(&self.root, &previous) {
                    Ok(()) => format!("Failed to save grid config: {err}"),
                    Err(rb_err) => {
                        format!("Failed to save grid config: {err} (rollback failed: {rb_err})")
                    }
                };
                self.refresh_audit();
                self.status_message = msg;
                return;
            }
        }

        if expanded {
            self.project.config = candidate_config;
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

    fn first_unoccupied_grid_pos(&self) -> Option<GridPos> {
        let grid = self.effective_grid();
        if grid.cols == 0 || grid.rows == 0 {
            return None;
        }
        let span = self
            .last_canvas_capacity
            .map_or(grid.cols, |(c, _)| c.min(grid.cols))
            .max(1);
        let center_col = span.saturating_sub(1) / 2;
        let max_offset = center_col.max(grid.cols.saturating_sub(1).saturating_sub(center_col));

        for row in 0..grid.rows {
            for offset in 0..=max_offset {
                let mut candidates = [None, None];
                if offset == 0 {
                    candidates[0] = Some(center_col);
                } else {
                    if let Some(right) = center_col.checked_add(offset) {
                        if right < grid.cols {
                            candidates[0] = Some(right);
                        }
                    }
                    if let Some(left) = center_col.checked_sub(offset) {
                        candidates[1] = Some(left);
                    }
                }
                for col in candidates.into_iter().flatten() {
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
        }
        None
    }

    fn find_proximity_grid_pos(&self, anchor: GridPos) -> GridPos {
        let occupied: HashSet<GridPos> = self
            .visible_component_indices()
            .into_iter()
            .filter_map(|idx| self.project.components.get(idx).map(|c| c.position))
            .collect();

        let cols = self.project.config.grid.cols.max(1);
        let rows = self.project.config.grid.rows.max(1);
        let max_dist = cols.saturating_add(rows);

        let is_free_in_bounds =
            |pos: GridPos| -> bool { pos.col < cols && pos.row < rows && !occupied.contains(&pos) };

        for dist in 1..=max_dist {
            // 1. Right / below anchor first: (+dist, 0), ..., (0, +dist)
            for dr in 0..=dist {
                let dc = dist - dr;
                if let (Some(col), Some(row)) =
                    (anchor.col.checked_add(dc), anchor.row.checked_add(dr))
                {
                    let candidate = GridPos { col, row };
                    if is_free_in_bounds(candidate) {
                        return candidate;
                    }
                }
            }

            // 2. Right / above anchor: dc > 0, dr < 0
            for dr in 1..dist {
                let dc = dist - dr;
                if let (Some(col), Some(row)) =
                    (anchor.col.checked_add(dc), anchor.row.checked_sub(dr))
                {
                    let candidate = GridPos { col, row };
                    if is_free_in_bounds(candidate) {
                        return candidate;
                    }
                }
            }

            // 3. Left / below anchor: dc < 0, dr > 0
            for dc in 1..dist {
                let dr = dist - dc;
                if let (Some(col), Some(row)) =
                    (anchor.col.checked_sub(dc), anchor.row.checked_add(dr))
                {
                    let candidate = GridPos { col, row };
                    if is_free_in_bounds(candidate) {
                        return candidate;
                    }
                }
            }

            // 4. Left / above anchor: dc <= 0, dr <= 0
            for dr in 0..=dist {
                let dc = dist - dr;
                if let (Some(col), Some(row)) =
                    (anchor.col.checked_sub(dc), anchor.row.checked_sub(dr))
                {
                    let candidate = GridPos { col, row };
                    if is_free_in_bounds(candidate) {
                        return candidate;
                    }
                }
            }
        }

        // Grid is full within current bounds: expand to the right of the grid at anchor.row.
        let mut extra_col = cols;
        loop {
            let candidate = GridPos {
                col: extra_col,
                row: anchor.row,
            };
            if !occupied.contains(&candidate) {
                return candidate;
            }
            extra_col = extra_col.saturating_add(1);
        }
    }

    fn ensure_grid_contains(&mut self, pos: GridPos) -> bool {
        let mut expanded = false;
        if pos.col >= self.project.config.grid.cols {
            self.project.config.grid.cols = pos.col.saturating_add(1);
            expanded = true;
        }
        if pos.row >= self.project.config.grid.rows {
            self.project.config.grid.rows = pos.row.saturating_add(1);
            expanded = true;
        }
        expanded
    }

    fn resolve_visible_collisions_for(&mut self, protected_id: &str) -> Vec<String> {
        let Some(protected_pos) = self.project.component(protected_id).map(|c| c.position) else {
            return Vec::new();
        };

        let colliding_indices: Vec<usize> = self
            .visible_component_indices()
            .into_iter()
            .filter(|&idx| {
                self.project
                    .components
                    .get(idx)
                    .is_some_and(|c| c.id != protected_id && c.position == protected_pos)
            })
            .collect();

        let mut grid_expanded_any = false;
        let mut errors = Vec::new();

        for idx in colliding_indices {
            let new_pos = self.find_proximity_grid_pos(protected_pos);
            let Some(mut candidate) = self.project.components.get(idx).cloned() else {
                continue;
            };
            candidate.position = new_pos;
            if let Err(err) = save_component(&self.root, &candidate) {
                errors.push(format!("failed to save {}: {err}", candidate.id));
                continue;
            }
            if self.ensure_grid_contains(new_pos) {
                grid_expanded_any = true;
            }
            self.project.components[idx] = candidate;
        }

        if grid_expanded_any {
            if let Err(err) = save_project_config(&self.root, &self.project.config) {
                errors.push(format!("failed to save grid config: {err}"));
            }
        }

        errors
    }

    fn is_descendant_of(&self, node_id: &str, ancestor_id: &str) -> bool {
        if node_id == ancestor_id {
            return true;
        }
        let mut current = node_id;
        let mut visited = HashSet::new();
        while visited.insert(current) {
            let Some(comp) = self.project.component(current) else {
                return false;
            };
            let Some(parent_id) = comp.parent_id.as_deref() else {
                return false;
            };
            if parent_id == ancestor_id {
                return true;
            }
            current = parent_id;
        }
        false
    }

    fn derive_unique_sub_component_id(&self, parent_id: &str, sub_name: &str) -> String {
        let mut slug = String::new();
        let mut prev_underscore = false;
        for ch in sub_name.chars() {
            let lower = ch.to_ascii_lowercase();
            if lower.is_ascii_lowercase() || lower.is_ascii_digit() {
                slug.push(lower);
                prev_underscore = false;
            } else if !prev_underscore && !slug.is_empty() {
                slug.push('_');
                prev_underscore = true;
            }
        }
        while slug.ends_with('_') {
            slug.pop();
        }
        if slug.len() > 56 {
            slug.truncate(56);
            while slug.ends_with('_') {
                slug.pop();
            }
        }

        if validate_component_id(&slug).is_ok() {
            if self.project.component(&slug).is_none() {
                return slug;
            }
            let mut n = 2usize;
            loop {
                let candidate = format!("{slug}_{n}");
                if validate_component_id(&candidate).is_ok()
                    && self.project.component(&candidate).is_none()
                {
                    return candidate;
                }
                n += 1;
            }
        }

        let base = if parent_id.len() > 50 {
            &parent_id[..50]
        } else {
            parent_id
        };
        let mut n = 1usize;
        loop {
            let candidate = format!("{base}_sub_{n}");
            if validate_component_id(&candidate).is_ok()
                && self.project.component(&candidate).is_none()
            {
                return candidate;
            }
            n += 1;
        }
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
                    let mut expanded = false;
                    if pos.col >= candidate_config.grid.cols {
                        candidate_config.grid.cols = pos.col.saturating_add(1);
                        expanded = true;
                    }
                    if pos.row >= candidate_config.grid.rows {
                        candidate_config.grid.rows = pos.row.saturating_add(1);
                        expanded = true;
                    }
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
                    let mut expanded = false;
                    if pos.col >= candidate_config.grid.cols {
                        candidate_config.grid.cols = pos.col.saturating_add(1);
                        expanded = true;
                    }
                    if pos.row >= candidate_config.grid.rows {
                        candidate_config.grid.rows = pos.row.saturating_add(1);
                        expanded = true;
                    }
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
                        let mut expanded = false;
                        if sub_pos.col >= candidate_config.grid.cols {
                            candidate_config.grid.cols = sub_pos.col.saturating_add(1);
                            expanded = true;
                        }
                        if sub_pos.row >= candidate_config.grid.rows {
                            candidate_config.grid.rows = sub_pos.row.saturating_add(1);
                            expanded = true;
                        }
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

                if let Err(err) = crate::storage::delete_component(&self.root, &id) {
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
        render_app(&mut app, 140, 40);

        // Open New Component modal with 'n'
        app.handle_key(key(KeyCode::Char('n')));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());

        // Type ID, tab to Name, type Name, tab to Archetype, cycle right to Interface, submit
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
        assert_eq!(selected.kind, ComponentKind::Interface);
        assert_eq!(selected.parent_id, None);
        assert_eq!(selected.position, GridPos { col: 2, row: 0 });
        assert!(selected.features.is_empty());

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
        app.handle_key(key(KeyCode::Tab)); // archetype
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
        app.handle_key(key(KeyCode::Tab)); // archetype
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

    #[test]
    fn app_delete_component_and_connection_cleanup_and_cancel() {
        use crate::audit::FindingKind;
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Delete Test", "Delete component and connection")
            .expect("init workspace");

        let mut alpha = make_component("alpha");
        alpha.position = GridPos { col: 0, row: 0 };
        let mut beta = make_component("beta");
        beta.position = GridPos { col: 1, row: 0 };
        let mut gamma = make_component("gamma");
        gamma.position = GridPos { col: 2, row: 0 };
        save_component(dir.path(), &alpha).expect("save alpha");
        save_component(dir.path(), &beta).expect("save beta");
        save_component(dir.path(), &gamma).expect("save gamma");

        let mut project = load_project(dir.path()).expect("load project");
        project.config.connections = vec![
            Connection {
                from: "alpha".to_string(),
                to: "beta".to_string(),
                kind: EdgeKind::DependsOn,
                label: None,
            },
            Connection {
                from: "gamma".to_string(),
                to: "beta".to_string(),
                kind: EdgeKind::DataFlow,
                label: Some("feed".to_string()),
            },
            Connection {
                from: "beta".to_string(),
                to: "alpha".to_string(),
                kind: EdgeKind::Invokes,
                label: None,
            },
        ];
        save_project_config(dir.path(), &project.config).expect("save config");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Selected is alpha (index 0). 'D' prefers outgoing alpha -> beta; cancel with Esc.
        app.handle_key(key(KeyCode::Char('D')));
        assert_eq!(app.mode, InputMode::Modal);
        assert_eq!(
            app.modal,
            Some(ModalState::confirm_delete_connection("alpha", "beta"))
        );
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(app.project.config.connections.len(), 3);

        // Confirm deleting outgoing alpha -> beta with 'y'
        app.handle_key(key(KeyCode::Char('D')));
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(app.project.config.connections.len(), 2);
        assert!(!app
            .project
            .config
            .connections
            .iter()
            .any(|c| c.from == "alpha" && c.to == "beta"));

        // Now alpha has no outgoing connection; 'D' falls back to incoming beta -> alpha
        app.handle_key(key(KeyCode::Char('D')));
        assert_eq!(
            app.modal,
            Some(ModalState::confirm_delete_connection("beta", "alpha"))
        );
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Navigate);
        assert_eq!(app.project.config.connections.len(), 1);
        assert_eq!(app.project.config.connections[0].from, "gamma");
        assert_eq!(app.project.config.connections[0].to, "beta");

        // Alpha now has no connections -> 'D' reports status and stays in Navigate
        app.handle_key(key(KeyCode::Char('D')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert_eq!(
            app.status_message,
            "No connection on selected component to delete"
        );

        // Select beta (index 1), press 'd' -> shows 1 affected connection (gamma -> beta), cancel with 'n'
        app.selected_index = 1;
        app.detail_scroll = 4;
        app.pending_editor_component = Some("beta".to_string());
        app.handle_key(key(KeyCode::Char('d')));
        assert_eq!(app.mode, InputMode::Modal);
        assert_eq!(
            app.modal,
            Some(ModalState::confirm_delete_component("beta", "beta", 1))
        );
        app.handle_key(key(KeyCode::Char('n')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.project.component("beta").is_some());

        // Delete beta with 'd' + 'y' -> removes component and incident edge gamma -> beta cleanly
        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());
        assert!(app.project.component("beta").is_none());
        assert!(app.project.config.connections.is_empty());
        assert!(app.pending_editor_component.is_none());
        assert_eq!(app.detail_scroll, 0);
        assert_eq!(app.status_message, "Deleted component beta");
        assert!(!app
            .audit_findings
            .iter()
            .any(|f| f.kind == FindingKind::DanglingConnection));

        let reloaded = load_project(dir.path()).expect("reload project");
        assert!(reloaded.component("beta").is_none());
        assert!(reloaded.config.connections.is_empty());
        assert!(!audit_project(&reloaded)
            .iter()
            .any(|f| f.kind == FindingKind::DanglingConnection));
    }

    #[test]
    fn app_delete_rolls_back_on_disk_failure_and_keeps_modal_open() {
        use crate::storage::{init_workspace, load_project, save_component, PROJECT_FILE};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(
            dir.path(),
            "Delete Rollback",
            "Transactional delete failure",
        )
        .expect("init workspace");

        let alpha = make_component("alpha");
        let beta = make_component("beta");
        save_component(dir.path(), &alpha).expect("save alpha");
        save_component(dir.path(), &beta).expect("save beta");

        let mut project = load_project(dir.path()).expect("load project");
        project.config.connections.push(Connection {
            from: "alpha".to_string(),
            to: "beta".to_string(),
            kind: EdgeKind::DependsOn,
            label: None,
        });
        save_project_config(dir.path(), &project.config).expect("save config");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // 1. Delete alpha when its file on disk is already missing -> delete_component fails,
        //    connections are restored on disk and in memory, modal stays open with form.error.
        let alpha_toml = dir
            .path()
            .join(ROCKET_DIR)
            .join(COMPONENTS_DIR)
            .join("alpha.toml");
        std::fs::remove_file(&alpha_toml).expect("remove alpha.toml");

        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.project.component("alpha").is_some());
        assert_eq!(app.project.config.connections.len(), 1);
        if let Some(ModalState::ConfirmDelete(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected ConfirmDelete modal to remain open on delete_component error");
        }
        let reloaded = load_project(dir.path()).expect("reload after restore");
        assert_eq!(reloaded.config.connections.len(), 1);

        // Close modal
        app.handle_key(key(KeyCode::Esc));

        // 2. Break project.toml by replacing it with a directory -> DeleteConnection fails and rolls back
        let project_file = dir.path().join(ROCKET_DIR).join(PROJECT_FILE);
        std::fs::remove_file(&project_file).expect("remove project.toml");
        std::fs::create_dir(&project_file).expect("replace project.toml with dir");

        app.handle_key(key(KeyCode::Char('D')));
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.mode, InputMode::Modal);
        assert_eq!(app.project.config.connections.len(), 1);
        assert!(app.status_message.contains("Failed to save connections"));
        if let Some(ModalState::ConfirmDelete(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected ConfirmDelete modal to remain open on save failure");
        }

        // 3. DeleteComponent also fails cleanly when saving updated connections fails
        app.handle_key(key(KeyCode::Esc));
        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.project.component("alpha").is_some());
        assert_eq!(app.project.config.connections.len(), 1);
        if let Some(ModalState::ConfirmDelete(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected ConfirmDelete modal to remain open on config save failure");
        }
    }

    #[test]
    fn app_noop_delete_in_connect_mode_preserves_connect_source() {
        // 1. In InputMode::Connect with connect_source_id = Some("alpha") and no connections on
        //    "alpha", pressing 'D' keeps Connect mode and connect_source_id untouched.
        let mut app = make_app(vec![make_component("alpha"), make_component("beta")]);
        app.mode = InputMode::Connect;
        app.connect_source_id = Some("alpha".to_string());

        app.handle_key(key(KeyCode::Char('D')));
        assert_eq!(app.mode, InputMode::Connect);
        assert_eq!(app.connect_source_id.as_deref(), Some("alpha"));
        assert!(app.modal.is_none());
        assert_eq!(
            app.status_message,
            "No connection on selected component to delete"
        );

        // 2. With an empty component list in InputMode::Connect and connect_source_id = Some("alpha"),
        //    pressing 'd' keeps Connect mode and connect_source_id untouched.
        let mut empty_app = make_app(vec![]);
        empty_app.mode = InputMode::Connect;
        empty_app.connect_source_id = Some("alpha".to_string());

        empty_app.handle_key(key(KeyCode::Char('d')));
        assert_eq!(empty_app.mode, InputMode::Connect);
        assert_eq!(empty_app.connect_source_id.as_deref(), Some("alpha"));
        assert!(empty_app.modal.is_none());
        assert_eq!(empty_app.status_message, "No component selected to delete");

        // 3. Pressing 'd' when a component IS selected clears connect_source_id and enters Modal.
        app.handle_key(key(KeyCode::Char('d')));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.connect_source_id.is_none());
        assert!(matches!(app.modal, Some(ModalState::ConfirmDelete(_))));
    }

    #[test]
    fn sub_component_expand_collapse_hides_and_shows_children() {
        use crate::storage::init_workspace;

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Sub Expand Test", "Sub-component expand test")
            .expect("init workspace");

        let mut parent = make_component("parent_svc");
        parent.position = GridPos { col: 0, row: 0 };

        let mut child = make_component("child_mod");
        child.parent_id = Some("parent_svc".to_string());
        child.position = GridPos { col: 1, row: 0 };

        let mut sibling = make_component("sibling_svc");
        sibling.position = GridPos { col: 2, row: 0 };

        let mut orphan = make_component("orphan_mod");
        orphan.parent_id = Some("missing_parent".to_string());
        orphan.position = GridPos { col: 3, row: 0 };

        let mut app = make_app(vec![parent, child, sibling, orphan]);
        app.root = dir.path().to_path_buf();

        assert!(app.has_sub_components("parent_svc"));
        assert!(!app.has_sub_components("sibling_svc"));
        assert!(!app.is_expanded("parent_svc"));

        // Initially collapsed: child_mod is hidden, dangling orphan_mod is treated as top-level
        let visible_ids = |a: &App| -> Vec<String> {
            a.visible_component_indices()
                .into_iter()
                .map(|i| a.project.components[i].id.clone())
                .collect()
        };
        assert_eq!(
            visible_ids(&app),
            vec!["parent_svc", "sibling_svc", "orphan_mod"]
        );

        // Expand parent_svc via Space -> child_mod appears immediately after parent_svc
        app.handle_key(key(KeyCode::Char(' ')));
        assert!(app.is_expanded("parent_svc"));
        assert!(app.status_message.contains("Expanded"));
        assert_eq!(
            visible_ids(&app),
            vec!["parent_svc", "child_mod", "sibling_svc", "orphan_mod"]
        );

        // Collapse parent_svc via Enter -> child_mod hidden again
        app.handle_key(key(KeyCode::Enter));
        assert!(!app.is_expanded("parent_svc"));
        assert!(app.status_message.contains("Collapsed"));
        assert_eq!(
            visible_ids(&app),
            vec!["parent_svc", "sibling_svc", "orphan_mod"]
        );

        // Create a new sub-component via 'a' -> auto-expands parent and places near parent
        app.handle_key(key(KeyCode::Char('a')));
        assert_eq!(app.mode, InputMode::Modal);
        type_keys(&mut app, "child_two");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Child Two");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.is_expanded("parent_svc"));
        let child_two = app
            .project
            .component("child_two")
            .expect("child_two created");
        assert_eq!(child_two.parent_id.as_deref(), Some("parent_svc"));
        assert_eq!(
            visible_ids(&app),
            vec![
                "parent_svc",
                "child_mod",
                "child_two",
                "sibling_svc",
                "orphan_mod"
            ]
        );
    }

    #[test]
    fn expand_shifts_colliding_card_without_overwriting() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Collision Test", "Expand collision shift")
            .expect("init workspace");

        let mut parent = make_component("parent_svc");
        parent.position = GridPos { col: 0, row: 0 };

        let mut child = make_component("child_mod");
        child.parent_id = Some("parent_svc".to_string());
        child.position = GridPos { col: 1, row: 0 };

        let mut blocker = make_component("blocker_svc");
        blocker.position = GridPos { col: 1, row: 0 };

        save_component(dir.path(), &parent).expect("save parent");
        save_component(dir.path(), &child).expect("save child");
        save_component(dir.path(), &blocker).expect("save blocker");

        let mut project = load_project(dir.path()).expect("load project");
        // Tight 2x1 grid forces grid expansion when shifting blocker right
        project.config.grid = GridConfig { cols: 2, rows: 1 };
        save_project_config(dir.path(), &project.config).expect("save tight grid");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Select parent_svc (index 0 in visible list: [blocker_svc, parent_svc] sorted by id)
        let parent_vis_idx = app
            .visible_component_indices()
            .into_iter()
            .position(|i| app.project.components[i].id == "parent_svc")
            .expect("parent visible");
        app.selected_index = parent_vis_idx;

        app.handle_key(key(KeyCode::Char(' ')));
        assert!(app.is_expanded("parent_svc"));

        let child_pos = app.project.component("child_mod").unwrap().position;
        let blocker_pos = app.project.component("blocker_svc").unwrap().position;
        let parent_pos = app.project.component("parent_svc").unwrap().position;

        assert_eq!(child_pos, GridPos { col: 1, row: 0 });
        assert_ne!(blocker_pos, child_pos);
        assert_ne!(blocker_pos, parent_pos);
        assert_eq!(blocker_pos, GridPos { col: 2, row: 0 });
        assert!(app.project.config.grid.cols >= 3);

        // Persisted shifted card and expanded grid on disk
        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(
            reloaded.component("blocker_svc").unwrap().position,
            GridPos { col: 2, row: 0 }
        );
        assert!(reloaded.config.grid.cols >= 3);
    }

    #[test]
    fn edit_component_changes_archetype_without_clobbering_and_adds_sub_component() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Edit Sub Test", "Archetype and sub-component")
            .expect("init workspace");

        let mut comp = make_component("core_engine");
        comp.kind = ComponentKind::Concept;
        comp.description = "Existing description".to_string();
        comp.markdown_body = "# Core Engine\nCustom architecture notes.\n".to_string();
        comp.position = GridPos { col: 0, row: 0 };
        comp.features.push(FeatureItem {
            id: "core_engine_f1".to_string(),
            title: "Existing feature".to_string(),
            pushed: false,
            external_id: None,
        });
        save_component(dir.path(), &comp).expect("save core_engine");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Open Edit Component modal
        app.handle_key(key(KeyCode::Char('e')));
        assert_eq!(app.mode, InputMode::Modal);

        // Tab to Archetype (field 1), move right twice: Concept -> Interface -> Logic
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Right));

        // Tab through Description (2), Path (3), Tags (4), Add Feature (5) to Add Sub-Component (6)
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Token Parser");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.status_message.contains("Token Parser"));

        let updated = app
            .project
            .component("core_engine")
            .expect("core_engine exists");
        assert_eq!(updated.kind, ComponentKind::Logic);
        assert_eq!(updated.description, "Existing description");
        assert_eq!(updated.features.len(), 1);
        assert_eq!(updated.features[0].title, "Existing feature");
        assert_eq!(
            updated.markdown_body,
            "# Core Engine\nCustom architecture notes.\n"
        );

        let sub = app
            .project
            .component("token_parser")
            .expect("sub-component created with snake_case id");
        assert_eq!(sub.name, "Token Parser");
        assert_eq!(sub.kind, ComponentKind::Concept);
        assert_eq!(sub.parent_id.as_deref(), Some("core_engine"));
        assert_eq!(sub.position, GridPos { col: 1, row: 0 });

        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(
            reloaded.component("core_engine").unwrap().kind,
            ComponentKind::Logic
        );
        assert_eq!(
            reloaded
                .component("token_parser")
                .unwrap()
                .parent_id
                .as_deref(),
            Some("core_engine")
        );
    }

    #[test]
    fn edit_connection_contains_ingests_and_detaches_sub_component() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Contains Test", "EdgeKind::Contains ingestion")
            .expect("init workspace");

        let mut alpha = make_component("alpha");
        alpha.position = GridPos { col: 0, row: 0 };
        let mut beta = make_component("beta");
        beta.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &alpha).expect("save alpha");
        save_component(dir.path(), &beta).expect("save beta");

        let mut project = load_project(dir.path()).expect("load project");
        project.config.connections.push(Connection {
            from: "alpha".to_string(),
            to: "beta".to_string(),
            kind: EdgeKind::DependsOn,
            label: None,
        });
        project.config.connections.push(Connection {
            from: "beta".to_string(),
            to: "alpha".to_string(),
            kind: EdgeKind::DependsOn,
            label: None,
        });
        save_project_config(dir.path(), &project.config).expect("save config");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Edit alpha -> beta to EdgeKind::Contains (index 0 -> 4 via 4 Right presses)
        app.handle_key(key(KeyCode::Char('E')));
        assert_eq!(app.mode, InputMode::Modal);
        for _ in 0..4 {
            app.handle_key(key(KeyCode::Right));
        }
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert_eq!(app.project.config.connections[0].kind, EdgeKind::Contains);
        assert_eq!(
            app.project.component("beta").unwrap().parent_id.as_deref(),
            Some("alpha")
        );

        let reloaded = load_project(dir.path()).expect("reload project");
        assert_eq!(
            reloaded.component("beta").unwrap().parent_id.as_deref(),
            Some("alpha")
        );

        // Expand alpha, select beta, and try to set beta -> alpha to Contains -> rejected as cycle
        app.handle_key(key(KeyCode::Char(' ')));
        assert!(app.is_expanded("alpha"));
        app.selected_index = 1; // beta
        assert_eq!(app.selected_component().unwrap().id, "beta");

        app.handle_key(key(KeyCode::Char('E')));
        for _ in 0..4 {
            app.handle_key(key(KeyCode::Right));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.status_message.contains("descendant"));
        assert_eq!(app.project.component("alpha").unwrap().parent_id, None);
        app.handle_key(key(KeyCode::Esc));

        // Select alpha again and change alpha -> beta from Contains to Custom -> detaches beta
        app.selected_index = 0;
        app.handle_key(key(KeyCode::Char('E')));
        app.handle_key(key(KeyCode::Right)); // Contains -> Custom
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        assert_eq!(app.project.config.connections[0].kind, EdgeKind::Custom);
        assert_eq!(app.project.component("beta").unwrap().parent_id, None);

        let reloaded_after = load_project(dir.path()).expect("reload after detach");
        assert_eq!(reloaded_after.component("beta").unwrap().parent_id, None);
    }

    #[test]
    fn delete_parent_detaches_sub_components() {
        use crate::storage::{init_workspace, load_project, save_component};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Delete Parent", "Detach children on delete")
            .expect("init workspace");

        let mut parent = make_component("parent_box");
        parent.position = GridPos { col: 0, row: 0 };
        let mut child = make_component("child_box");
        child.parent_id = Some("parent_box".to_string());
        child.position = GridPos { col: 1, row: 0 };
        save_component(dir.path(), &parent).expect("save parent");
        save_component(dir.path(), &child).expect("save child");

        let project = load_project(dir.path()).expect("load project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        // Expand parent_box then delete it
        app.handle_key(key(KeyCode::Char(' ')));
        assert!(app.is_expanded("parent_box"));

        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Char('y')));

        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.project.component("parent_box").is_none());
        assert!(!app.is_expanded("parent_box"));
        let detached = app
            .project
            .component("child_box")
            .expect("child_box still exists");
        assert_eq!(detached.parent_id, None);
        assert_eq!(app.visible_component_indices().len(), 1);

        let reloaded = load_project(dir.path()).expect("reload project");
        assert!(reloaded.component("parent_box").is_none());
        assert_eq!(reloaded.component("child_box").unwrap().parent_id, None);
    }

    #[test]
    fn connect_prompt_states_dependency_direction() {
        use crate::storage::init_workspace;

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Connect Prompt", "Connect prompt test")
            .expect("init workspace");

        let mut alpha = make_component("alpha");
        alpha.name = "Alpha Service".to_string();
        alpha.position = GridPos { col: 0, row: 0 };
        let mut beta = make_component("beta");
        beta.name = "Beta Store".to_string();
        beta.position = GridPos { col: 1, row: 0 };

        let mut app = make_app(vec![alpha, beta]);
        app.root = dir.path().to_path_buf();
        app.focus = FocusColumn::Canvas;

        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.mode, InputMode::Connect);
        assert_eq!(
            app.status_message,
            "Connect: select dependency for Alpha Service (Alpha Service depends on <target>), Enter to confirm"
        );

        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.mode, InputMode::Navigate);
        assert_eq!(app.status_message, "Connected: alpha depends on beta");
    }

    #[test]
    fn zoom_keys_cycle_between_compact_normal_and_expanded() {
        use crate::tui::canvas::ZoomLevel;

        let mut app = make_app(vec![make_component("alpha")]);
        assert_eq!(app.viewport.zoom, ZoomLevel::Compact);

        app.handle_key(key(KeyCode::Char('+')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Normal);
        assert_eq!(app.status_message, "Zoom: Normal");

        app.handle_key(key(KeyCode::Char('=')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Expanded);
        assert_eq!(app.status_message, "Zoom: Expanded");

        // Clamps at Expanded
        app.handle_key(key(KeyCode::Char('+')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Expanded);
        assert_eq!(app.status_message, "Zoom: Expanded");

        app.handle_key(key(KeyCode::Char('-')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Normal);
        assert_eq!(app.status_message, "Zoom: Normal");

        app.handle_key(key(KeyCode::Char('-')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Compact);
        assert_eq!(app.status_message, "Zoom: Compact");

        // Clamps at Compact
        app.handle_key(key(KeyCode::Char('-')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Compact);
        assert_eq!(app.status_message, "Zoom: Compact");
    }

    #[test]
    fn first_unoccupied_grid_pos_starts_top_center() {
        use crate::storage::{init_workspace, load_project, save_component, save_project_config};

        let empty_app = make_app(Vec::new());
        let first_pos = empty_app
            .first_unoccupied_grid_pos()
            .expect("empty grid has free cell");
        assert_eq!(first_pos.row, 0);
        assert_eq!(first_pos.col, 3);
        assert_ne!(first_pos, GridPos { col: 0, row: 0 });

        // Full 1x1 grid returns None when effective capacity does not expand it.
        let mut full_app = make_app(vec![make_component("only")]);
        full_app.project.config.grid = GridConfig { cols: 1, rows: 1 };
        assert_eq!(full_app.first_unoccupied_grid_pos(), None);

        // Creating a top-level component in a floored cell outside saved grid persists grid expansion.
        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Grid Floor", "Persist expanded grid").expect("init workspace");
        let mut cfg = load_project(dir.path()).expect("load project").config;
        cfg.grid = GridConfig { cols: 1, rows: 1 };
        save_project_config(dir.path(), &cfg).expect("save 1x1 grid");
        let mut existing = make_component("alpha");
        existing.position = GridPos { col: 0, row: 0 };
        save_component(dir.path(), &existing).expect("save alpha");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        render_app(&mut app, 140, 40);

        app.handle_key(key(KeyCode::Char('n')));
        type_keys(&mut app, "beta");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Beta");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Navigate);
        let beta = app.project.component("beta").expect("beta created");
        assert_eq!(beta.position, GridPos { col: 2, row: 0 });
        assert_eq!(app.project.config.grid, GridConfig { cols: 3, rows: 1 });

        let reloaded = load_project(dir.path()).expect("reload after creation");
        assert_eq!(reloaded.config.grid, GridConfig { cols: 3, rows: 1 });
    }

    fn render_app(app: &mut App, width: u16, height: u16) {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).expect("create test terminal");
        terminal
            .draw(|frame| crate::tui::ui::render(frame, app))
            .expect("render frame");
    }

    fn set_dir_readonly(path: &std::path::Path, readonly: bool) {
        let mut perms = std::fs::metadata(path)
            .expect("read metadata")
            .permissions();
        perms.set_readonly(readonly);
        std::fs::set_permissions(path, perms).expect("set permissions");
    }

    #[test]
    fn create_top_level_rolls_back_component_when_config_save_fails() {
        use crate::storage::{init_workspace, load_project, save_component, save_project_config};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Rollback Create", "Rollback top-level create")
            .expect("init workspace");
        let mut cfg = load_project(dir.path()).expect("load project").config;
        cfg.grid = GridConfig { cols: 1, rows: 1 };
        save_project_config(dir.path(), &cfg).expect("save 1x1 grid");
        let mut existing = make_component("alpha");
        existing.position = GridPos { col: 0, row: 0 };
        save_component(dir.path(), &existing).expect("save alpha");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        render_app(&mut app, 140, 40);

        let rocket_dir = dir.path().join(ROCKET_DIR);
        let project_toml = rocket_dir.join(crate::storage::PROJECT_FILE);
        set_dir_readonly(&project_toml, true);
        set_dir_readonly(&rocket_dir, true);

        app.handle_key(key(KeyCode::Char('n')));
        type_keys(&mut app, "beta");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Beta");
        app.handle_key(key(KeyCode::Enter));

        set_dir_readonly(&rocket_dir, false);
        set_dir_readonly(&project_toml, false);

        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        if let Some(ModalState::NewComponent(ref form)) = app.modal {
            assert!(form
                .error
                .as_deref()
                .unwrap_or("")
                .contains("Failed to save grid config"));
        } else {
            panic!("expected NewComponent modal to remain open");
        }
        assert!(app.project.component("beta").is_none());
        assert_eq!(app.project.config.grid, GridConfig { cols: 1, rows: 1 });

        let reloaded = load_project(dir.path()).expect("reload after rollback");
        assert!(reloaded.component("beta").is_none());
        assert_eq!(reloaded.config.grid, GridConfig { cols: 1, rows: 1 });
    }

    #[test]
    fn create_sub_component_rolls_back_component_when_config_save_fails() {
        use crate::storage::{init_workspace, load_project, save_component, save_project_config};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Rollback Sub", "Rollback sub-component create")
            .expect("init workspace");
        let mut cfg = load_project(dir.path()).expect("load project").config;
        cfg.grid = GridConfig { cols: 1, rows: 1 };
        save_project_config(dir.path(), &cfg).expect("save 1x1 grid");
        let mut existing = make_component("alpha");
        existing.position = GridPos { col: 0, row: 0 };
        save_component(dir.path(), &existing).expect("save alpha");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);

        let rocket_dir = dir.path().join(ROCKET_DIR);
        let project_toml = rocket_dir.join(crate::storage::PROJECT_FILE);
        set_dir_readonly(&project_toml, true);
        set_dir_readonly(&rocket_dir, true);

        app.handle_key(key(KeyCode::Char('a')));
        type_keys(&mut app, "alpha_sub");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Alpha Sub");
        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.modal.is_some());
        if let Some(ModalState::NewComponent(ref form)) = app.modal {
            assert!(form
                .error
                .as_deref()
                .unwrap_or("")
                .contains("Failed to save grid config"));
        } else {
            panic!("expected NewComponent modal to remain open");
        }
        assert!(app.project.component("alpha_sub").is_none());

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, InputMode::Navigate);
        assert!(app.modal.is_none());

        set_dir_readonly(&rocket_dir, false);
        set_dir_readonly(&project_toml, false);

        let reloaded = load_project(dir.path()).expect("reload after rollback");
        assert!(reloaded.component("alpha_sub").is_none());
        assert_eq!(reloaded.config.grid, GridConfig { cols: 1, rows: 1 });
    }

    #[test]
    fn move_card_rolls_back_component_when_config_save_fails() {
        use crate::storage::{init_workspace, load_project, save_component, save_project_config};

        let dir = tempfile::tempdir().expect("tempdir");
        init_workspace(dir.path(), "Rollback Move", "Rollback card move").expect("init workspace");
        let mut cfg = load_project(dir.path()).expect("load project").config;
        cfg.grid = GridConfig { cols: 1, rows: 1 };
        save_project_config(dir.path(), &cfg).expect("save 1x1 grid");
        let mut existing = make_component("alpha");
        existing.position = GridPos { col: 0, row: 0 };
        save_component(dir.path(), &existing).expect("save alpha");

        let project = load_project(dir.path()).expect("reload project");
        let theme = default_theme().expect("default theme");
        let mut app = App::new(dir.path().to_path_buf(), project, theme);
        render_app(&mut app, 140, 40);

        let rocket_dir = dir.path().join(ROCKET_DIR);
        let project_toml = rocket_dir.join(crate::storage::PROJECT_FILE);
        set_dir_readonly(&project_toml, true);
        set_dir_readonly(&rocket_dir, true);

        app.handle_key(key(KeyCode::Char('m')));
        app.handle_key(key(KeyCode::Right));

        set_dir_readonly(&rocket_dir, false);
        set_dir_readonly(&project_toml, false);

        assert!(app.status_message.contains("Failed to save grid config"));
        assert_eq!(
            app.project
                .component("alpha")
                .expect("alpha exists")
                .position,
            GridPos { col: 0, row: 0 }
        );
        assert_eq!(app.project.config.grid, GridConfig { cols: 1, rows: 1 });

        let reloaded = load_project(dir.path()).expect("reload after rollback");
        assert_eq!(
            reloaded.component("alpha").expect("alpha on disk").position,
            GridPos { col: 0, row: 0 }
        );
        assert_eq!(reloaded.config.grid, GridConfig { cols: 1, rows: 1 });
    }

    #[test]
    fn move_out_of_grid_card_forward_does_not_jump_backwards() {
        let mut far_right = make_component("alpha");
        far_right.position = GridPos { col: 20, row: 0 };
        let mut app = make_app(vec![far_right]);
        app.project.config.grid = GridConfig { cols: 8, rows: 8 };

        app.handle_key(key(KeyCode::Char('m')));
        app.handle_key(key(KeyCode::Right));
        assert_eq!(
            app.project
                .component("alpha")
                .expect("alpha exists")
                .position,
            GridPos { col: 20, row: 0 }
        );

        // Also verify Down on an out-of-grid row does not jump backwards.
        app.project.components[0].position = GridPos { col: 0, row: 20 };
        app.handle_key(key(KeyCode::Down));
        assert_eq!(
            app.project
                .component("alpha")
                .expect("alpha exists")
                .position,
            GridPos { col: 0, row: 20 }
        );
    }

    #[test]
    fn mutation_errors_when_rocket_dir_missing() {
        let dir = tempfile::tempdir().expect("tempdir without .rocket");
        let mut alpha = make_component("alpha");
        alpha.position = GridPos { col: 0, row: 0 };
        alpha.description = "Initial alpha".to_string();

        let mut app = make_app(vec![alpha]);
        app.root = dir.path().to_path_buf();
        let initial_project = app.project.clone();

        // 1. Create component fails and leaves project unchanged
        app.handle_key(key(KeyCode::Char('n')));
        type_keys(&mut app, "beta");
        app.handle_key(key(KeyCode::Tab));
        type_keys(&mut app, "Beta");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.project, initial_project);
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.status_message.contains("Failed to save"));
        if let Some(ModalState::NewComponent(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected NewComponent modal to remain open");
        }
        app.handle_key(key(KeyCode::Esc));

        // 2. Edit component fails and leaves project unchanged
        app.handle_key(key(KeyCode::Char('e')));
        app.handle_key(key(KeyCode::Tab)); // archetype
        app.handle_key(key(KeyCode::Tab)); // description
        type_keys(&mut app, " edited");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.project, initial_project);
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.status_message.contains("Failed to save"));
        if let Some(ModalState::EditComponent(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected EditComponent modal to remain open");
        }
        app.handle_key(key(KeyCode::Esc));

        // 3. Move card fails and leaves project unchanged
        app.handle_key(key(KeyCode::Char('m')));
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.project, initial_project);
        assert!(app.status_message.contains("Failed to save"));
        app.handle_key(key(KeyCode::Char('m')));

        // 4. Delete component fails and leaves project unchanged
        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.project, initial_project);
        assert_eq!(app.mode, InputMode::Modal);
        assert!(app.status_message.contains("Failed to delete"));
        if let Some(ModalState::ConfirmDelete(ref form)) = app.modal {
            assert!(form.error.is_some());
        } else {
            panic!("expected ConfirmDelete modal to remain open");
        }
    }
}
