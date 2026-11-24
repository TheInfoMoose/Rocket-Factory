use std::collections::HashSet;

use crate::model::{GridConfig, GridPos};
use crate::storage::{save_component, save_project_config};

use super::state::{App, Direction};

pub(super) fn ensure_grid_contains(grid: &mut GridConfig, pos: GridPos) -> bool {
    let mut expanded = false;
    if pos.col >= grid.cols {
        grid.cols = pos.col.saturating_add(1);
        expanded = true;
    }
    if pos.row >= grid.rows {
        grid.rows = pos.row.saturating_add(1);
        expanded = true;
    }
    expanded
}

impl App {
    pub fn effective_grid(&self) -> GridConfig {
        let (vis_cols, vis_rows) = self.last_canvas_capacity.unwrap_or((0, 0));
        GridConfig {
            cols: self.project.config.grid.cols.max(vis_cols),
            rows: self.project.config.grid.rows.max(vis_rows),
        }
    }

    pub(super) fn select_in_direction(&mut self, dir: Direction) {
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

    pub(super) fn move_selected_card(&mut self, dir: Direction) {
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
        let expanded = ensure_grid_contains(&mut candidate_config.grid, target_pos);

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

    pub(super) fn first_unoccupied_grid_pos(&self) -> Option<GridPos> {
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

    pub(super) fn find_proximity_grid_pos(&self, anchor: GridPos) -> GridPos {
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

    pub(super) fn resolve_visible_collisions_for(&mut self, protected_id: &str) -> Vec<String> {
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
            if ensure_grid_contains(&mut self.project.config.grid, new_pos) {
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
}
