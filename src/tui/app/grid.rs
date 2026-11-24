use std::collections::HashSet;

use crate::model::{Component, GridConfig, GridPos};
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

    pub(super) fn find_proximity_grid_pos(&self, anchor: GridPos) -> Option<GridPos> {
        let occupied: HashSet<GridPos> = self
            .visible_component_indices()
            .into_iter()
            .filter_map(|idx| self.project.components.get(idx).map(|c| c.position))
            .collect();

        find_proximity_in_occupied(anchor, self.project.config.grid, &occupied)
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

        if colliding_indices.is_empty() {
            return Vec::new();
        }

        let mut occupied: HashSet<GridPos> = self
            .visible_component_indices()
            .into_iter()
            .filter_map(|idx| self.project.components.get(idx).map(|c| c.position))
            .collect();

        let mut candidate_config = self.project.config.clone();
        let mut grid_expanded_any = false;
        let mut planned: Vec<(usize, Component, Component)> = Vec::new();
        let mut errors = Vec::new();

        for idx in colliding_indices {
            let Some(previous) = self.project.components.get(idx).cloned() else {
                continue;
            };
            let Some(new_pos) =
                find_proximity_in_occupied(protected_pos, candidate_config.grid, &occupied)
            else {
                errors.push(format!(
                    "no free grid cell available to relocate {}",
                    previous.id
                ));
                continue;
            };
            occupied.insert(new_pos);
            if ensure_grid_contains(&mut candidate_config.grid, new_pos) {
                grid_expanded_any = true;
            }
            let mut candidate = previous.clone();
            candidate.position = new_pos;
            planned.push((idx, previous, candidate));
        }

        let mut written: Vec<usize> = Vec::new();
        let mut failed = false;

        for (plan_pos, (_, _, candidate)) in planned.iter().enumerate() {
            if let Err(err) = save_component(&self.root, candidate) {
                errors.push(format!("failed to save {}: {err}", candidate.id));
                failed = true;
                break;
            }
            written.push(plan_pos);
        }

        let mut config_written = false;
        if !failed && grid_expanded_any {
            if let Err(err) = save_project_config(&self.root, &candidate_config) {
                errors.push(format!("failed to save grid config: {err}"));
                failed = true;
            } else {
                config_written = true;
            }
        }

        if failed {
            if config_written {
                if let Err(rb_err) = save_project_config(&self.root, &self.project.config) {
                    errors.push(format!("rollback failed for grid config: {rb_err}"));
                }
            }
            for plan_pos in written.into_iter().rev() {
                let (_, ref previous, _) = planned[plan_pos];
                if let Err(rb_err) = save_component(&self.root, previous) {
                    errors.push(format!("rollback failed for {}: {rb_err}", previous.id));
                }
            }
            return errors;
        }

        for (comp_idx, _, candidate) in planned {
            self.project.components[comp_idx] = candidate;
        }
        if grid_expanded_any {
            self.project.config = candidate_config;
        }

        errors
    }
}

fn find_proximity_in_occupied(
    anchor: GridPos,
    grid: GridConfig,
    occupied: &HashSet<GridPos>,
) -> Option<GridPos> {
    let cols = grid.cols.max(1);
    let rows = grid.rows.max(1);
    let max_dist = cols.saturating_add(rows);

    let is_free_in_bounds =
        |pos: GridPos| -> bool { pos.col < cols && pos.row < rows && !occupied.contains(&pos) };

    let total_in_bounds = u64::from(cols) * u64::from(rows);
    let occupied_in_bounds = occupied
        .iter()
        .filter(|pos| pos.col < cols && pos.row < rows)
        .count() as u64;

    if occupied_in_bounds < total_in_bounds {
        for dist in 1..=max_dist {
            // 1. Right / below anchor first: (+dist, 0), ..., (0, +dist)
            if anchor.row < rows {
                let max_dr = dist.min((rows - 1) - anchor.row);
                for dr in 0..=max_dr {
                    let dc = dist - dr;
                    if let (Some(col), Some(row)) =
                        (anchor.col.checked_add(dc), anchor.row.checked_add(dr))
                    {
                        let candidate = GridPos { col, row };
                        if is_free_in_bounds(candidate) {
                            return Some(candidate);
                        }
                    }
                }
            }

            // 2. Right / above anchor: dc > 0, dr < 0
            if dist > 1 && anchor.row > 0 {
                let max_dr = (dist - 1).min(anchor.row);
                for dr in 1..=max_dr {
                    let dc = dist - dr;
                    if let (Some(col), Some(row)) =
                        (anchor.col.checked_add(dc), anchor.row.checked_sub(dr))
                    {
                        let candidate = GridPos { col, row };
                        if is_free_in_bounds(candidate) {
                            return Some(candidate);
                        }
                    }
                }
            }

            // 3. Left / below anchor: dc < 0, dr > 0
            if dist > 1 && anchor.row < rows {
                let max_dr = (dist - 1).min((rows - 1) - anchor.row);
                if max_dr >= 1 {
                    let min_dc = dist - max_dr;
                    for dc in min_dc..dist {
                        let dr = dist - dc;
                        if let (Some(col), Some(row)) =
                            (anchor.col.checked_sub(dc), anchor.row.checked_add(dr))
                        {
                            let candidate = GridPos { col, row };
                            if is_free_in_bounds(candidate) {
                                return Some(candidate);
                            }
                        }
                    }
                }
            }

            // 4. Left / above anchor: dc <= 0, dr <= 0
            let max_dr = dist.min(anchor.row);
            for dr in 0..=max_dr {
                let dc = dist - dr;
                if let (Some(col), Some(row)) =
                    (anchor.col.checked_sub(dc), anchor.row.checked_sub(dr))
                {
                    let candidate = GridPos { col, row };
                    if is_free_in_bounds(candidate) {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    // Grid is full within current bounds: expand to the right of the grid at anchor.row.
    let mut extra_col = Some(cols);
    while let Some(col) = extra_col {
        let candidate = GridPos {
            col,
            row: anchor.row,
        };
        if !occupied.contains(&candidate) {
            return Some(candidate);
        }
        extra_col = col.checked_add(1);
    }

    // anchor.row is exhausted across cols..=u16::MAX; scan remaining cells across 0..=u16::MAX.
    for row in 0..=u16::MAX {
        for col in 0..=u16::MAX {
            let candidate = GridPos { col, row };
            if !occupied.contains(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}
