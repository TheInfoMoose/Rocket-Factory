use std::collections::HashSet;
use std::path::PathBuf;

use ratatui::widgets::ListState;

use crate::audit::{audit_project, compute_card_status, AuditFinding};
use crate::model::{CardStatus, Component, Project};
use crate::storage::validate_component_id;
use crate::theme::Theme;
use crate::tui::canvas::CanvasViewport;
use crate::tui::modal::ModalState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Direction {
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
    pub list_state: ListState,
    pub detail_scroll: u16,
    pub last_detail_max_scroll: Option<u16>,
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
            list_state: ListState::default(),
            detail_scroll: 0,
            last_detail_max_scroll: None,
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

    pub fn zoom_in(&mut self) {
        self.viewport.zoom = self.viewport.zoom.zoom_in();
        self.status_message = format!("Zoom: {}", self.viewport.zoom.label());
    }

    pub fn zoom_out(&mut self) {
        self.viewport.zoom = self.viewport.zoom.zoom_out();
        self.status_message = format!("Zoom: {}", self.viewport.zoom.label());
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

    fn has_top_level_root(&self, idx: usize, existing_ids: &HashSet<&str>) -> bool {
        let Some(mut current) = self.project.components.get(idx) else {
            return false;
        };
        let mut seen = HashSet::new();
        seen.insert(current.id.as_str());
        while let Some(pid) = current.parent_id.as_deref() {
            if !existing_ids.contains(pid) {
                return true;
            }
            if !seen.insert(pid) {
                return false;
            }
            match self.project.component(pid) {
                Some(parent) => current = parent,
                None => return true,
            }
        }
        true
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

        for idx in 0..self.project.components.len() {
            if !visited.contains(&idx) && !self.has_top_level_root(idx, &existing_ids) {
                self.push_visible_component(idx, &mut out, &mut visited);
            }
        }

        out
    }

    pub fn selected_component(&self) -> Option<&Component> {
        let indices = self.visible_component_indices();
        indices
            .get(self.selected_index)
            .and_then(|&idx| self.project.components.get(idx))
    }

    pub fn clamp_selection(&mut self) {
        let visible = self.visible_component_indices();
        let before_id = visible
            .get(self.selected_index)
            .and_then(|&idx| self.project.components.get(idx))
            .map(|c| c.id.as_str());
        if visible.is_empty() {
            self.selected_index = 0;
        } else {
            self.selected_index = self.selected_index.min(visible.len() - 1);
        }
        let after_id = visible
            .get(self.selected_index)
            .and_then(|&idx| self.project.components.get(idx))
            .map(|c| c.id.as_str());
        if visible.is_empty() || before_id != after_id {
            self.detail_scroll = 0;
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

    pub(super) fn is_descendant_of(&self, node_id: &str, ancestor_id: &str) -> bool {
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

    pub(super) fn derive_unique_sub_component_id(&self, parent_id: &str, sub_name: &str) -> String {
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
}
