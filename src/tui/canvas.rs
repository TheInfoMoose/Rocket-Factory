use std::collections::{HashMap, HashSet};

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
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

fn display_width(s: &str) -> usize {
    Span::raw(s).width()
}

fn char_display_width(ch: char) -> usize {
    let mut buf = [0u8; 4];
    let s: &str = ch.encode_utf8(&mut buf);
    Span::raw(s).width()
}

fn is_arrow(ch: char) -> bool {
    matches!(ch, '▶' | '◀' | '▼' | '▲')
}

fn corridor_mask(ch: char) -> u8 {
    const UP: u8 = 0b0001;
    const DOWN: u8 = 0b0010;
    const LEFT: u8 = 0b0100;
    const RIGHT: u8 = 0b1000;
    match ch {
        '─' => LEFT | RIGHT,
        '│' => UP | DOWN,
        '┌' => DOWN | RIGHT,
        '┐' => DOWN | LEFT,
        '└' => UP | RIGHT,
        '┘' => UP | LEFT,
        '├' => UP | DOWN | RIGHT,
        '┤' => UP | DOWN | LEFT,
        '┬' => DOWN | LEFT | RIGHT,
        '┴' => UP | LEFT | RIGHT,
        '┼' => UP | DOWN | LEFT | RIGHT,
        _ => 0,
    }
}

fn mask_to_corridor_glyph(mask: u8) -> Option<char> {
    const UP: u8 = 0b0001;
    const DOWN: u8 = 0b0010;
    const LEFT: u8 = 0b0100;
    const RIGHT: u8 = 0b1000;
    match mask {
        m if m == LEFT | RIGHT => Some('─'),
        m if m == UP | DOWN => Some('│'),
        m if m == DOWN | RIGHT => Some('┌'),
        m if m == DOWN | LEFT => Some('┐'),
        m if m == UP | RIGHT => Some('└'),
        m if m == UP | LEFT => Some('┘'),
        m if m == UP | DOWN | RIGHT => Some('├'),
        m if m == UP | DOWN | LEFT => Some('┤'),
        m if m == DOWN | LEFT | RIGHT => Some('┬'),
        m if m == UP | LEFT | RIGHT => Some('┴'),
        m if m == UP | DOWN | LEFT | RIGHT => Some('┼'),
        _ => None,
    }
}

fn merge_corridor_glyph(existing: char, incoming: char) -> char {
    if is_arrow(incoming) {
        return incoming;
    }
    if is_arrow(existing) {
        return existing;
    }
    let m1 = corridor_mask(existing);
    let m2 = corridor_mask(incoming);
    if m1 != 0 && m2 != 0 {
        if let Some(merged) = mask_to_corridor_glyph(m1 | m2) {
            return merged;
        }
    }
    incoming
}

pub fn route_orthogonal_edge(
    from_rect: Rect,
    to_rect: Rect,
    label: Option<&str>,
    bounds: Rect,
) -> RoutedEdge {
    if from_rect.width == 0 || from_rect.height == 0 || to_rect.width == 0 || to_rect.height == 0 {
        return RoutedEdge {
            segments: Vec::new(),
            label_pos: None,
            label: None,
        };
    }

    let from_a = card_anchor_points(from_rect);
    let to_a = card_anchor_points(to_rect);

    let from_right = from_rect.x.saturating_add(from_rect.width);
    let to_right = to_rect.x.saturating_add(to_rect.width);
    let from_bottom = from_rect.y.saturating_add(from_rect.height);
    let to_bottom = to_rect.y.saturating_add(to_rect.height);

    let (start, end, arrow, horizontal_first) = if to_rect.x > from_right {
        (
            (from_a.right.0.saturating_add(1), from_a.right.1),
            (to_a.left.0.saturating_sub(1), to_a.left.1),
            '▶',
            true,
        )
    } else if from_rect.x > to_right {
        (
            (from_a.left.0.saturating_sub(1), from_a.left.1),
            (to_a.right.0.saturating_add(1), to_a.right.1),
            '◀',
            true,
        )
    } else if to_rect.y > from_bottom {
        (
            (from_a.bottom.0, from_a.bottom.1.saturating_add(1)),
            (to_a.top.0, to_a.top.1.saturating_sub(1)),
            '▼',
            false,
        )
    } else if from_rect.y > to_bottom {
        (
            (from_a.top.0, from_a.top.1.saturating_sub(1)),
            (to_a.bottom.0, to_a.bottom.1.saturating_add(1)),
            '▲',
            false,
        )
    } else {
        return RoutedEdge {
            segments: Vec::new(),
            label_pos: None,
            label: None,
        };
    };

    if in_bounds(start.0, start.1, from_rect)
        || in_bounds(start.0, start.1, to_rect)
        || in_bounds(end.0, end.1, from_rect)
        || in_bounds(end.0, end.1, to_rect)
    {
        return RoutedEdge {
            segments: Vec::new(),
            label_pos: None,
            label: None,
        };
    }

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
                    for y in sy.saturating_add(1)..ey {
                        raw.push((mid_x, y, '│'));
                    }
                } else {
                    for y in (ey.saturating_add(1)..sy).rev() {
                        raw.push((mid_x, y, '│'));
                    }
                }
                let corner2 = if sy < ey { '└' } else { '┌' };
                raw.push((mid_x, ey, corner2));
                if mid_x < ex {
                    for x in mid_x.saturating_add(1)..=ex {
                        raw.push((x, ey, '─'));
                    }
                }
            } else {
                if mid_x < sx {
                    for x in (mid_x.saturating_add(1)..=sx).rev() {
                        raw.push((x, sy, '─'));
                    }
                }
                let corner1 = if sy < ey { '┌' } else { '└' };
                raw.push((mid_x, sy, corner1));
                if sy < ey {
                    for y in sy.saturating_add(1)..ey {
                        raw.push((mid_x, y, '│'));
                    }
                } else {
                    for y in (ey.saturating_add(1)..sy).rev() {
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
                for x in sx.saturating_add(1)..ex {
                    raw.push((x, mid_y, '─'));
                }
            } else {
                for x in (ex.saturating_add(1)..sx).rev() {
                    raw.push((x, mid_y, '─'));
                }
            }
            let corner2 = if sx < ex { '┐' } else { '┌' };
            raw.push((ex, mid_y, corner2));
            if mid_y < ey {
                for y in mid_y.saturating_add(1)..=ey {
                    raw.push((ex, y, '│'));
                }
            }
        } else {
            if mid_y < sy {
                for y in (mid_y.saturating_add(1)..=sy).rev() {
                    raw.push((sx, y, '│'));
                }
            }
            let corner1 = if sx < ex { '┌' } else { '┐' };
            raw.push((sx, mid_y, corner1));
            if sx < ex {
                for x in sx.saturating_add(1)..ex {
                    raw.push((x, mid_y, '─'));
                }
            } else {
                for x in (ex.saturating_add(1)..sx).rev() {
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
        .filter(|&(x, y, _)| {
            in_bounds(x, y, bounds) && !in_bounds(x, y, from_rect) && !in_bounds(x, y, to_rect)
        })
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
    let text_width = display_width(text);
    let text_len = u16::try_from(text_width).unwrap_or(u16::MAX);
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
        let offset = u16::try_from(run_len.saturating_sub(text_width) / 2).unwrap_or(u16::MAX);
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

fn truncate_text(s: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    if display_width(s) <= max_cols {
        return s.to_string();
    }
    let ellipsis_w = display_width("…");
    if max_cols <= ellipsis_w {
        return "…".to_string();
    }
    let target = max_cols.saturating_sub(ellipsis_w);
    let mut out = String::new();
    let mut used = 0usize;
    for ch in s.chars() {
        let ch_w = char_display_width(ch);
        if used + ch_w > target {
            break;
        }
        out.push(ch);
        used += ch_w;
    }
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

fn alternate_label_candidates(
    segments: &[(u16, u16, char)],
    primary: Option<(u16, u16)>,
    label_width: usize,
    bounds: Rect,
) -> Vec<(u16, u16)> {
    let mut candidates: Vec<(u16, u16)> = Vec::new();
    let max_x = bounds.x.saturating_add(bounds.width);
    let max_y = bounds.y.saturating_add(bounds.height);
    let label_w_u16 = u16::try_from(label_width).unwrap_or(u16::MAX);

    let mut push_unique = |pos: (u16, u16)| {
        if in_bounds(pos.0, pos.1, bounds) && !candidates.contains(&pos) {
            candidates.push(pos);
        }
    };

    if let Some((px, py)) = primary {
        push_unique((px, py));
    }

    if segments.is_empty() {
        return candidates;
    }

    let mid_idx = segments.len() / 2;
    let mut order = Vec::with_capacity(segments.len());
    order.push(mid_idx);
    for dist in 1..segments.len() {
        if let Some(left) = mid_idx.checked_sub(dist) {
            order.push(left);
        }
        let right = mid_idx + dist;
        if right < segments.len() {
            order.push(right);
        }
    }

    for idx in order {
        let (sx, sy, ch) = segments[idx];
        if matches!(ch, '─' | '▶' | '◀') {
            if sy > bounds.y {
                push_unique((sx, sy - 1));
            }
            if sy.saturating_add(1) < max_y {
                push_unique((sx, sy.saturating_add(1)));
            }
        } else if matches!(ch, '│' | '▼' | '▲') {
            if sx.saturating_add(1) < max_x {
                push_unique((sx.saturating_add(1), sy));
            }
            if sx > bounds.x {
                push_unique((sx.saturating_sub(label_w_u16).max(bounds.x), sy));
                push_unique((sx - 1, sy));
            }
        } else {
            if sy > bounds.y {
                push_unique((sx, sy - 1));
            }
            if sy.saturating_add(1) < max_y {
                push_unique((sx, sy.saturating_add(1)));
            }
            if sx.saturating_add(1) < max_x {
                push_unique((sx.saturating_add(1), sy));
            }
            if sx > bounds.x {
                push_unique((sx - 1, sy));
            }
        }
    }

    candidates
}

pub fn render_canvas(frame: &mut Frame, area: Rect, app: &App, viewport: CanvasViewport) {
    let visible = app.visible_component_indices();
    let mut child_counts: HashMap<&str, usize> = HashMap::new();
    for comp in &app.project.components {
        if let Some(pid) = comp.parent_id.as_deref() {
            *child_counts.entry(pid).or_insert(0) += 1;
        }
    }
    render_canvas_with_cache(frame, area, app, viewport, &visible, &child_counts);
}

pub(crate) fn render_canvas_with_cache(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    viewport: CanvasViewport,
    visible: &[usize],
    child_counts: &HashMap<&str, usize>,
) {
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
    for &idx in visible {
        if let Some(comp) = app.project.components.get(idx) {
            if let Some(rect) = card_rect(inner, viewport, comp.position) {
                visible_cards.push((idx, rect));
                card_rects.insert(comp.id.as_str(), rect);
            }
        }
    }

    let selected_id = visible
        .get(app.selected_index)
        .and_then(|&idx| app.project.components.get(idx))
        .map(|c| c.id.as_str());

    let highlight_style = Style::default()
        .fg(style::to_color(app.theme.palette.accent))
        .bg(style::to_color(app.theme.palette.background));
    let normal_style = style::muted_style(&app.theme);

    let mut routed_edges: Vec<(RoutedEdge, Style)> = Vec::new();
    let mut corridor_cells: HashMap<(u16, u16), (char, bool)> = HashMap::new();
    let mut segment_cells: HashSet<(u16, u16)> = HashSet::new();

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
                .map(crate::tui::ui::sanitize_display_text)
                .filter(|s| !s.is_empty())
                .or_else(|| Some(edge_kind_tag(conn.kind).to_string()))
        };
        let routed = route_orthogonal_edge(from_rect, to_rect, edge_label.as_deref(), inner);

        let is_highlighted =
            selected_id == Some(conn.from.as_str()) || selected_id == Some(conn.to.as_str());
        let edge_style = if is_highlighted {
            highlight_style
        } else {
            normal_style
        };

        for &(x, y, ch) in &routed.segments {
            segment_cells.insert((x, y));
            corridor_cells
                .entry((x, y))
                .and_modify(|(existing_ch, existing_hi)| {
                    *existing_ch = merge_corridor_glyph(*existing_ch, ch);
                    *existing_hi = *existing_hi || is_highlighted;
                })
                .or_insert((ch, is_highlighted));
        }

        routed_edges.push((routed, edge_style));
    }

    {
        let buf = frame.buffer_mut();
        for (&(x, y), &(ch, is_highlighted)) in &corridor_cells {
            if let Some(cell) = buf.cell_mut((x, y)) {
                let mut ch_buf = [0u8; 4];
                let cell_style = if is_highlighted {
                    highlight_style
                } else {
                    normal_style
                };
                cell.set_symbol(ch.encode_utf8(&mut ch_buf))
                    .set_style(cell_style);
            }
        }
    }

    let mut occupied_label_cells: HashSet<(u16, u16)> = HashSet::new();
    let buf = frame.buffer_mut();
    let max_x = inner.x.saturating_add(inner.width);

    for (routed, edge_style) in &routed_edges {
        let Some(text) = routed.label.as_deref() else {
            continue;
        };
        let safe_label = crate::tui::ui::sanitize_display_text(text);
        let label_width = display_width(&safe_label);
        if label_width == 0 {
            continue;
        }

        let is_blocked = |x: u16, y: u16, occupied: &HashSet<(u16, u16)>| -> bool {
            !in_bounds(x, y, inner)
                || segment_cells.contains(&(x, y))
                || occupied.contains(&(x, y))
                || visible_cards.iter().any(|&(_, rect)| in_bounds(x, y, rect))
        };

        let avail_at = |cx: u16, cy: u16, occupied: &HashSet<(u16, u16)>| -> usize {
            if is_blocked(cx, cy, occupied) {
                return 0;
            }
            let mut avail = 0usize;
            for x in cx..max_x {
                if is_blocked(x, cy, occupied) {
                    break;
                }
                avail += 1;
            }
            avail
        };

        let candidates =
            alternate_label_candidates(&routed.segments, routed.label_pos, label_width, inner);
        let mut chosen: Option<(u16, u16, usize)> = None;
        for (cx, cy) in candidates {
            let avail = avail_at(cx, cy, &occupied_label_cells);
            if avail == 0 {
                continue;
            }
            if avail >= label_width {
                chosen = Some((cx, cy, avail));
                break;
            }
            if chosen.is_none_or(|(_, _, best_avail)| avail > best_avail) {
                chosen = Some((cx, cy, avail));
            }
        }

        let Some((lx, ly, avail)) = chosen else {
            continue;
        };

        let clipped = truncate_text(&safe_label, avail);
        let mut x = lx;
        for ch in clipped.chars() {
            let w = char_display_width(ch);
            if w == 0 {
                continue;
            }
            let Ok(w_u16) = u16::try_from(w) else {
                break;
            };
            let Some(end_x) = x.checked_add(w_u16) else {
                break;
            };
            if end_x > max_x || (x..end_x).any(|col| is_blocked(col, ly, &occupied_label_cells)) {
                break;
            }
            if let Some(cell) = buf.cell_mut((x, ly)) {
                let mut ch_buf = [0u8; 4];
                cell.set_symbol(ch.encode_utf8(&mut ch_buf))
                    .set_style(*edge_style);
            }
            for trail_x in x.saturating_add(1)..end_x {
                if let Some(cell) = buf.cell_mut((trail_x, ly)) {
                    cell.set_symbol("").set_style(*edge_style);
                }
            }
            for col in x..end_x {
                occupied_label_cells.insert((col, ly));
            }
            x = end_x;
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
        let sub_count = child_counts.get(comp.id.as_str()).copied().unwrap_or(0);
        let safe_name = crate::tui::ui::sanitize_display_text(&comp.name);
        let safe_description = crate::tui::ui::sanitize_display_text(&comp.description);
        let line1 = if sub_count > 0 {
            let badge = if app.is_expanded(&comp.id) {
                format!("[-{sub_count}]")
            } else {
                format!("[+{sub_count}]")
            };
            let badge_len = display_width(&badge);
            if inner_w > badge_len + 1 {
                let prefix =
                    truncate_text(&format!("{glyph} {safe_name}"), inner_w - badge_len - 1);
                format!("{prefix} {badge}")
            } else {
                truncate_text(&format!("{glyph} {safe_name} {badge}"), inner_w)
            }
        } else {
            truncate_text(&format!("{glyph} {safe_name}"), inner_w)
        };
        let archetype = archetype_for(comp.kind);
        let mut lines = match viewport.zoom {
            ZoomLevel::Compact => vec![
                Line::from(line1),
                Line::from(truncate_text(
                    &crate::tui::ui::sanitize_display_text(&format!(
                        "Tasks {}/{}",
                        progress.complete, progress.total
                    )),
                    inner_w,
                )),
            ],
            ZoomLevel::Normal => vec![
                Line::from(line1),
                Line::from(truncate_text(
                    &crate::tui::ui::sanitize_display_text(&format!(
                        "Tasks {}/{} [{}]",
                        progress.complete, progress.total, archetype.label
                    )),
                    inner_w,
                )),
                Line::from(truncate_text(&safe_description, inner_w)),
            ],
            ZoomLevel::Expanded => {
                let tags_summary = if comp.tags.is_empty() {
                    "Tags: -".to_string()
                } else {
                    format!(
                        "Tags: {}",
                        crate::tui::ui::sanitize_display_text(&comp.tags.join(", "))
                    )
                };
                vec![
                    Line::from(line1),
                    Line::from(truncate_text(
                        &crate::tui::ui::sanitize_display_text(&format!(
                            "Tasks {}/{} [{}]",
                            progress.complete, progress.total, archetype.label
                        )),
                        inner_w,
                    )),
                    Line::from(truncate_text(&safe_description, inner_w)),
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

    #[test]
    fn canvas_strips_control_chars_from_card_and_edge_labels() {
        let mut app = make_canvas_test_app(
            GridPos { col: 0, row: 0 },
            GridPos { col: 2, row: 0 },
            Some("flow\x1b[31m\x07\u{009b}"),
        );
        app.project.components[0].name = "Alpha\x1b[31m\x07\u{009b}".to_string();
        app.project.components[1].name = "Beta\x1b[31m\x07\u{009b}".to_string();

        let buf = render_canvas_buffer(&app, 80, 24);
        let mut text = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                let sym = buf[(x, y)].symbol();
                assert!(
                    !sym.chars().any(char::is_control),
                    "found control character in canvas cell ({x}, {y}): {sym:?}"
                );
                text.push_str(sym);
            }
            text.push('\n');
        }

        assert!(text.contains("Alpha"));
        assert!(text.contains("Beta"));
        assert!(text.contains("flow"));
    }

    #[test]
    fn canvas_unicode_width_truncation_and_label_placement() {
        let truncated = truncate_text("日本語テスト🚀", 6);
        assert!(
            display_width(&truncated) <= 6,
            "expected display width <= 6, got {} for {truncated:?}",
            display_width(&truncated)
        );
        assert!(truncated.ends_with('…'));
        assert_eq!(truncate_text("日本語テスト🚀", 0), "");
        assert!(display_width(&truncate_text("日本語テスト🚀", 1)) <= 1);

        let mut app = make_canvas_test_app(
            GridPos { col: 0, row: 0 },
            GridPos { col: 1, row: 0 },
            Some("日本語テスト🚀"),
        );
        app.project.components[0].name = "日本語テスト🚀コンポーネント".to_string();
        app.project.components[1].name = "テスト🚀受信".to_string();

        let buf = render_canvas_buffer(&app, 80, 24);
        let inner = Rect::new(1, 1, 78, 22);
        let normal_vp = CanvasViewport {
            zoom: ZoomLevel::Normal,
            ..Default::default()
        };
        let from_rect = card_rect(inner, normal_vp, GridPos { col: 0, row: 0 }).unwrap();
        let to_rect = card_rect(inner, normal_vp, GridPos { col: 1, row: 0 }).unwrap();

        // Selected card (alpha) uses Double borders; unselected card (beta) uses Plain borders.
        assert_eq!(buf[(from_rect.x, from_rect.y)].symbol(), "╔");
        assert_eq!(
            buf[(from_rect.x + from_rect.width - 1, from_rect.y)].symbol(),
            "╗"
        );
        assert_eq!(
            buf[(from_rect.x + from_rect.width - 1, from_rect.y + 1)].symbol(),
            "║"
        );
        assert_eq!(buf[(to_rect.x, to_rect.y)].symbol(), "┌");
        assert_eq!(buf[(to_rect.x, to_rect.y + 1)].symbol(), "│");
        assert_eq!(
            buf[(to_rect.x + to_rect.width - 1, to_rect.y + 1)].symbol(),
            "│"
        );
    }

    #[test]
    fn route_orthogonal_edge_handles_touching_and_overlapping_cards_without_overflow() {
        let bounds = Rect::new(0, 0, u16::MAX, u16::MAX);
        let cases = [
            // Touching horizontally: x: 0..10 and x: 10..20
            (Rect::new(0, 0, 10, 5), Rect::new(10, 0, 10, 5)),
            // Touching vertically: y: 0..5 and y: 5..10
            (Rect::new(0, 0, 10, 5), Rect::new(0, 5, 10, 5)),
            // Overlapping rects
            (Rect::new(0, 0, 10, 5), Rect::new(5, 2, 10, 5)),
            // Rects near u16::MAX
            (
                Rect::new(u16::MAX - 20, u16::MAX - 10, 8, 4),
                Rect::new(u16::MAX - 8, u16::MAX - 4, 8, 4),
            ),
            (
                Rect::new(u16::MAX - 10, u16::MAX - 5, 10, 5),
                Rect::new(u16::MAX - 20, u16::MAX - 5, 10, 5),
            ),
            (
                Rect::new(u16::MAX - 4, u16::MAX - 4, 4, 4),
                Rect::new(u16::MAX - 4, u16::MAX - 4, 4, 4),
            ),
        ];

        for (from_rect, to_rect) in cases {
            let edge = route_orthogonal_edge(from_rect, to_rect, Some("link"), bounds);
            for &(x, y, _) in &edge.segments {
                assert!(
                    !in_bounds(x, y, from_rect),
                    "segment ({x}, {y}) lies inside from_rect {from_rect:?}"
                );
                assert!(
                    !in_bounds(x, y, to_rect),
                    "segment ({x}, {y}) lies inside to_rect {to_rect:?}"
                );
            }
        }
    }

    #[test]
    fn render_canvas_merges_corridor_junctions_and_avoids_label_overlap() {
        use std::path::PathBuf;

        use crate::model::{
            Component, ComponentKind, Connection, GridConfig, Project, ProjectConfig,
        };
        use crate::theme::default_theme;

        let make_comp = |id: &str, name: &str, pos: GridPos| Component {
            id: id.to_string(),
            name: name.to_string(),
            kind: ComponentKind::Logic,
            parent_id: None,
            description: format!("{name} node"),
            relative_path: Some(format!("src/{id}.rs")),
            position: pos,
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
                name: "Corridor Test".to_string(),
                description: "Junction and label overlap test".to_string(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig::default(),
                connections: vec![
                    Connection {
                        from: "left".to_string(),
                        to: "right".to_string(),
                        kind: EdgeKind::DataFlow,
                        label: Some("req".to_string()),
                    },
                    Connection {
                        from: "left".to_string(),
                        to: "right".to_string(),
                        kind: EdgeKind::Invokes,
                        label: Some("ack".to_string()),
                    },
                    Connection {
                        from: "top".to_string(),
                        to: "bottom".to_string(),
                        kind: EdgeKind::DependsOn,
                        label: None,
                    },
                ],
            },
            components: vec![
                make_comp("left", "Left", GridPos { col: 0, row: 1 }),
                make_comp("right", "Right", GridPos { col: 2, row: 1 }),
                make_comp("top", "Top", GridPos { col: 1, row: 0 }),
                make_comp("bottom", "Bottom", GridPos { col: 1, row: 2 }),
            ],
        };

        let theme = default_theme().unwrap();
        let app = App::new(PathBuf::from("/tmp/example-project"), project, theme);
        // "left" is selected (index 0), so left->right is highlighted and top->bottom is unhighlighted.
        let buf = render_canvas_buffer(&app, 95, 25);

        let inner = Rect::new(1, 1, 93, 23);
        let normal_vp = CanvasViewport {
            zoom: ZoomLevel::Normal,
            ..Default::default()
        };
        let left_rect = card_rect(inner, normal_vp, GridPos { col: 0, row: 1 }).unwrap();
        let top_rect = card_rect(inner, normal_vp, GridPos { col: 1, row: 0 }).unwrap();
        let cross_x = card_anchor_points(top_rect).bottom.0;
        let cross_y = card_anchor_points(left_rect).right.1;

        assert_eq!(
            buf[(cross_x, cross_y)].symbol(),
            "┼",
            "expected crossing junction '┼' at ({cross_x}, {cross_y})"
        );
        assert_eq!(
            buf[(cross_x, cross_y)].fg,
            style::to_color(app.theme.palette.accent),
            "expected highlighted foreground style on merged junction cell"
        );

        let full_text: String = (0..buf.area.height)
            .map(|y| row_text(&buf, y))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            full_text.contains("req"),
            "expected first parallel edge label 'req' in canvas:\n{full_text}"
        );
        assert!(
            full_text.contains("ack"),
            "expected second parallel edge label 'ack' in canvas:\n{full_text}"
        );
    }
}
