use ratatui::layout::Rect;

use crate::model::GridPos;

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
}
