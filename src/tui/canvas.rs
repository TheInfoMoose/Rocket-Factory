use std::collections::HashMap;

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CanvasViewport {
    pub scroll_col: u16,
    pub scroll_row: u16,
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

    let rel_col = pos.col - viewport.scroll_col;
    let rel_row = pos.row - viewport.scroll_row;

    let x = inner
        .x
        .saturating_add(1)
        .saturating_add(rel_col.saturating_mul(CELL_STRIDE_X));
    let y = inner
        .y
        .saturating_add(1)
        .saturating_add(rel_row.saturating_mul(CELL_STRIDE_Y));

    if x.saturating_add(CARD_WIDTH) > inner.x.saturating_add(inner.width)
        || y.saturating_add(CARD_HEIGHT) > inner.y.saturating_add(inner.height)
    {
        return None;
    }

    Some(Rect {
        x,
        y,
        width: CARD_WIDTH,
        height: CARD_HEIGHT,
    })
}

pub fn visible_grid_capacity(inner: Rect) -> (u16, u16) {
    let cols = (inner.width.saturating_sub(1).saturating_add(GUTTER_X) / CELL_STRIDE_X).max(1);
    let rows = (inner.height.saturating_sub(1).saturating_add(GUTTER_Y) / CELL_STRIDE_Y).max(1);
    (cols, rows)
}

pub fn ensure_visible(inner: Rect, viewport: &mut CanvasViewport, pos: GridPos) {
    let (vis_cols, vis_rows) = visible_grid_capacity(inner);

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
        Some(text) if !text.is_empty() && !segments.is_empty() => {
            let mid = segments[segments.len() / 2];
            (Some((mid.0, mid.1)), Some(text.to_string()))
        }
        _ => (None, None),
    };

    RoutedEdge {
        segments,
        label_pos,
        label,
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
        EdgeKind::Custom => "link",
    }
}

pub fn render_canvas(frame: &mut Frame, area: Rect, app: &App, viewport: CanvasViewport) {
    let title = format!(
        " Snap Grid ({}x{}) ",
        app.project.config.grid.cols, app.project.config.grid.rows
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

    for conn in &app.project.config.connections {
        let (Some(&from_rect), Some(&to_rect)) = (
            card_rects.get(conn.from.as_str()),
            card_rects.get(conn.to.as_str()),
        ) else {
            continue;
        };

        let edge_label = conn
            .label
            .as_deref()
            .filter(|s| !s.is_empty())
            .or(Some(edge_kind_tag(conn.kind)));
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
            if let Some(cell) = buf.cell_mut((x, y)) {
                let mut ch_buf = [0u8; 4];
                cell.set_symbol(ch.encode_utf8(&mut ch_buf))
                    .set_style(edge_style);
            }
        }

        if let (Some((lx, ly)), Some(text)) = (routed.label_pos, routed.label.as_deref()) {
            let max_x = inner.x.saturating_add(inner.width);
            let max_y = inner.y.saturating_add(inner.height);
            if lx >= inner.x && lx < max_x && ly >= inner.y && ly < max_y {
                let avail = max_x.saturating_sub(lx) as usize;
                let clipped = truncate_text(text, avail);
                buf.set_string(lx, ly, clipped, edge_style);
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
        let line1 = truncate_text(&format!("{glyph} {}", comp.name), inner_w);
        let line2 = truncate_text(
            &format!(
                "Tasks {}/{} [{:?}]",
                progress.complete, progress.total, comp.kind
            ),
            inner_w,
        );
        let line3 = truncate_text(&comp.description, inner_w);

        let card_block = Block::bordered()
            .border_type(border_type)
            .border_style(border_style)
            .style(style::base_style(&app.theme));
        let paragraph = Paragraph::new(vec![
            Line::from(line1),
            Line::from(line2),
            Line::from(line3),
        ])
        .block(card_block);

        frame.render_widget(paragraph, rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_rect_projects_grid_positions_and_culls_offscreen() {
        let inner = Rect::new(0, 0, 60, 20);
        let viewport = CanvasViewport::default();

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
}
