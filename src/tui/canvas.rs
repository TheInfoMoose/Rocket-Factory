use std::collections::HashMap;

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use crate::archetype::archetype_for;
use crate::audit::{compute_card_status, task_progress};
use crate::model::{EdgeKind, GridPos};
use crate::tui::app::{App, FocusColumn};
use crate::tui::style;

pub const CARD_WIDTH: u16 = 22;
pub const CARD_HEIGHT: u16 = 5;
pub const GUTTER_X: u16 = 4;
pub const GUTTER_Y: u16 = 2;
pub const CELL_STRIDE_X: u16 = CARD_WIDTH + GUTTER_X;
pub const CELL_STRIDE_Y: u16 = CARD_HEIGHT + GUTTER_Y;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardMetrics {
    pub card_width: u16,
    pub card_height: u16,
    pub gutter_x: u16,
    pub gutter_y: u16,
}

impl CardMetrics {
    pub fn stride_x(self) -> u16 {
        self.card_width.saturating_add(self.gutter_x)
    }

    pub fn stride_y(self) -> u16 {
        self.card_height.saturating_add(self.gutter_y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoomLevel {
    #[default]
    Compact,
    Normal,
    Expanded,
}

impl ZoomLevel {
    pub fn zoom_in(self) -> Self {
        match self {
            Self::Compact => Self::Normal,
            Self::Normal | Self::Expanded => Self::Expanded,
        }
    }

    pub fn zoom_out(self) -> Self {
        match self {
            Self::Expanded => Self::Normal,
            Self::Normal | Self::Compact => Self::Compact,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Normal => "Normal",
            Self::Expanded => "Expanded",
        }
    }

    pub fn metrics(self) -> CardMetrics {
        match self {
            Self::Compact => CardMetrics {
                card_width: 14,
                card_height: 4,
                gutter_x: 2,
                gutter_y: 1,
            },
            Self::Normal => CardMetrics {
                card_width: CARD_WIDTH,
                card_height: CARD_HEIGHT,
                gutter_x: GUTTER_X,
                gutter_y: GUTTER_Y,
            },
            Self::Expanded => CardMetrics {
                card_width: 30,
                card_height: 7,
                gutter_x: 4,
                gutter_y: 2,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CanvasViewport {
    pub scroll_col: u16,
    pub scroll_row: u16,
    pub zoom: ZoomLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardAnchors {
    pub top: (u16, u16),
    pub bottom: (u16, u16),
    pub left: (u16, u16),
    pub right: (u16, u16),
}

pub fn card_rect(inner: Rect, viewport: CanvasViewport, pos: GridPos) -> Option<Rect> {
    if pos.col < viewport.scroll_col || pos.row < viewport.scroll_row {
        return None;
    }

    let metrics = viewport.zoom.metrics();
    let rel_col = pos.col - viewport.scroll_col;
    let rel_row = pos.row - viewport.scroll_row;

    let x = inner
        .x
        .saturating_add(1)
        .saturating_add(rel_col.saturating_mul(metrics.stride_x()));
    let y = inner
        .y
        .saturating_add(1)
        .saturating_add(rel_row.saturating_mul(metrics.stride_y()));

    let max_x = inner.x.saturating_add(inner.width);
    let max_y = inner.y.saturating_add(inner.height);

    if x.saturating_add(metrics.card_width) <= max_x
        && y.saturating_add(metrics.card_height) <= max_y
    {
        return Some(Rect {
            x,
            y,
            width: metrics.card_width,
            height: metrics.card_height,
        });
    }

    if inner.width >= 4 && inner.height >= 3 {
        let width = if rel_col == 0 {
            let clamped = metrics.card_width.min(max_x.saturating_sub(x));
            (clamped >= 3).then_some(clamped)
        } else if x.saturating_add(metrics.card_width) <= max_x {
            Some(metrics.card_width)
        } else {
            None
        };

        let height = if rel_row == 0 {
            let clamped = metrics.card_height.min(max_y.saturating_sub(y));
            (clamped >= 2).then_some(clamped)
        } else if y.saturating_add(metrics.card_height) <= max_y {
            Some(metrics.card_height)
        } else {
            None
        };

        if let (Some(width), Some(height)) = (width, height) {
            return Some(Rect {
                x,
                y,
                width,
                height,
            });
        }
    }

    None
}

pub fn clamp_viewport_scroll(
    inner: Rect,
    viewport: &mut CanvasViewport,
    occupied_cols: u16,
    occupied_rows: u16,
) {
    let (vis_cols, vis_rows) = visible_grid_capacity_for(inner, viewport.zoom);
    let max_scroll_col = occupied_cols.saturating_sub(vis_cols);
    let max_scroll_row = occupied_rows.saturating_sub(vis_rows);
    viewport.scroll_col = viewport.scroll_col.min(max_scroll_col);
    viewport.scroll_row = viewport.scroll_row.min(max_scroll_row);
}

pub fn visible_grid_capacity_for(inner: Rect, zoom: ZoomLevel) -> (u16, u16) {
    let metrics = zoom.metrics();
    let cols = (inner
        .width
        .saturating_sub(1)
        .saturating_add(metrics.gutter_x)
        / metrics.stride_x())
    .max(1);
    let rows = (inner
        .height
        .saturating_sub(1)
        .saturating_add(metrics.gutter_y)
        / metrics.stride_y())
    .max(1);
    (cols, rows)
}

pub fn visible_grid_capacity(inner: Rect) -> (u16, u16) {
    visible_grid_capacity_for(inner, ZoomLevel::Normal)
}

pub fn ensure_visible(inner: Rect, viewport: &mut CanvasViewport, pos: GridPos) {
    let (vis_cols, vis_rows) = visible_grid_capacity_for(inner, viewport.zoom);

    if pos.col < viewport.scroll_col {
        viewport.scroll_col = pos.col;
    } else if pos.col >= viewport.scroll_col.saturating_add(vis_cols) {
        viewport.scroll_col = pos.col.saturating_sub(vis_cols.saturating_sub(1));
    }

    if pos.row < viewport.scroll_row {
        viewport.scroll_row = pos.row;
    } else if pos.row >= viewport.scroll_row.saturating_add(vis_rows) {
        viewport.scroll_row = pos.row.saturating_sub(vis_rows.saturating_sub(1));
    }
}

pub fn card_anchor_points(rect: Rect) -> CardAnchors {
    let mid_x = rect.x.saturating_add(rect.width / 2);
    let mid_y = rect.y.saturating_add(rect.height / 2);

    CardAnchors {
        top: (mid_x, rect.y),
        bottom: (mid_x, rect.y.saturating_add(rect.height.saturating_sub(1))),
        left: (rect.x, mid_y),
        right: (rect.x.saturating_add(rect.width.saturating_sub(1)), mid_y),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedEdge {
    pub segments: Vec<(u16, u16, char)>,
    pub label_pos: Option<(u16, u16)>,
    pub label: Option<String>,
}

fn in_bounds(x: u16, y: u16, bounds: Rect) -> bool {
    x >= bounds.x
        && x < bounds.x.saturating_add(bounds.width)
        && y >= bounds.y
        && y < bounds.y.saturating_add(bounds.height)
}

pub fn route_orthogonal_edge(
    from_rect: Rect,
    to_rect: Rect,
    label: Option<&str>,
    bounds: Rect,
) -> RoutedEdge {
    let from_a = card_anchor_points(from_rect);
    let to_a = card_anchor_points(to_rect);

    let (start, end, arrow, horizontal_first) =
        if to_rect.x >= from_rect.x.saturating_add(from_rect.width) {
            (
                (from_a.right.0.saturating_add(1), from_a.right.1),
                (to_a.left.0.saturating_sub(1), to_a.left.1),
                '▶',
                true,
            )
        } else if from_rect.x >= to_rect.x.saturating_add(to_rect.width) {
            (
                (from_a.left.0.saturating_sub(1), from_a.left.1),
                (to_a.right.0.saturating_add(1), to_a.right.1),
                '◀',
                true,
            )
        } else if to_rect.y >= from_rect.y.saturating_add(from_rect.height) {
            (
                (from_a.bottom.0, from_a.bottom.1.saturating_add(1)),
                (to_a.top.0, to_a.top.1.saturating_sub(1)),
                '▼',
                false,
            )
        } else {
            (
                (from_a.top.0, from_a.top.1.saturating_sub(1)),
                (to_a.bottom.0, to_a.bottom.1.saturating_add(1)),
                '▲',
                false,
            )
        };

    let (sx, sy) = start;
    let (ex, ey) = end;
    let mut raw: Vec<(u16, u16, char)> = Vec::new();

    if horizontal_first {
        if sy == ey {
            if sx <= ex {
                for x in sx..=ex {
                    raw.push((x, sy, '─'));
                }
            } else {
                for x in (ex..=sx).rev() {
                    raw.push((x, sy, '─'));
                }
            }
        } else {
            let mid_x = sx.min(ex).saturating_add(sx.abs_diff(ex) / 2);
            if sx <= ex {
                for x in sx..mid_x {
                    raw.push((x, sy, '─'));
                }
                let corner1 = if sy < ey { '┐' } else { '┘' };
                raw.push((mid_x, sy, corner1));
                if sy < ey {
                    for y in (sy + 1)..ey {
                        raw.push((mid_x, y, '│'));
                    }
                } else {
                    for y in ((ey + 1)..sy).rev() {
                        raw.push((mid_x, y, '│'));
                    }
                }
                let corner2 = if sy < ey { '└' } else { '┌' };
                raw.push((mid_x, ey, corner2));
                for x in (mid_x + 1)..=ex {
                    raw.push((x, ey, '─'));
                }
            } else {
                for x in ((mid_x + 1)..=sx).rev() {
                    raw.push((x, sy, '─'));
                }
                let corner1 = if sy < ey { '┌' } else { '└' };
                raw.push((mid_x, sy, corner1));
                if sy < ey {
                    for y in (sy + 1)..ey {
                        raw.push((mid_x, y, '│'));
                    }
                } else {
                    for y in ((ey + 1)..sy).rev() {
                        raw.push((mid_x, y, '│'));
                    }
                }
                let corner2 = if sy < ey { '┘' } else { '┐' };
                raw.push((mid_x, ey, corner2));
                for x in (ex..mid_x).rev() {
                    raw.push((x, ey, '─'));
                }
            }
        }
    } else if sx == ex {
        if sy <= ey {
            for y in sy..=ey {
                raw.push((sx, y, '│'));
            }
        } else {
            for y in (ey..=sy).rev() {
                raw.push((sx, y, '│'));
            }
        }
    } else {
        let mid_y = sy.min(ey).saturating_add(sy.abs_diff(ey) / 2);
        if sy <= ey {
            for y in sy..mid_y {
                raw.push((sx, y, '│'));
            }
            let corner1 = if sx < ex { '└' } else { '┘' };
            raw.push((sx, mid_y, corner1));
            if sx < ex {
                for x in (sx + 1)..ex {
                    raw.push((x, mid_y, '─'));
                }
            } else {
                for x in ((ex + 1)..sx).rev() {
                    raw.push((x, mid_y, '─'));
                }
            }
            let corner2 = if sx < ex { '┐' } else { '┌' };
            raw.push((ex, mid_y, corner2));
            for y in (mid_y + 1)..=ey {
                raw.push((ex, y, '│'));
            }
        } else {
            for y in ((mid_y + 1)..=sy).rev() {
                raw.push((sx, y, '│'));
            }
            let corner1 = if sx < ex { '┌' } else { '┐' };
            raw.push((sx, mid_y, corner1));
            if sx < ex {
                for x in (sx + 1)..ex {
                    raw.push((x, mid_y, '─'));
                }
            } else {
                for x in ((ex + 1)..sx).rev() {
                    raw.push((x, mid_y, '─'));
                }
            }
            let corner2 = if sx < ex { '┘' } else { '└' };
            raw.push((ex, mid_y, corner2));
            for y in (ey..mid_y).rev() {
                raw.push((ex, y, '│'));
            }
        }
    }

    if let Some(last) = raw.last_mut() {
        last.2 = arrow;
    } else {
        raw.push((ex, ey, arrow));
    }

    let segments: Vec<(u16, u16, char)> = raw
        .into_iter()
        .filter(|&(x, y, _)| in_bounds(x, y, bounds))
        .collect();

    let (label_pos, label) = match label {
        Some(text) if !text.is_empty() && !segments.is_empty() => (
            compute_label_pos(&segments, text, bounds),
            Some(text.to_string()),
        ),
        _ => (None, None),
    };

    RoutedEdge {
        segments,
        label_pos,
        label,
    }
}

fn compute_label_pos(
    segments: &[(u16, u16, char)],
    text: &str,
    bounds: Rect,
) -> Option<(u16, u16)> {
    if text.is_empty() || segments.is_empty() {
        return None;
    }

    let max_x = bounds.x.saturating_add(bounds.width);
    let max_y = bounds.y.saturating_add(bounds.height);
    let text_len = u16::try_from(text.chars().count()).unwrap_or(u16::MAX);
    let is_segment = |x: u16, y: u16| segments.iter().any(|&(sx, sy, _)| sx == x && sy == y);
    let span_clear = |cx: u16, cy: u16| -> bool {
        if !in_bounds(cx, cy, bounds) {
            return false;
        }
        let end_x = cx.saturating_add(text_len).min(max_x);
        (cx..end_x).all(|x| !is_segment(x, cy))
    };
    let pick_clear = |candidates: &[(u16, u16)]| -> Option<(u16, u16)> {
        candidates
            .iter()
            .copied()
            .find(|&(cx, cy)| span_clear(cx, cy))
            .or_else(|| {
                candidates
                    .iter()
                    .copied()
                    .find(|&(cx, cy)| in_bounds(cx, cy, bounds) && !is_segment(cx, cy))
            })
    };

    let mid_idx = segments.len() / 2;
    let (mx, my, mch) = segments[mid_idx];
    let is_horizontal_seg = matches!(mch, '─' | '▶' | '◀');
    let is_corner = matches!(mch, '┐' | '┘' | '└' | '┌');

    if is_horizontal_seg {
        let mut run_min_x = mx;
        let mut run_max_x = mx;
        let mut i = mid_idx;
        while i > 0 && segments[i - 1].1 == my {
            i -= 1;
            run_min_x = run_min_x.min(segments[i].0);
            run_max_x = run_max_x.max(segments[i].0);
        }
        let mut j = mid_idx;
        while j + 1 < segments.len() && segments[j + 1].1 == my {
            j += 1;
            run_min_x = run_min_x.min(segments[j].0);
            run_max_x = run_max_x.max(segments[j].0);
        }

        let run_len = usize::from(run_max_x.saturating_sub(run_min_x)) + 1;
        let offset =
            u16::try_from(run_len.saturating_sub(text.chars().count()) / 2).unwrap_or(u16::MAX);
        let lx = run_min_x.saturating_add(offset);

        let mut candidates = Vec::new();
        if my > bounds.y {
            candidates.push((lx, my - 1));
        }
        if my.saturating_add(1) < max_y {
            candidates.push((lx, my + 1));
        }
        pick_clear(&candidates)
    } else if is_corner {
        let mut candidates = Vec::new();
        if my > bounds.y {
            candidates.push((mx, my - 1));
        }
        if my.saturating_add(1) < max_y {
            candidates.push((mx, my + 1));
        }
        if mx.saturating_add(1) < max_x {
            candidates.push((mx + 1, my));
        }
        if mx > bounds.x {
            candidates.push((mx - 1, my));
        }
        pick_clear(&candidates)
    } else {
        let mut candidates = Vec::new();
        if mx.saturating_add(1) < max_x {
            candidates.push((mx + 1, my));
        }
        if my > bounds.y {
            candidates.push((mx, my - 1));
        }
        if my.saturating_add(1) < max_y {
            candidates.push((mx, my + 1));
        }
        if mx > bounds.x {
            candidates.push((mx - 1, my));
        }
        pick_clear(&candidates)
    }
}

fn truncate_text(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn edge_kind_tag(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::DependsOn => "dep",
        EdgeKind::DataFlow => "data",
        EdgeKind::Renders => "ui",
        EdgeKind::Invokes => "call",
        EdgeKind::Contains => "sub",
        EdgeKind::Custom => "link",
    }
}

pub fn render_canvas(frame: &mut Frame, area: Rect, app: &App, viewport: CanvasViewport) {
    let grid = app.effective_grid();
    let title = format!(
        " Snap Grid ({}x{}) [{}] ",
        grid.cols,
        grid.rows,
        viewport.zoom.label()
    );
    let block = Block::bordered()
        .title(title)
        .border_style(style::panel_border_style(
            &app.theme,
            app.focus == FocusColumn::Canvas,
        ))
        .style(style::base_style(&app.theme));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut visible_cards: Vec<(usize, Rect)> = Vec::new();
    let mut card_rects: HashMap<&str, Rect> = HashMap::new();
    for idx in app.visible_component_indices() {
        if let Some(comp) = app.project.components.get(idx) {
            if let Some(rect) = card_rect(inner, viewport, comp.position) {
                visible_cards.push((idx, rect));
                card_rects.insert(comp.id.as_str(), rect);
            }
        }
    }

    let selected_id = app.selected_component().map(|c| c.id.as_str());

    let mut routed_edges: Vec<(RoutedEdge, Style)> = Vec::new();
    let mut segment_cells: Vec<(u16, u16)> = Vec::new();

    for conn in &app.project.config.connections {
        let (Some(&from_rect), Some(&to_rect)) = (
            card_rects.get(conn.from.as_str()),
            card_rects.get(conn.to.as_str()),
        ) else {
            continue;
        };

        let edge_label = if viewport.zoom == ZoomLevel::Compact {
            None
        } else {
            conn.label
                .as_deref()
                .filter(|s| !s.is_empty())
                .or(Some(edge_kind_tag(conn.kind)))
        };
        let routed = route_orthogonal_edge(from_rect, to_rect, edge_label, inner);

        let is_highlighted =
            selected_id == Some(conn.from.as_str()) || selected_id == Some(conn.to.as_str());
        let edge_style = if is_highlighted {
            Style::default()
                .fg(style::to_color(app.theme.palette.accent))
                .bg(style::to_color(app.theme.palette.background))
        } else {
            style::muted_style(&app.theme)
        };

        let buf = frame.buffer_mut();
        for &(x, y, ch) in &routed.segments {
            segment_cells.push((x, y));
            if let Some(cell) = buf.cell_mut((x, y)) {
                let mut ch_buf = [0u8; 4];
                cell.set_symbol(ch.encode_utf8(&mut ch_buf))
                    .set_style(edge_style);
            }
        }

        routed_edges.push((routed, edge_style));
    }

    let is_blocked = |x: u16, y: u16| -> bool {
        !in_bounds(x, y, inner)
            || segment_cells.contains(&(x, y))
            || visible_cards.iter().any(|&(_, rect)| in_bounds(x, y, rect))
    };

    let buf = frame.buffer_mut();
    let max_x = inner.x.saturating_add(inner.width);
    for (routed, edge_style) in &routed_edges {
        if let (Some((lx, ly)), Some(text)) = (routed.label_pos, routed.label.as_deref()) {
            if is_blocked(lx, ly) {
                continue;
            }
            let mut avail = 0usize;
            for x in lx..max_x {
                if is_blocked(x, ly) {
                    break;
                }
                avail += 1;
            }
            if avail == 0 {
                continue;
            }
            let clipped = truncate_text(text, avail);
            for (offset, ch) in clipped.chars().enumerate() {
                let Ok(dx) = u16::try_from(offset) else {
                    break;
                };
                let x = lx.saturating_add(dx);
                if is_blocked(x, ly) {
                    break;
                }
                if let Some(cell) = buf.cell_mut((x, ly)) {
                    let mut ch_buf = [0u8; 4];
                    cell.set_symbol(ch.encode_utf8(&mut ch_buf))
                        .set_style(*edge_style);
                }
            }
        }
    }

    for (idx, rect) in visible_cards {
        let Some(comp) = app.project.components.get(idx) else {
            continue;
        };
        let status = compute_card_status(comp);
        let progress = task_progress(comp);
        let glyph = style::status_glyph(status);
        let is_selected = selected_id == Some(comp.id.as_str());

        let border_type = if is_selected {
            BorderType::Double
        } else {
            BorderType::Plain
        };
        let mut border_style = Style::default().fg(style::card_border_color(&app.theme, status));
        if is_selected {
            border_style = border_style.add_modifier(Modifier::BOLD);
        }

        let inner_w = rect.width.saturating_sub(2) as usize;
        let inner_h = rect.height.saturating_sub(2) as usize;
        let sub_count = app.project.sub_components(&comp.id).len();
        let line1 = if sub_count > 0 {
            let badge = if app.is_expanded(&comp.id) {
                format!("[-{sub_count}]")
            } else {
                format!("[+{sub_count}]")
            };
            let badge_len = badge.chars().count();
            if inner_w > badge_len + 1 {
                let prefix =
                    truncate_text(&format!("{glyph} {}", comp.name), inner_w - badge_len - 1);
                format!("{prefix} {badge}")
            } else {
                truncate_text(&format!("{glyph} {} {badge}", comp.name), inner_w)
            }
        } else {
            truncate_text(&format!("{glyph} {}", comp.name), inner_w)
        };
        let archetype = archetype_for(comp.kind);
        let mut lines = match viewport.zoom {
            ZoomLevel::Compact => vec![
                Line::from(line1),
                Line::from(truncate_text(
                    &format!("Tasks {}/{}", progress.complete, progress.total),
                    inner_w,
                )),
            ],
            ZoomLevel::Normal => vec![
                Line::from(line1),
                Line::from(truncate_text(
                    &format!(
                        "Tasks {}/{} [{}]",
                        progress.complete, progress.total, archetype.label
                    ),
                    inner_w,
                )),
                Line::from(truncate_text(&comp.description, inner_w)),
            ],
            ZoomLevel::Expanded => {
                let tags_summary = if comp.tags.is_empty() {
                    "Tags: -".to_string()
                } else {
                    format!("Tags: {}", comp.tags.join(", "))
                };
                vec![
                    Line::from(line1),
                    Line::from(truncate_text(
                        &format!(
                            "Tasks {}/{} [{}]",
                            progress.complete, progress.total, archetype.label
                        ),
                        inner_w,
                    )),
                    Line::from(truncate_text(&comp.description, inner_w)),
                    Line::from(truncate_text(&tags_summary, inner_w)),
                    Line::from(truncate_text(
                        &format!("Feat: {} | Sub: {}", comp.features.len(), sub_count),
                        inner_w,
                    )),
                ]
            }
        };
        lines.truncate(inner_h);

        let card_block = Block::bordered()
            .border_type(border_type)
            .border_style(border_style)
            .style(style::base_style(&app.theme));
        let paragraph = Paragraph::new(lines).block(card_block);

        frame.render_widget(paragraph, rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_rect_projects_grid_positions_and_culls_offscreen() {
        let inner = Rect::new(0, 0, 60, 20);
        let viewport = CanvasViewport {
            zoom: ZoomLevel::Normal,
            ..Default::default()
        };

        assert_eq!(
            card_rect(inner, viewport, GridPos { col: 0, row: 0 }),
            Some(Rect::new(1, 1, 22, 5))
        );
        assert_eq!(
            card_rect(inner, viewport, GridPos { col: 1, row: 1 }),
            Some(Rect::new(27, 8, 22, 5))
        );
        assert!(card_rect(inner, viewport, GridPos { col: 5, row: 0 }).is_none());
    }

    #[test]
    fn ensure_visible_pans_viewport_to_target() {
        let inner = Rect::new(0, 0, 60, 20);
        let mut viewport = CanvasViewport::default();
        let target = GridPos { col: 4, row: 3 };

        ensure_visible(inner, &mut viewport, target);
        assert!(card_rect(inner, viewport, target).is_some());
    }

    #[test]
    fn card_anchor_points_centers_edges() {
        let anchors = card_anchor_points(Rect::new(10, 4, 22, 5));

        assert_eq!(anchors.top, (21, 4));
        assert_eq!(anchors.bottom, (21, 8));
        assert_eq!(anchors.left, (10, 6));
        assert_eq!(anchors.right, (31, 6));
    }

    #[test]
    fn route_horizontal_edge_emits_line_and_right_arrow() {
        let edge = route_orthogonal_edge(
            Rect::new(1, 1, 22, 5),
            Rect::new(27, 1, 22, 5),
            None,
            Rect::new(0, 0, 80, 25),
        );

        assert!(edge.segments.iter().any(|&(_, _, ch)| ch == '─'));
        assert_eq!(edge.segments.last().map(|&(_, _, ch)| ch), Some('▶'));
    }

    #[test]
    fn route_l_bend_edge_emits_corners_and_arrow() {
        let edge = route_orthogonal_edge(
            Rect::new(1, 1, 22, 5),
            Rect::new(27, 8, 22, 5),
            Some("events"),
            Rect::new(0, 0, 80, 25),
        );

        assert!(edge.segments.iter().any(|&(_, _, ch)| ch == '│'));
        assert_eq!(edge.segments.last().map(|&(_, _, ch)| ch), Some('▶'));
        assert_eq!(edge.label.as_deref(), Some("events"));
    }

    #[test]
    fn route_vertical_edge_emits_down_arrow() {
        let edge = route_orthogonal_edge(
            Rect::new(1, 1, 22, 5),
            Rect::new(1, 8, 22, 5),
            None,
            Rect::new(0, 0, 80, 25),
        );

        assert_eq!(edge.segments.last().map(|&(_, _, ch)| ch), Some('▼'));
    }

    fn make_canvas_test_app(from_pos: GridPos, to_pos: GridPos, label: Option<&str>) -> App {
        use std::path::PathBuf;

        use crate::model::{
            Component, ComponentKind, Connection, GridConfig, Project, ProjectConfig,
        };
        use crate::theme::default_theme;

        let alpha = Component {
            id: "alpha".to_string(),
            name: "Alpha".to_string(),
            kind: ComponentKind::UiView,
            parent_id: None,
            description: "Source node".to_string(),
            relative_path: Some("src/alpha.rs".to_string()),
            position: from_pos,
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        };

        let beta = Component {
            id: "beta".to_string(),
            name: "Beta".to_string(),
            kind: ComponentKind::ServiceCore,
            parent_id: None,
            description: "Target node".to_string(),
            relative_path: Some("src/beta.rs".to_string()),
            position: to_pos,
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        };

        let project = Project {
            config: ProjectConfig {
                name: "Canvas Test".to_string(),
                description: "Edge label test".to_string(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig::default(),
                connections: vec![Connection {
                    from: "alpha".to_string(),
                    to: "beta".to_string(),
                    kind: EdgeKind::DataFlow,
                    label: label.map(str::to_string),
                }],
            },
            components: vec![alpha, beta],
        };

        let theme = default_theme().unwrap();
        App::new(PathBuf::from("/tmp/example-project"), project, theme)
    }

    fn render_canvas_buffer_with_zoom(
        app: &App,
        width: u16,
        height: u16,
        zoom: ZoomLevel,
    ) -> ratatui::buffer::Buffer {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let viewport = CanvasViewport {
            zoom,
            ..Default::default()
        };
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_canvas(frame, area, app, viewport);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn render_canvas_buffer(app: &App, width: u16, height: u16) -> ratatui::buffer::Buffer {
        render_canvas_buffer_with_zoom(app, width, height, ZoomLevel::Normal)
    }

    fn row_text(buf: &ratatui::buffer::Buffer, y: u16) -> String {
        (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>()
    }

    #[test]
    fn render_canvas_preserves_arrowhead_and_renders_short_horizontal_label() {
        let app = make_canvas_test_app(
            GridPos { col: 0, row: 0 },
            GridPos { col: 1, row: 0 },
            Some("dep"),
        );
        let buf = render_canvas_buffer(&app, 60, 15);
        let inner = Rect::new(1, 1, 58, 13);
        let normal_vp = CanvasViewport {
            zoom: ZoomLevel::Normal,
            ..Default::default()
        };
        let to_rect = card_rect(inner, normal_vp, GridPos { col: 1, row: 0 }).unwrap();
        let edge_y = to_rect.y + to_rect.height / 2;

        assert_eq!(buf[(to_rect.x - 1, edge_y)].symbol(), "▶");
        assert!(row_text(&buf, edge_y - 1).contains("dep"));
    }

    #[test]
    fn render_canvas_clips_long_horizontal_label_without_corrupting_arrowhead_or_card_border() {
        let app = make_canvas_test_app(
            GridPos { col: 0, row: 0 },
            GridPos { col: 1, row: 0 },
            Some("events_stream"),
        );
        let buf = render_canvas_buffer(&app, 60, 15);
        let inner = Rect::new(1, 1, 58, 13);
        let normal_vp = CanvasViewport {
            zoom: ZoomLevel::Normal,
            ..Default::default()
        };
        let to_rect = card_rect(inner, normal_vp, GridPos { col: 1, row: 0 }).unwrap();
        let edge_y = to_rect.y + to_rect.height / 2;

        assert_eq!(buf[(to_rect.x - 1, edge_y)].symbol(), "▶");
        assert!(row_text(&buf, edge_y - 1).contains("eve…"));
        assert_eq!(buf[(to_rect.x, edge_y - 1)].symbol(), "│");
        assert_eq!(buf[(to_rect.x, edge_y)].symbol(), "│");
    }

    #[test]
    fn render_canvas_renders_full_label_when_cards_have_horizontal_spacing() {
        let app = make_canvas_test_app(
            GridPos { col: 0, row: 0 },
            GridPos { col: 2, row: 0 },
            Some("events"),
        );
        let buf = render_canvas_buffer(&app, 90, 15);
        let inner = Rect::new(1, 1, 88, 13);
        let normal_vp = CanvasViewport {
            zoom: ZoomLevel::Normal,
            ..Default::default()
        };
        let to_rect = card_rect(inner, normal_vp, GridPos { col: 2, row: 0 }).unwrap();
        let edge_y = to_rect.y + to_rect.height / 2;

        assert_eq!(buf[(to_rect.x - 1, edge_y)].symbol(), "▶");
        assert!(row_text(&buf, edge_y - 1).contains("events"));
    }

    #[test]
    fn compute_label_pos_avoids_horizontal_segment_cells_at_l_bend_corner() {
        let bounds = Rect::new(0, 0, 80, 25);
        // Short L-bend whose midpoint lands directly on the '└' corner at (24, 4).
        let edge = route_orthogonal_edge(
            Rect::new(1, 1, 22, 5),
            Rect::new(27, 2, 22, 5),
            Some("bus"),
            bounds,
        );
        let label_pos = edge.label_pos.expect("expected label position");
        for &(sx, sy, _) in &edge.segments {
            assert_ne!(
                label_pos,
                (sx, sy),
                "label start must not collide with segment ({sx}, {sy})"
            );
        }
        // Corner '└' is at (24, 4) with '┐' above at (24, 3), so label should be below at (24, 5).
        assert_eq!(label_pos, (24, 5));
    }

    #[test]
    fn render_canvas_l_bend_preserves_all_segments_corners_and_arrowhead() {
        let app = make_canvas_test_app(
            GridPos { col: 0, row: 0 },
            GridPos { col: 1, row: 1 },
            Some("events"),
        );
        let buf = render_canvas_buffer(&app, 70, 20);
        let inner = Rect::new(1, 1, 68, 18);
        let normal_vp = CanvasViewport {
            zoom: ZoomLevel::Normal,
            ..Default::default()
        };
        let from_rect = card_rect(inner, normal_vp, GridPos { col: 0, row: 0 }).unwrap();
        let to_rect = card_rect(inner, normal_vp, GridPos { col: 1, row: 1 }).unwrap();
        let expected = route_orthogonal_edge(from_rect, to_rect, Some("events"), inner);

        for &(x, y, ch) in &expected.segments {
            let mut ch_buf = [0u8; 4];
            let expected_sym = ch.encode_utf8(&mut ch_buf);
            assert_eq!(
                buf[(x, y)].symbol(),
                expected_sym,
                "segment at ({x}, {y}) was overwritten"
            );
        }
        assert_eq!(expected.segments.last().map(|&(_, _, ch)| ch), Some('▶'));
    }

    #[test]
    fn render_canvas_displays_sub_component_badges_and_archetype_label() {
        use crate::model::{Component, ComponentKind};

        let mut app =
            make_canvas_test_app(GridPos { col: 0, row: 0 }, GridPos { col: 1, row: 0 }, None);
        app.project.components[0].kind = ComponentKind::Logic;
        app.project.components.push(Component {
            id: "alpha_child".to_string(),
            name: "Alpha Child".to_string(),
            kind: ComponentKind::Data,
            parent_id: Some("alpha".to_string()),
            description: "Child node".to_string(),
            relative_path: Some("src/alpha_child.rs".to_string()),
            position: GridPos { col: 0, row: 1 },
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        });

        let collapsed_buf = render_canvas_buffer(&app, 60, 20);
        let collapsed_text: String = (0..collapsed_buf.area.height)
            .map(|y| row_text(&collapsed_buf, y))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(collapsed_text.contains("[+1]"));
        assert!(!collapsed_text.contains("[-1]"));
        assert!(collapsed_text.contains("[Logic]"));

        app.expanded_components.insert("alpha".to_string());
        let expanded_buf = render_canvas_buffer(&app, 60, 20);
        let expanded_text: String = (0..expanded_buf.area.height)
            .map(|y| row_text(&expanded_buf, y))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(expanded_text.contains("[-1]"));
        assert!(!expanded_text.contains("[+1]"));
        assert!(expanded_text.contains("Alpha Child"));
    }

    #[test]
    fn zoom_level_transitions_and_compact_4x4_capacity() {
        use crate::model::FeatureItem;

        assert_eq!(ZoomLevel::default(), ZoomLevel::Compact);
        assert_eq!(ZoomLevel::Compact.zoom_in(), ZoomLevel::Normal);
        assert_eq!(ZoomLevel::Normal.zoom_in(), ZoomLevel::Expanded);
        assert_eq!(ZoomLevel::Expanded.zoom_in(), ZoomLevel::Expanded);
        assert_eq!(ZoomLevel::Expanded.zoom_out(), ZoomLevel::Normal);
        assert_eq!(ZoomLevel::Normal.zoom_out(), ZoomLevel::Compact);
        assert_eq!(ZoomLevel::Compact.zoom_out(), ZoomLevel::Compact);

        let compact_inner = Rect::new(0, 0, 63, 20);
        let (cols, rows) = visible_grid_capacity_for(compact_inner, ZoomLevel::Compact);
        assert!(cols >= 4 && rows >= 4, "expected >=4x4, got {cols}x{rows}");
        assert!(card_rect(
            compact_inner,
            CanvasViewport::default(),
            GridPos { col: 3, row: 3 }
        )
        .is_some());

        // Compact suppresses edge text labels while preserving arrowheads.
        let mut app = make_canvas_test_app(
            GridPos { col: 0, row: 0 },
            GridPos { col: 1, row: 0 },
            Some("dep"),
        );
        let compact_buf = render_canvas_buffer_with_zoom(&app, 65, 22, ZoomLevel::Compact);
        let compact_text: String = (0..compact_buf.area.height)
            .map(|y| row_text(&compact_buf, y))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(compact_text.contains("[Compact]"));
        assert!(compact_text.contains('▶'));
        assert!(!compact_text.contains("dep"));

        // Expanded renders extra metadata lines (tags and features/sub-components summary).
        app.project.components[0].tags = vec!["ui".to_string(), "core".to_string()];
        app.project.components[0].features = vec![FeatureItem {
            id: "feat_1".to_string(),
            title: "Render canvas".to_string(),
            pushed: false,
            external_id: None,
        }];
        let expanded_buf = render_canvas_buffer_with_zoom(&app, 80, 14, ZoomLevel::Expanded);
        let expanded_text: String = (0..expanded_buf.area.height)
            .map(|y| row_text(&expanded_buf, y))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(expanded_text.contains("[Expanded]"));
        assert!(expanded_text.contains("Tags: ui, core"));
        assert!(expanded_text.contains("Feat: 1 | Sub: 0"));

        // Narrow-canvas fallback clamps the top-left visible card instead of drawing zero cards.
        let narrow_inner = Rect::new(0, 0, 20, 6);
        let expanded_vp = CanvasViewport {
            zoom: ZoomLevel::Expanded,
            ..Default::default()
        };
        let fallback_rect =
            card_rect(narrow_inner, expanded_vp, GridPos { col: 0, row: 0 }).unwrap();
        assert_eq!(fallback_rect, Rect::new(1, 1, 19, 5));
        let narrow_buf = render_canvas_buffer_with_zoom(&app, 22, 8, ZoomLevel::Expanded);
        let narrow_text: String = (0..narrow_buf.area.height)
            .map(|y| row_text(&narrow_buf, y))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(narrow_text.contains("Alpha"));
    }

    fn rect_text(buf: &ratatui::buffer::Buffer, area: Rect) -> String {
        (area.y..area.y.saturating_add(area.height))
            .map(|y| {
                (area.x..area.x.saturating_add(area.width))
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn card_rect_narrow_canvas_draws_selected_row_one() {
        use ratatui::backend::TestBackend;
        use ratatui::layout::{Constraint, Layout};
        use ratatui::widgets::Borders;
        use ratatui::Terminal;

        let mut app =
            make_canvas_test_app(GridPos { col: 0, row: 0 }, GridPos { col: 0, row: 1 }, None);
        app.selected_index = 1;
        app.viewport = CanvasViewport {
            scroll_col: 0,
            scroll_row: 0,
            zoom: ZoomLevel::Expanded,
        };

        let term_area = Rect::new(0, 0, 60, 24);
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(term_area);
        let cols = crate::tui::ui::column_layout(chunks[1]);
        let canvas_inner = Block::default().borders(Borders::ALL).inner(cols[1]);

        let rect = card_rect(canvas_inner, app.viewport, GridPos { col: 0, row: 1 });
        assert!(
            rect.is_some(),
            "expected card_rect to return Some for (0, 1) on narrow Expanded canvas"
        );

        let backend = TestBackend::new(term_area.width, term_area.height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| crate::tui::ui::render(frame, &mut app))
            .unwrap();

        let canvas_text = rect_text(terminal.backend().buffer(), cols[1]);
        assert!(
            canvas_text.contains("Beta"),
            "expected selected row-1 card 'Beta' in narrow canvas:\n{canvas_text}"
        );
    }

    #[test]
    fn zoom_out_pulls_scroll_back_to_show_all_fitting_cards() {
        use std::path::PathBuf;

        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        use ratatui::backend::TestBackend;
        use ratatui::layout::{Constraint, Layout};
        use ratatui::Terminal;

        use crate::model::{Component, ComponentKind, GridConfig, Project, ProjectConfig};
        use crate::theme::default_theme;

        let names = ["Alpha", "Beta", "Gamma", "Delta"];
        let components = names
            .iter()
            .enumerate()
            .map(|(col, name)| Component {
                id: name.to_lowercase(),
                name: (*name).to_string(),
                kind: ComponentKind::Logic,
                parent_id: None,
                description: format!("{name} service"),
                relative_path: Some(format!("src/{}.rs", name.to_lowercase())),
                position: GridPos {
                    col: col as u16,
                    row: 0,
                },
                tags: Vec::new(),
                tech_stack: Vec::new(),
                files: Vec::new(),
                flags: Vec::new(),
                features: Vec::new(),
                external_tasks: Vec::new(),
                markdown_body: String::new(),
            })
            .collect();

        let project = Project {
            config: ProjectConfig {
                name: "Zoom Scroll Test".to_string(),
                description: "Scroll pullback test".to_string(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig::default(),
                connections: Vec::new(),
            },
            components,
        };

        let theme = default_theme().unwrap();
        let mut app = App::new(PathBuf::from("/tmp/example-project"), project, theme);
        app.selected_index = 3;

        let term_area = Rect::new(0, 0, 140, 40);
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(term_area);
        let cols = crate::tui::ui::column_layout(chunks[1]);

        let backend = TestBackend::new(term_area.width, term_area.height);
        let mut terminal = Terminal::new(backend).unwrap();

        let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::NONE);

        // Zoom in twice (Compact -> Normal -> Expanded) and render so scroll_col pans right.
        app.handle_key(key(KeyCode::Char('+')));
        app.handle_key(key(KeyCode::Char('+')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Expanded);
        terminal
            .draw(|frame| crate::tui::ui::render(frame, &mut app))
            .unwrap();
        assert!(
            app.viewport.scroll_col > 0,
            "expected scroll_col > 0 at Expanded zoom with col 3 selected"
        );

        // Zoom out twice (Expanded -> Normal -> Compact) and render; scroll_col must pull back to 0.
        app.handle_key(key(KeyCode::Char('-')));
        app.handle_key(key(KeyCode::Char('-')));
        assert_eq!(app.viewport.zoom, ZoomLevel::Compact);
        terminal
            .draw(|frame| crate::tui::ui::render(frame, &mut app))
            .unwrap();

        assert_eq!(app.viewport.scroll_col, 0);
        let canvas_text = rect_text(terminal.backend().buffer(), cols[1]);
        for name in names {
            assert!(
                canvas_text.contains(name),
                "expected card '{name}' in canvas buffer after zooming out:\n{canvas_text}"
            );
        }
    }

    #[test]
    fn render_canvas_compact_renders_l_bend_edge() {
        let app = make_canvas_test_app(
            GridPos { col: 0, row: 0 },
            GridPos { col: 1, row: 1 },
            Some("events"),
        );
        let buf = render_canvas_buffer_with_zoom(&app, 60, 20, ZoomLevel::Compact);
        let inner = Rect::new(1, 1, 58, 18);
        let compact_vp = CanvasViewport {
            zoom: ZoomLevel::Compact,
            ..Default::default()
        };
        let from_rect = card_rect(inner, compact_vp, GridPos { col: 0, row: 0 }).unwrap();
        let to_rect = card_rect(inner, compact_vp, GridPos { col: 1, row: 1 }).unwrap();
        let expected = route_orthogonal_edge(from_rect, to_rect, Some("events"), inner);

        let has_corner = expected
            .segments
            .iter()
            .any(|&(_, _, ch)| matches!(ch, '┐' | '└' | '┌' | '┘'));
        assert!(
            has_corner,
            "expected at least one L-bend box-drawing corner in routed Compact edge"
        );

        let last_ch = expected.segments.last().map(|&(_, _, ch)| ch);
        assert!(
            matches!(last_ch, Some('▶' | '▼')),
            "expected arrowhead '▶' or '▼', got {last_ch:?}"
        );

        for &(x, y, ch) in &expected.segments {
            let mut ch_buf = [0u8; 4];
            let expected_sym = ch.encode_utf8(&mut ch_buf);
            assert_eq!(
                buf[(x, y)].symbol(),
                expected_sym,
                "Compact L-bend segment at ({x}, {y}) missing or overwritten"
            );
        }
    }
}
