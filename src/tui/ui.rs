use std::collections::HashSet;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::archetype::archetype_for;
use crate::audit::{compute_card_status, task_progress};
use crate::theme::Theme;
use crate::tui::app::{App, FocusColumn, ListFilter};
use crate::tui::modal::{
    edge_kind_label, edge_relationship_preview, ConfirmDeleteTarget, ModalState,
};
use crate::tui::style;

pub(crate) fn sanitize_display_text(input: &str) -> String {
    input
        .chars()
        .filter_map(|c| {
            if c == '\t' {
                Some(' ')
            } else if c.is_control() {
                None
            } else {
                Some(c)
            }
        })
        .collect()
}

pub fn column_layout(area: Rect) -> [Rect; 3] {
    let left_w = ((u32::from(area.width) * 19 / 100) as u16)
        .clamp(14, 22)
        .min(area.width / 3);
    let right_w = ((u32::from(area.width) * 25 / 100) as u16)
        .clamp(18, 30)
        .min(area.width / 3);
    let cols = Layout::horizontal([
        Constraint::Length(left_w),
        Constraint::Min(0),
        Constraint::Length(right_w),
    ])
    .split(area);
    [cols[0], cols[1], cols[2]]
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    frame.render_widget(Block::default().style(style::base_style(&app.theme)), area);

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    render_header(frame, chunks[0], app);

    let cols = column_layout(chunks[1]);

    let canvas_inner = Block::default().borders(Borders::ALL).inner(cols[1]);
    let (vis_cols, vis_rows) =
        crate::tui::canvas::visible_grid_capacity_for(canvas_inner, app.viewport.zoom);
    app.last_canvas_capacity = Some((vis_cols, vis_rows));

    let occupied_cols = app
        .project
        .components
        .iter()
        .map(|c| c.position.col.saturating_add(1))
        .max()
        .unwrap_or(0);
    let occupied_rows = app
        .project
        .components
        .iter()
        .map(|c| c.position.row.saturating_add(1))
        .max()
        .unwrap_or(0);
    crate::tui::canvas::clamp_viewport_scroll(
        canvas_inner,
        &mut app.viewport,
        occupied_cols,
        occupied_rows,
    );

    if let Some(selected_pos) = app.selected_component().map(|selected| selected.position) {
        crate::tui::canvas::ensure_visible(canvas_inner, &mut app.viewport, selected_pos);
    }

    render_component_list(frame, cols[0], app);
    crate::tui::canvas::render_canvas(frame, cols[1], app, app.viewport);
    render_detail_panel(frame, cols[2], app);

    render_footer(frame, chunks[2], app);

    if let Some(modal) = &app.modal {
        render_modal(frame, area, modal, &app.theme, &app.project.config.theme);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let gap_count = app.gap_count();
    let gap_style = if gap_count > 0 {
        Style::default()
            .fg(style::to_color(app.theme.palette.alert_error))
            .add_modifier(Modifier::BOLD)
    } else {
        style::base_style(&app.theme)
    };

    let line = Line::from(vec![
        Span::styled(
            format!(
                " ROCKET BENCH :: {}",
                sanitize_display_text(&app.project.config.name)
            ),
            style::header_style(&app.theme),
        ),
        Span::styled(
            format!(" | MODE: {:?} | ", app.mode),
            style::base_style(&app.theme),
        ),
        Span::styled(format!("GAPS: {gap_count}"), gap_style),
        Span::styled(
            format!(" | {}", sanitize_display_text(&app.status_message)),
            style::base_style(&app.theme),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(style::panel_border_style(&app.theme, false))
        .style(style::base_style(&app.theme));

    let header = Paragraph::new(line).block(block);
    frame.render_widget(header, area);
}

fn component_depth(app: &App, comp_id: &str) -> usize {
    let mut depth = 0usize;
    let mut current_id = comp_id;
    let mut visited = HashSet::new();
    visited.insert(current_id);
    while let Some(parent_id) = app
        .project
        .component(current_id)
        .and_then(|c| c.parent_id.as_deref())
    {
        if !visited.insert(parent_id) {
            break;
        }
        depth += 1;
        if app.project.component(parent_id).is_none() {
            break;
        }
        current_id = parent_id;
    }
    depth
}

fn format_component_list_label(
    indent: &str,
    expand_prefix: &str,
    glyph: &str,
    name: &str,
    complete: usize,
    total: usize,
    inner_width: usize,
) -> String {
    let prefix = format!("{indent}{expand_prefix}{glyph} ");
    let suffix = format!(" ({complete}/{total})");
    let prefix_w = Line::from(prefix.as_str()).width();
    let suffix_w = Line::from(suffix.as_str()).width();
    let name_w = Line::from(name).width();

    if prefix_w + name_w + suffix_w <= inner_width {
        return format!("{prefix}{name}{suffix}");
    }

    let avail_name_w = inner_width.saturating_sub(prefix_w.saturating_add(suffix_w));
    let truncated_name = if avail_name_w == 0 {
        String::new()
    } else if avail_name_w == 1 {
        "…".to_string()
    } else {
        let mut out = String::new();
        let target = avail_name_w - 1;
        let mut used = 0usize;
        for ch in name.chars() {
            let ch_w = Line::from(ch.to_string()).width();
            if used + ch_w > target {
                break;
            }
            out.push(ch);
            used += ch_w;
        }
        out.push('…');
        out
    };

    format!("{prefix}{truncated_name}{suffix}")
}

fn render_component_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let title = match app.filter {
        ListFilter::All => " Components ",
        ListFilter::GapsOnly => " Components [GAPS] ",
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(style::panel_border_style(
            &app.theme,
            app.focus == FocusColumn::List,
        ))
        .style(style::base_style(&app.theme));

    let visible = app.visible_component_indices();
    if visible.is_empty() {
        app.list_state.select(None);
        let empty = Paragraph::new(" (no components) ")
            .style(style::muted_style(&app.theme))
            .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let selected = app.selected_index.min(visible.len() - 1);
    app.list_state.select(Some(selected));

    let inner_width = usize::from(area.width.saturating_sub(2));
    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .filter_map(|(row_idx, &comp_idx)| {
            let comp = app.project.components.get(comp_idx)?;
            let status = compute_card_status(comp);
            let glyph = style::status_glyph(status);
            let progress = task_progress(comp);
            let depth = component_depth(app, &comp.id);
            let indent = if depth > 0 {
                format!("{}└─ ", "  ".repeat(depth))
            } else {
                String::new()
            };
            let expand_prefix = if app.has_sub_components(&comp.id) {
                if app.is_expanded(&comp.id) {
                    "▾ "
                } else {
                    "▸ "
                }
            } else {
                ""
            };
            let safe_name = sanitize_display_text(&comp.name);
            let label = format_component_list_label(
                &indent,
                expand_prefix,
                glyph,
                &safe_name,
                progress.complete,
                progress.total,
                inner_width,
            );
            let item_style = if row_idx == selected {
                style::selected_row_style(&app.theme)
            } else {
                Style::default().fg(style::card_border_color(&app.theme, status))
            };
            Some(ListItem::new(label).style(item_style))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .style(style::base_style(&app.theme));
    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_detail_panel(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::default()
        .title(" Component Details ")
        .borders(Borders::ALL)
        .border_style(style::panel_border_style(
            &app.theme,
            app.focus == FocusColumn::Detail,
        ))
        .style(style::base_style(&app.theme));

    let Some(comp) = app.selected_component() else {
        app.last_detail_max_scroll = Some(0);
        app.detail_scroll = 0;
        let empty = Paragraph::new("Select a component to inspect details.")
            .style(style::muted_style(&app.theme))
            .wrap(Wrap { trim: false })
            .block(block);
        frame.render_widget(empty, area);
        return;
    };

    let status = compute_card_status(comp);
    let archetype = archetype_for(comp.kind);
    let path_display = comp
        .relative_path
        .as_deref()
        .map(sanitize_display_text)
        .unwrap_or_else(|| "-".to_string());
    let tags_display = if comp.tags.is_empty() {
        "-".to_string()
    } else {
        sanitize_display_text(&comp.tags.join(", "))
    };

    let heading_style = Style::default()
        .fg(style::to_color(app.theme.palette.accent))
        .add_modifier(Modifier::BOLD);
    let base = style::base_style(&app.theme);
    let muted = style::muted_style(&app.theme);

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            format!(
                "Name: {} ({})",
                sanitize_display_text(&comp.name),
                sanitize_display_text(&comp.id)
            ),
            base.add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(
                "Archetype: {} | Status: {:?} | Grid: ({}, {})",
                archetype.label, status, comp.position.col, comp.position.row
            ),
            Style::default().fg(style::card_border_color(&app.theme, status)),
        )),
    ];
    if let Some(pid) = comp.parent_id.as_deref() {
        lines.push(Line::from(Span::styled(
            format!("Parent: {}", sanitize_display_text(pid)),
            base,
        )));
    }
    lines.push(Line::from(Span::styled(
        format!("Path: {path_display}"),
        base,
    )));
    lines.push(Line::from(Span::styled(
        format!("Tags: {tags_display}"),
        base,
    )));
    lines.push(Line::from(Span::styled(
        format!("Description: {}", sanitize_display_text(&comp.description)),
        base,
    )));

    let sub_components = app.project.sub_components(&comp.id);
    lines.push(Line::from(""));
    if sub_components.is_empty() {
        lines.push(Line::from(Span::styled("Sub-components:", heading_style)));
        lines.push(Line::from(Span::styled("  (none)", muted)));
    } else {
        let sub_state = if app.is_expanded(&comp.id) {
            "[expanded]"
        } else {
            "[collapsed]"
        };
        lines.push(Line::from(Span::styled(
            format!("Sub-components: {sub_state}"),
            heading_style,
        )));
        for child in sub_components {
            lines.push(Line::from(Span::styled(
                format!(
                    "  {} ({})",
                    sanitize_display_text(&child.name),
                    sanitize_display_text(&child.id)
                ),
                base,
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Connections:", heading_style)));

    let mut has_connections = false;
    for conn in &app.project.config.connections {
        if conn.from == comp.id {
            has_connections = true;
            lines.push(Line::from(Span::styled(
                format!("  -> {} [{:?}]", sanitize_display_text(&conn.to), conn.kind),
                base,
            )));
        }
        if conn.to == comp.id {
            has_connections = true;
            lines.push(Line::from(Span::styled(
                format!(
                    "  <- {} [{:?}]",
                    sanitize_display_text(&conn.from),
                    conn.kind
                ),
                base,
            )));
        }
    }
    if !has_connections {
        lines.push(Line::from(Span::styled("  (none)", muted)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Features:", heading_style)));
    if comp.features.is_empty() {
        lines.push(Line::from(Span::styled("  (none)", muted)));
    } else {
        for feature in &comp.features {
            let state = if feature.pushed {
                "[pushed]"
            } else {
                "[local]"
            };
            lines.push(Line::from(Span::styled(
                format!("  {state} {}", sanitize_display_text(&feature.title)),
                base,
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("External Tasks:", heading_style)));
    if comp.external_tasks.is_empty() {
        lines.push(Line::from(Span::styled("  (none)", muted)));
    } else {
        for task in &comp.external_tasks {
            lines.push(Line::from(Span::styled(
                format!(
                    "  [{:?}] {} {}",
                    task.status,
                    sanitize_display_text(&task.external_id),
                    sanitize_display_text(&task.title)
                ),
                base,
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Specification:", heading_style)));
    if comp.markdown_body.is_empty() {
        lines.push(Line::from(Span::styled("  (none)", muted)));
    } else {
        for md_line in comp.markdown_body.lines() {
            let clean_line = sanitize_display_text(md_line);
            let line_style = style::markdown_line_style(&app.theme, &clean_line);
            lines.push(Line::from(Span::styled(clean_line, line_style)));
        }
    }

    let inner_width = area.width.saturating_sub(2);
    let inner_height = usize::from(area.height.saturating_sub(2));
    let total_lines = usize::from(wrapped_line_count(&lines, inner_width));
    let max_scroll = u16::try_from(total_lines.saturating_sub(inner_height)).unwrap_or(u16::MAX);
    app.last_detail_max_scroll = Some(max_scroll);
    app.detail_scroll = app.detail_scroll.min(max_scroll);

    let detail = Paragraph::new(lines)
        .style(base)
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0))
        .block(block);
    frame.render_widget(detail, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let label_style = style::muted_style(&app.theme);
    let key_style = Style::default()
        .fg(style::to_color(app.theme.palette.accent))
        .add_modifier(Modifier::UNDERLINED);

    let core_items: &[&[(&str, bool)]] = &[
        &[("Tab", true), (" focus", false)],
        &[("n", true), ("ew", false)],
        &[("e", true), ("dit", false)],
        &[("m", true), ("ove", false)],
        &[("c", true), ("onn", false)],
        &[("f", true), ("ilter", false)],
        &[("+/-", true), (" zoom", false)],
        &[("?", true), (" help", false)],
        &[("q", true), ("uit", false)],
    ];
    let extended_items: &[&[(&str, bool)]] = &[
        &[("a", true), ("dd sub", false)],
        &[("E", true), ("dit edge", false)],
        &[("v", true), ("iew MD", false)],
        &[("r", true), ("/", false), ("S", true), (" sync", false)],
        &[("p", true), ("ush", false)],
        &[("t", true), ("heme", false)],
    ];

    let max_width = usize::from(area.width);
    let mut spans: Vec<Span<'static>> = vec![Span::styled(" ", label_style)];
    let mut used_width: usize = 1;

    for (idx, item) in core_items.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled("  ", label_style));
            used_width += 2;
        }
        for &(text, is_key) in *item {
            let st = if is_key { key_style } else { label_style };
            spans.push(Span::styled(text, st));
            used_width += text.chars().count();
        }
    }

    for item in extended_items {
        let item_width: usize = item.iter().map(|(text, _)| text.chars().count()).sum();
        if used_width + 2 + item_width > max_width {
            break;
        }
        spans.push(Span::styled("  ", label_style));
        used_width += 2;
        for &(text, is_key) in *item {
            let st = if is_key { key_style } else { label_style };
            spans.push(Span::styled(text, st));
        }
        used_width += item_width;
    }

    let footer = Paragraph::new(Line::from(spans)).style(label_style);
    frame.render_widget(footer, area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x.saturating_add(area.width.saturating_sub(w) / 2);
    let y = area.y.saturating_add(area.height.saturating_sub(h) / 2);
    Rect::new(x, y, w, h)
}

fn wrapped_line_count(lines: &[Line<'_>], inner_width: u16) -> u16 {
    let max_w = usize::from(inner_width);
    if max_w == 0 {
        return u16::try_from(lines.len()).unwrap_or(u16::MAX);
    }
    let mut total: usize = 0;
    for line in lines {
        let line_w = line.width();
        if line_w <= max_w {
            total = total.saturating_add(1);
            continue;
        }
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        let mut cur_w = 0usize;
        let mut rows = 1usize;
        for segment in text.split_inclusive(char::is_whitespace) {
            let seg_w = Line::from(segment).width();
            if cur_w == 0 {
                if seg_w <= max_w {
                    cur_w = seg_w;
                } else {
                    rows = rows.saturating_add(seg_w.saturating_sub(1) / max_w);
                    cur_w = ((seg_w - 1) % max_w) + 1;
                }
            } else if cur_w + seg_w <= max_w {
                cur_w += seg_w;
            } else {
                rows = rows.saturating_add(1);
                if seg_w <= max_w {
                    cur_w = seg_w;
                } else {
                    rows = rows.saturating_add(seg_w.saturating_sub(1) / max_w);
                    cur_w = ((seg_w - 1) % max_w) + 1;
                }
            }
        }
        total = total.saturating_add(rows);
    }
    u16::try_from(total).unwrap_or(u16::MAX)
}

fn truncate_text(text: &str, max_width: usize) -> String {
    if Line::from(text).width() <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return "…".to_string();
    }
    let target = max_width - 1;
    let mut out = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let ch_w = Line::from(ch.to_string()).width();
        if used + ch_w > target {
            break;
        }
        out.push(ch);
        used += ch_w;
    }
    out.push('…');
    out
}

fn modal_field_line(label: &str, value: &str, active: bool, theme: &Theme) -> Line<'static> {
    let prefix = if active { "> " } else { "  " };
    let field_style = if active {
        style::selected_row_style(theme)
    } else {
        style::base_style(theme)
    };
    let safe_value = sanitize_display_text(value);
    Line::from(Span::styled(
        format!("{prefix}{label}: {safe_value}"),
        field_style,
    ))
}

fn render_modal(
    frame: &mut Frame,
    area: Rect,
    modal: &ModalState,
    theme: &Theme,
    current_theme: &str,
) {
    let base = style::base_style(theme);
    let muted = style::muted_style(theme);
    let heading_style = Style::default()
        .fg(style::to_color(theme.palette.accent))
        .add_modifier(Modifier::BOLD);
    let error_style = Style::default()
        .fg(style::to_color(theme.palette.alert_error))
        .add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line> = Vec::new();
    let error_text = match modal {
        ModalState::NewComponent(form) => {
            if let Some(pid) = &form.parent_id {
                lines.push(Line::from(Span::styled(
                    format!("  Parent: {}", sanitize_display_text(pid)),
                    muted,
                )));
            }
            lines.push(modal_field_line(
                "ID",
                &form.id,
                form.active_field == 0,
                theme,
            ));
            lines.push(modal_field_line(
                "Name",
                &form.name,
                form.active_field == 1,
                theme,
            ));
            lines.push(modal_field_line(
                "Archetype",
                &format!("< {} >", form.selected_archetype().label),
                form.active_field == 2,
                theme,
            ));
            form.error.as_deref()
        }
        ModalState::EditComponent(form) => {
            lines.push(Line::from(Span::styled(
                format!("  ID: {}", sanitize_display_text(&form.component_id)),
                muted,
            )));
            lines.push(modal_field_line(
                "Name",
                &form.name,
                form.active_field == 0,
                theme,
            ));
            lines.push(modal_field_line(
                "Archetype",
                &format!("< {} >", form.selected_archetype().label),
                form.active_field == 1,
                theme,
            ));
            lines.push(modal_field_line(
                "Description",
                &form.description,
                form.active_field == 2,
                theme,
            ));
            lines.push(modal_field_line(
                "Path",
                &form.relative_path,
                form.active_field == 3,
                theme,
            ));
            lines.push(modal_field_line(
                "Tags",
                &form.tags,
                form.active_field == 4,
                theme,
            ));
            lines.push(modal_field_line(
                "Add Feature",
                &form.new_feature,
                form.active_field == 5,
                theme,
            ));
            lines.push(modal_field_line(
                "Add Sub-component",
                &form.new_sub_component,
                form.active_field == 6,
                theme,
            ));
            form.error.as_deref()
        }
        ModalState::EditConnection(form) => {
            let max_text_w = usize::from(area.width.saturating_sub(4)).clamp(20, 74);
            if form.candidates.len() > 1 {
                let counter = format!(
                    "({}/{})",
                    form.selected_candidate + 1,
                    form.candidates.len()
                );
                lines.push(Line::from(Span::styled(
                    format!(
                        "  Connection {counter} [{}]",
                        edge_kind_label(form.original_kind)
                    ),
                    heading_style,
                )));
            }
            let preview = sanitize_display_text(&edge_relationship_preview(
                &form.from,
                &form.to,
                form.selected_kind(),
            ));
            let edge_prefix = "  Edge: ";
            let truncated_preview = truncate_text(
                &preview,
                max_text_w.saturating_sub(edge_prefix.chars().count()),
            );
            lines.push(Line::from(Span::styled(
                format!("{edge_prefix}{truncated_preview}"),
                muted,
            )));
            lines.push(modal_field_line(
                "Kind",
                &format!("< {} >", edge_kind_label(form.selected_kind())),
                form.active_field == 0,
                theme,
            ));
            let label_prefix_w = "> Label: ".chars().count();
            let truncated_label = truncate_text(
                &sanitize_display_text(&form.label),
                max_text_w.saturating_sub(label_prefix_w),
            );
            lines.push(modal_field_line(
                "Label",
                &truncated_label,
                form.active_field == 1,
                theme,
            ));
            form.error.as_deref()
        }
        ModalState::Help => {
            type HelpRows = &'static [(&'static str, bool)];
            let (left_col_width, left_rows, right_rows): (usize, HelpRows, HelpRows) =
                if area.width < 80 {
                    (
                        28,
                        &[
                            (" Navigation", true),
                            (" Tab/S-Tab  Cycle focus", false),
                            (" hjkl/Arr   Move / Scroll", false),
                            (" f          Gaps Only filter", false),
                            (" Spc/Enter  Expand/collapse", false),
                            (" Editing & Canvas", true),
                            (" n          New component", false),
                            (" a          Add sub-comp", false),
                            (" e          Edit component", false),
                            (" E          Edit edge", false),
                            (" d          Delete component", false),
                            (" D          Delete edge", false),
                            (" m          Move card mode", false),
                            (" c          Connect (A depends on B)", false),
                            (" v          Edit Markdown", false),
                        ],
                        &[
                            (" Zoom", true),
                            (" + / =  Zoom in", false),
                            (" -      Zoom out", false),
                            (" Sync & Display", true),
                            (" r / S  Pull adapter status", false),
                            (" p      Push selected", false),
                            (" t      Switch RON theme", false),
                            (" General", true),
                            (" ?      Toggle help", false),
                            (" Esc    Cancel mode", false),
                            (" Enter  Confirm connect", false),
                            (" q      Quit", false),
                            (" Ctrl+C Force quit", false),
                        ],
                    )
                } else {
                    (
                        34,
                        &[
                            (" Navigation", true),
                            (" Tab / Shift+Tab  Cycle focus", false),
                            (" hjkl / Arrows    Move / Scroll", false),
                            (" f                Gaps Only filter", false),
                            (" Space / Enter    Expand/collapse", false),
                            (" Editing & Canvas", true),
                            (" n                New component", false),
                            (" a                Add sub-comp", false),
                            (" e                Edit component", false),
                            (" E                Edit edge", false),
                            (" d                Delete component", false),
                            (" D                Delete edge", false),
                            (" m                Move card mode", false),
                            (" c                Connect (A depends on B)", false),
                            (" v                Edit Markdown", false),
                        ],
                        &[
                            (" Zoom", true),
                            (" + / =  Zoom in", false),
                            (" -      Zoom out", false),
                            (" Sync & Display", true),
                            (" r / S  Pull status from active adapters", false),
                            (" p      Push selected component", false),
                            (" t      Switch RON theme", false),
                            (" General", true),
                            (" ?      Toggle this keyboard help", false),
                            (" Esc    Cancel current mode", false),
                            (" Enter  Confirm connect target", false),
                            (" q      Quit", false),
                            (" Ctrl+C Force quit", false),
                        ],
                    )
                };

            let total_rows = left_rows.len().max(right_rows.len());
            for row_idx in 0..total_rows {
                let left = left_rows.get(row_idx).copied();
                let right = right_rows.get(row_idx).copied();
                match (left, right) {
                    (Some((l_text, l_heading)), Some((r_text, r_heading))) => {
                        let l_style = if l_heading { heading_style } else { base };
                        let r_style = if r_heading { heading_style } else { base };
                        lines.push(Line::from(vec![
                            Span::styled(format!("{l_text:<left_col_width$}"), l_style),
                            Span::styled("  ", base),
                            Span::styled(r_text, r_style),
                        ]));
                    }
                    (Some((l_text, l_heading)), None) => {
                        let l_style = if l_heading { heading_style } else { base };
                        lines.push(Line::from(Span::styled(l_text, l_style)));
                    }
                    (None, Some((r_text, r_heading))) => {
                        let r_style = if r_heading { heading_style } else { base };
                        lines.push(Line::from(vec![
                            Span::styled(" ".repeat(left_col_width + 2), base),
                            Span::styled(r_text, r_style),
                        ]));
                    }
                    (None, None) => {}
                }
            }
            None
        }
        ModalState::ThemeSwitcher(form) => {
            if form.themes.is_empty() {
                lines.push(Line::from(Span::styled("  (no themes found)", muted)));
            } else {
                for (idx, theme_name) in form.themes.iter().enumerate() {
                    let is_selected = idx == form.selected_index;
                    let is_active = theme_name == current_theme;
                    let prefix = if is_selected { "> " } else { "  " };
                    let suffix = if is_active { " [active]" } else { "" };
                    let row_style = if is_selected {
                        style::selected_row_style(theme)
                    } else {
                        base
                    };
                    lines.push(Line::from(Span::styled(
                        format!("{prefix}{}{suffix}", sanitize_display_text(theme_name)),
                        row_style,
                    )));
                }
            }
            form.error.as_deref()
        }
        ModalState::ConfirmDelete(form) => {
            match &form.target {
                ConfirmDeleteTarget::Component {
                    id,
                    name,
                    affected_connections,
                } => {
                    lines.push(Line::from(Span::styled(
                        format!(
                            "  Delete component '{}' ({})?",
                            sanitize_display_text(name),
                            sanitize_display_text(id)
                        ),
                        base.add_modifier(Modifier::BOLD),
                    )));
                    if *affected_connections > 0 {
                        let noun = if *affected_connections == 1 {
                            "connection"
                        } else {
                            "connections"
                        };
                        lines.push(Line::from(Span::styled(
                            format!(
                                "  Warning: {affected_connections} {noun} will also be removed."
                            ),
                            error_style,
                        )));
                    }
                }
                ConfirmDeleteTarget::Connection {
                    from,
                    to,
                    kind,
                    candidates,
                    selected_candidate,
                } => {
                    let max_text_w = usize::from(area.width.saturating_sub(4)).clamp(20, 74);
                    if candidates.len() > 1 {
                        let counter = format!("({}/{})", selected_candidate + 1, candidates.len());
                        let kind_tag = kind
                            .map(|k| format!(" [{}]", edge_kind_label(k)))
                            .unwrap_or_default();
                        lines.push(Line::from(Span::styled(
                            format!("  Connection {counter}{kind_tag}"),
                            heading_style,
                        )));
                    }
                    let prompt = format!(
                        "Delete connection '{} -> {}'?",
                        sanitize_display_text(from),
                        sanitize_display_text(to)
                    );
                    let truncated_prompt = truncate_text(&prompt, max_text_w.saturating_sub(2));
                    lines.push(Line::from(Span::styled(
                        format!("  {truncated_prompt}"),
                        base.add_modifier(Modifier::BOLD),
                    )));
                }
            }
            form.error.as_deref()
        }
    };

    if let Some(err) = error_text {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  Error: {}", sanitize_display_text(err)),
            error_style,
        )));
    }

    let footer_hint = match modal {
        ModalState::Help => "  [Esc/?/Enter/q] Close",
        ModalState::ThemeSwitcher(_) => "  [↑/↓] Select  [Enter] Apply  [Esc] Cancel",
        ModalState::ConfirmDelete(form) => match &form.target {
            ConfirmDeleteTarget::Connection { candidates, .. } if candidates.len() > 1 => {
                "  [↑/↓] Select  [Enter/y] Confirm Delete  [Esc/n] Cancel"
            }
            _ => "  [Enter/y] Confirm Delete  [Esc/n] Cancel",
        },
        ModalState::EditConnection(form) if form.candidates.len() > 1 => {
            "  [[/]] Edge  [Tab] Field  [←/→] Kind  [Enter] Save  [Esc] Cancel"
        }
        ModalState::NewComponent(_)
        | ModalState::EditComponent(_)
        | ModalState::EditConnection(_) => "  [Tab] Field  [←/→] Cycle  [Enter] Save  [Esc] Cancel",
    };

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(footer_hint, muted)));

    let max_line_width = lines.iter().map(Line::width).max().unwrap_or(0);
    let content_width = u16::try_from(max_line_width.saturating_add(4)).unwrap_or(u16::MAX);
    let max_popup_width = if area.width > 2 {
        area.width.saturating_sub(2)
    } else {
        area.width
    };
    let popup_width = ((u32::from(area.width) * 60 / 100) as u16)
        .max(content_width)
        .clamp(54, 78)
        .min(max_popup_width);
    let popup_inner_width = popup_width.saturating_sub(2);
    let popup_height = wrapped_line_count(&lines, popup_inner_width)
        .saturating_add(2)
        .min(area.height);
    let popup_area = centered_rect(popup_width, popup_height, area);

    let block = Block::default()
        .title(format!(" {} ", modal.title()))
        .borders(Borders::ALL)
        .border_style(style::panel_border_style(theme, true))
        .style(base);

    let content = Paragraph::new(lines)
        .style(base)
        .wrap(Wrap { trim: false })
        .block(block);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(content, popup_area);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;
    use crate::model::{
        Component, ComponentKind, ExternalTaskRef, GridConfig, GridPos, Project, ProjectConfig,
        TaskStatus,
    };
    use crate::theme::default_theme;

    fn make_test_app() -> App {
        let view_box = Component {
            id: "view_box".to_string(),
            name: "view_box".to_string(),
            kind: ComponentKind::UiView,
            parent_id: None,
            description: "Primary viewport".to_string(),
            relative_path: Some("src/view_box.rs".to_string()),
            position: GridPos { col: 0, row: 0 },
            tags: vec!["ui".to_string()],
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: "# Overview\nRender viewport".to_string(),
        };

        let core_bus = Component {
            id: "core_bus".to_string(),
            name: "core_bus".to_string(),
            kind: ComponentKind::ServiceCore,
            parent_id: None,
            description: "Event bus".to_string(),
            relative_path: Some("src/core_bus.rs".to_string()),
            position: GridPos { col: 1, row: 0 },
            tags: vec!["core".to_string()],
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: vec![ExternalTaskRef {
                external_id: "TASK-1".to_string(),
                title: "Implement bus".to_string(),
                status: TaskStatus::Complete,
            }],
            markdown_body: String::new(),
        };

        let project = Project {
            config: ProjectConfig {
                name: "Example Project".to_string(),
                description: "UI layout test".to_string(),
                theme: "crt_amber".to_string(),
                active_adapters: Vec::new(),
                grid: GridConfig::default(),
                connections: Vec::new(),
            },
            components: vec![view_box, core_bus],
        };

        let theme = default_theme().unwrap();
        App::new(PathBuf::from("/tmp/example-project"), project, theme)
    }

    fn render_to_string(app: &mut App) -> String {
        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, app)).unwrap();
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn render_three_column_layout_displays_header_list_and_details() {
        let mut app = make_test_app();
        let text = render_to_string(&mut app);

        assert!(text.contains("ROCKET BENCH"));
        assert!(text.contains("GAPS: 1"));
        assert!(text.contains("Components"));
        assert!(text.contains("Snap Grid"));
        assert!(text.contains("Component Details"));
        assert!(text.contains("view_box"));
        assert!(text.contains("view MD"));
        assert!(!text.contains("[v] Edit MD"));
    }

    #[test]
    fn render_gaps_filter_hides_tracked_component() {
        let mut app = make_test_app();
        app.toggle_gaps_filter();
        let text = render_to_string(&mut app);

        assert!(text.contains("Components [GAPS]"));
        assert!(text.contains("view_box"));
        assert!(!text.contains("core_bus"));
    }

    #[test]
    fn render_canvas_column_displays_connected_cards_progress_and_arrow() {
        use crate::model::{Connection, EdgeKind};

        let mut app = make_test_app();
        app.project.config.connections.push(Connection {
            from: "view_box".to_string(),
            to: "core_bus".to_string(),
            kind: EdgeKind::DependsOn,
            label: None,
        });

        let text = render_to_string(&mut app);

        assert!(text.contains("view_box"));
        assert!(text.contains("core_bus"));
        assert!(text.contains("Tasks 0/0"));
        assert!(text.contains("Tasks 1/1"));
        assert!(text.contains('▶'));
    }

    #[test]
    fn render_modal_overlay_displays_title_fields_error_and_hints() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_test_app();
        app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        // Trigger validation error on empty submission
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        let text = render_to_string(&mut app);

        assert!(text.contains("New Component"));
        assert!(text.contains("ID:"));
        assert!(text.contains("Name:"));
        assert!(text.contains("Archetype:"));
        assert!(text.contains("Error:"));
        assert!(text.contains("Tab"));
        assert!(text.contains("←/→"));
        assert!(text.contains("Enter"));
        assert!(text.contains("Esc"));
    }

    #[test]
    fn render_pans_viewport_to_offscreen_selected_component() {
        let mut app = make_test_app();
        app.project.components.push(Component {
            id: "far_node".to_string(),
            name: "Far Outpost".to_string(),
            kind: ComponentKind::ServiceCore,
            parent_id: None,
            description: "Remote grid card".to_string(),
            relative_path: Some("src/far_node.rs".to_string()),
            position: GridPos { col: 4, row: 2 },
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        });
        app.selected_index = 2;
        assert_eq!(app.viewport.scroll_col, 0);
        assert_eq!(app.viewport.scroll_row, 0);

        let text = render_to_string(&mut app);

        assert!(app.viewport.scroll_col > 0 || app.viewport.scroll_row > 0);
        assert!(text.contains("Far Outpost"));
    }

    #[test]
    fn render_modal_handles_wide_terminal_without_overflow() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_test_app();
        app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));

        let backend = TestBackend::new(1200, 35);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
    }

    #[test]
    fn render_help_and_theme_switcher_modals_display_entries_and_footer_hints() {
        let mut app = make_test_app();
        let base_text = render_to_string(&mut app);
        assert!(base_text.contains("view MD"));
        assert!(base_text.contains("r/S sync"));
        assert!(base_text.contains("push"));
        assert!(base_text.contains("theme"));
        assert!(base_text.contains("? help"));
        assert!(!base_text.contains("[v] Edit MD"));
        assert!(!base_text.contains("[r/S] Sync"));

        // Help modal
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        let help_text = render_to_string(&mut app);
        assert!(help_text.contains("Keyboard Help"));
        assert!(help_text.contains("Tab / Shift+Tab"));
        assert!(help_text.contains("hjkl / Arrows"));
        assert!(help_text.contains("Switch RON theme"));
        assert!(help_text.contains("Pull status from active adapters"));
        assert!(help_text.contains("Push selected component"));

        // Theme switcher modal
        app.modal = Some(ModalState::theme_switcher(
            vec!["amber_alt".to_string(), "crt_amber".to_string()],
            "crt_amber",
        ));
        let theme_text = render_to_string(&mut app);
        assert!(theme_text.contains("Switch Theme"));
        assert!(theme_text.contains("amber_alt"));
        assert!(theme_text.contains("crt_amber [active]"));
        assert!(theme_text.contains("[↑/↓] Select  [Enter] Apply  [Esc] Cancel"));
    }

    #[test]
    fn render_footer_uses_real_case_underlined_mnemonics_and_fits_80_cols() {
        let mut app = make_test_app();

        // 120x35: verify all core + extended hotkeys are underlined in their real case
        let backend_wide = TestBackend::new(120, 35);
        let mut terminal_wide = Terminal::new(backend_wide).unwrap();
        terminal_wide.draw(|frame| render(frame, &mut app)).unwrap();
        let buf_wide = terminal_wide.backend().buffer();
        let footer_y = buf_wide.area.height - 1;
        let footer_row: String = (0..buf_wide.area.width)
            .map(|x| buf_wide[(x, footer_y)].symbol())
            .collect();
        let underlined_chars: String = (0..buf_wide.area.width)
            .filter(|&x| {
                buf_wide[(x, footer_y)]
                    .modifier
                    .contains(Modifier::UNDERLINED)
            })
            .map(|x| buf_wide[(x, footer_y)].symbol())
            .collect();

        assert!(!footer_row.contains('['));
        assert!(!footer_row.contains(']'));
        assert!(footer_row.contains("Tab focus"));
        assert!(footer_row.contains("Edit edge"));
        assert!(footer_row.contains("r/S sync"));
        assert!(underlined_chars.contains("Tab"));
        assert!(underlined_chars.contains('n'));
        assert!(underlined_chars.contains('e'));
        assert!(underlined_chars.contains('E'));
        assert!(underlined_chars.contains('r'));
        assert!(underlined_chars.contains('S'));
        assert!(underlined_chars.contains('q'));

        // 80x24: verify core hints fit without clipping
        let backend_80 = TestBackend::new(80, 24);
        let mut terminal_80 = Terminal::new(backend_80).unwrap();
        terminal_80.draw(|frame| render(frame, &mut app)).unwrap();
        let buf_80 = terminal_80.backend().buffer();
        let footer_y_80 = buf_80.area.height - 1;
        let footer_row_80: String = (0..buf_80.area.width)
            .map(|x| buf_80[(x, footer_y_80)].symbol())
            .collect();
        assert!(footer_row_80.contains("Tab focus"));
        assert!(footer_row_80.contains("new"));
        assert!(footer_row_80.contains("edit"));
        assert!(footer_row_80.contains("? help"));
        assert!(footer_row_80.contains("quit"));
    }

    #[test]
    fn render_help_modal_fits_80x24_without_wrapping_or_footer_clipping() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_test_app();
        app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }

        assert!(text.contains("Keyboard Help"));
        assert!(text.contains("New component"));
        assert!(text.contains("Add sub-comp"));
        assert!(text.contains("Edit component"));
        assert!(text.contains("Edit edge"));
        assert!(text.contains("Delete component"));
        assert!(text.contains("Delete edge"));
        assert!(text.contains("Zoom in"));
        assert!(text.contains("Zoom out"));
        assert!(text.contains("Expand/collapse"));
        assert!(text.contains("Confirm connect target"));
        assert!(text.contains("Force quit"));
        assert!(text.contains("[Esc/?/Enter/q] Close"));
        assert!(!text.contains("New component / Edit component / Edit edge"));
    }

    #[test]
    fn render_confirm_delete_modals_display_component_warning_and_connection_prompt() {
        let mut app = make_test_app();

        // Component target with affected connections
        app.modal = Some(ModalState::confirm_delete_component(
            "view_box", "View Box", 2,
        ));
        let comp_text = render_to_string(&mut app);
        assert!(comp_text.contains("Delete Component"));
        assert!(comp_text.contains("Delete component 'View Box' (view_box)?"));
        assert!(comp_text.contains("2 connections will also be removed"));
        assert!(comp_text.contains("[Enter/y] Confirm Delete  [Esc/n] Cancel"));

        // Connection target
        app.modal = Some(ModalState::confirm_delete_connection(
            "view_box", "core_bus",
        ));
        let conn_text = render_to_string(&mut app);
        assert!(conn_text.contains("Delete Connection"));
        assert!(conn_text.contains("Delete connection 'view_box -> core_bus'?"));
        assert!(conn_text.contains("[Enter/y] Confirm Delete  [Esc/n] Cancel"));
    }

    #[test]
    fn render_edit_component_sub_component_and_edge_preview_modals() {
        use crate::model::EdgeKind;

        let mut app = make_test_app();

        // New Sub-Component modal renders parent header
        app.modal = Some(ModalState::new_sub_component("core_bus"));
        let sub_text = render_to_string(&mut app);
        assert!(sub_text.contains("New Sub-Component"));
        assert!(sub_text.contains("Parent: core_bus"));

        // Edit Component modal renders Archetype and Add Sub-component fields
        let comp = app.project.components[0].clone();
        app.modal = Some(ModalState::edit_component(&comp));
        let edit_text = render_to_string(&mut app);
        assert!(edit_text.contains("Edit Component"));
        assert!(edit_text.contains("Archetype: < Interface >"));
        assert!(edit_text.contains("Add Sub-component:"));

        // Edit Connection modal renders human-readable relationship preview
        app.modal = Some(ModalState::edit_connection(
            "view_box",
            "core_bus",
            EdgeKind::DependsOn,
            None,
        ));
        let edge_text = render_to_string(&mut app);
        assert!(edge_text.contains("Edit Connection"));
        assert!(edge_text.contains("Edge: view_box depends on core_bus"));
    }

    #[test]
    fn render_component_list_shows_expand_collapse_indicators_and_nested_indentation() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_test_app();
        app.handle_key(KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE));
        app.project.components.push(Component {
            id: "bus_worker".to_string(),
            name: "bus_worker".to_string(),
            kind: ComponentKind::Logic,
            parent_id: Some("core_bus".to_string()),
            description: "Worker sub-component".to_string(),
            relative_path: Some("src/bus_worker.rs".to_string()),
            position: GridPos { col: 1, row: 1 },
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        });
        app.project.components.push(Component {
            id: "worker_pool".to_string(),
            name: "worker_pool".to_string(),
            kind: ComponentKind::Data,
            parent_id: Some("bus_worker".to_string()),
            description: "Nested pool".to_string(),
            relative_path: Some("src/worker_pool.rs".to_string()),
            position: GridPos { col: 1, row: 2 },
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        });

        let collapsed = render_to_string(&mut app);
        assert!(collapsed.contains("▸ "));
        assert!(!collapsed.contains("  └─ "));

        app.expanded_components.insert("core_bus".to_string());
        let expanded_one = render_to_string(&mut app);
        assert!(expanded_one.contains("▾ "));
        assert!(expanded_one.contains("  └─ ▸ "));
        assert!(expanded_one.contains("bus_worker"));
        assert!(!expanded_one.contains("    └─ "));

        app.expanded_components.insert("bus_worker".to_string());
        let expanded_two = render_to_string(&mut app);
        assert!(expanded_two.contains("  └─ ▾ "));
        assert!(expanded_two.contains("    └─ "));
        assert!(expanded_two.contains("worker_pool"));
    }

    #[test]
    fn render_detail_panel_displays_parent_archetype_and_sub_components() {
        let mut app = make_test_app();
        app.project.components.push(Component {
            id: "bus_worker".to_string(),
            name: "Bus Worker".to_string(),
            kind: ComponentKind::Data,
            parent_id: Some("core_bus".to_string()),
            description: "Worker sub-component".to_string(),
            relative_path: Some("src/bus_worker.rs".to_string()),
            position: GridPos { col: 1, row: 1 },
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        });

        // Select parent (core_bus) while collapsed
        app.selected_index = 1;
        let parent_collapsed = render_to_string(&mut app);
        assert!(parent_collapsed.contains("Archetype: Logic"));
        assert!(!parent_collapsed.contains("Parent:"));
        assert!(parent_collapsed.contains("Sub-components: [collapsed]"));
        assert!(parent_collapsed.contains("Bus Worker (bus_worker)"));

        // Expand parent and verify status updates
        app.expanded_components.insert("core_bus".to_string());
        let parent_expanded = render_to_string(&mut app);
        assert!(parent_expanded.contains("Sub-components: [expanded]"));
        assert!(parent_expanded.contains("Bus Worker (bus_worker)"));

        // Select child (bus_worker) and verify Parent, Archetype, and empty Sub-components
        app.selected_index = 2;
        let child_detail = render_to_string(&mut app);
        assert!(child_detail.contains("Archetype: Data"));
        assert!(child_detail.contains("Parent: core_bus"));
        assert!(child_detail.contains("Sub-components:"));
        assert!(child_detail.contains("(none)"));
    }

    #[test]
    fn column_layout_widens_canvas_and_renders_4x4_compact_grid() {
        use crate::tui::canvas::ZoomLevel;

        let cols = column_layout(Rect::new(0, 0, 140, 40));
        assert!(
            cols[1].width * 100 >= 140 * 60,
            "expected middle column >=60% on 140-col terminal, got {}",
            cols[1].width
        );

        let mut app = make_test_app();
        app.project.config.grid = GridConfig { cols: 2, rows: 2 };
        app.project.components.clear();
        for row in 0..4u16 {
            for col in 0..4u16 {
                let idx = row * 4 + col;
                app.project.components.push(Component {
                    id: format!("n{idx:02}"),
                    name: format!("N{idx:02}"),
                    kind: ComponentKind::Logic,
                    parent_id: None,
                    description: "Grid card".to_string(),
                    relative_path: Some(format!("src/n{idx:02}.rs")),
                    position: GridPos { col, row },
                    tags: Vec::new(),
                    tech_stack: Vec::new(),
                    files: Vec::new(),
                    flags: Vec::new(),
                    features: Vec::new(),
                    external_tasks: Vec::new(),
                    markdown_body: String::new(),
                });
            }
        }

        let backend = TestBackend::new(140, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();

        assert_eq!(app.viewport.zoom, ZoomLevel::Compact);
        // Saved project grid config must not be mutated during render.
        assert_eq!(app.project.config.grid, GridConfig { cols: 2, rows: 2 });
        let (vis_cols, vis_rows) = app
            .last_canvas_capacity
            .expect("render updates last_canvas_capacity");
        assert!(vis_cols >= 4 && vis_rows >= 4);

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        assert!(text.contains("[Compact]"));
        for idx in 0..16u16 {
            assert!(
                text.contains(&format!("N{idx:02}")),
                "expected card N{idx:02} in 4x4 compact canvas"
            );
        }
    }

    #[test]
    fn render_help_modal_shows_close_hint_at_68_and_79_cols() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        for width in [68u16, 79u16] {
            let mut app = make_test_app();
            app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));

            let backend = TestBackend::new(width, 24);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|frame| render(frame, &mut app)).unwrap();
            let buffer = terminal.backend().buffer();
            let mut text = String::new();
            for y in 0..buffer.area.height {
                for x in 0..buffer.area.width {
                    text.push_str(buffer[(x, y)].symbol());
                }
                text.push('\n');
            }

            assert!(
                text.contains("Keyboard Help"),
                "expected Keyboard Help title at {width}x24"
            );
            assert!(
                text.contains("[Esc/?/Enter/q] Close"),
                "expected close hint without clipping at {width}x24:\n{text}"
            );
        }
    }

    #[test]
    fn render_component_list_preserves_task_count_when_narrow() {
        let mut app = make_test_app();
        app.project.components.push(Component {
            id: "bus_worker".to_string(),
            name: "bus_worker".to_string(),
            kind: ComponentKind::Logic,
            parent_id: Some("core_bus".to_string()),
            description: "Worker sub-component".to_string(),
            relative_path: Some("src/bus_worker.rs".to_string()),
            position: GridPos { col: 1, row: 1 },
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        });
        app.project.components.push(Component {
            id: "worker_pool".to_string(),
            name: "worker_pool".to_string(),
            kind: ComponentKind::Data,
            parent_id: Some("bus_worker".to_string()),
            description: "Nested pool".to_string(),
            relative_path: Some("src/worker_pool.rs".to_string()),
            position: GridPos { col: 1, row: 2 },
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: String::new(),
        });
        app.expanded_components.insert("core_bus".to_string());
        app.expanded_components.insert("bus_worker".to_string());

        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();

        let list_rect = column_layout(Rect::new(0, 3, 120, 31))[0];
        let mut list_rows: Vec<String> = Vec::new();
        for y in list_rect.y..list_rect.y.saturating_add(list_rect.height) {
            let mut row = String::new();
            for x in list_rect.x..list_rect.x.saturating_add(list_rect.width) {
                row.push_str(buffer[(x, y)].symbol());
            }
            list_rows.push(row);
        }

        let depth2_row = list_rows
            .iter()
            .find(|row| row.contains("    └─ "))
            .expect("expected depth-2 worker_pool row in component list");
        assert!(
            depth2_row.contains("(0/0)"),
            "expected preserved (0/0) task count in narrow list row, got: {depth2_row:?}"
        );
    }

    #[test]
    fn render_help_modal_clarifies_connect_dependency_direction() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        for width in [80u16, 68u16] {
            let mut app = make_test_app();
            app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));

            let backend = TestBackend::new(width, 24);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|frame| render(frame, &mut app)).unwrap();
            let buffer = terminal.backend().buffer();
            let mut text = String::new();
            for y in 0..buffer.area.height {
                for x in 0..buffer.area.width {
                    text.push_str(buffer[(x, y)].symbol());
                }
                text.push('\n');
            }

            assert!(
                text.contains("Connect (A depends on B)"),
                "expected Connect dependency direction at {width}x24:\n{text}"
            );
            assert!(
                text.contains("[Esc/?/Enter/q] Close"),
                "expected close hint without clipping at {width}x24:\n{text}"
            );
        }
    }

    #[test]
    fn column_layout_gives_canvas_at_least_60_percent_at_140_cols_and_4x4_at_114x26() {
        use crate::tui::canvas::{visible_grid_capacity_for, ZoomLevel};

        let cols_140 = column_layout(Rect::new(0, 3, 140, 36));
        assert!(
            u32::from(cols_140[1].width) * 100 / 140 >= 60,
            "expected middle column >= 60% width at 140 cols, got {}",
            cols_140[1].width
        );

        let cols_114 = column_layout(Rect::new(0, 3, 114, 22));
        let canvas_inner = Block::default().borders(Borders::ALL).inner(cols_114[1]);
        let (vis_cols, vis_rows) = visible_grid_capacity_for(canvas_inner, ZoomLevel::Compact);
        assert!(
            vis_cols >= 4 && vis_rows >= 4,
            "expected >= (4, 4) Compact grid capacity at 114x26, got ({vis_cols}, {vis_rows})"
        );
    }

    #[test]
    fn render_strips_control_chars_from_untrusted_text() {
        use crate::model::FeatureItem;

        let mut app = make_test_app();
        app.project.config.name = "Rocket\x1b[31m\r\x07\t\u{009b}".to_string();
        app.status_message = "Ready\x1b[31m\r\x07\t\u{009b}".to_string();
        app.project.components[0].name = "SafeTitle\x1b[31m\r\x07\t\u{009b}".to_string();
        app.project.components[0].description = "Desc\x1b[31m\r\x07\t\u{009b}".to_string();
        app.project.components[0].tags = vec!["tag\x07\t1".to_string()];
        app.project.components[0].features = vec![FeatureItem {
            id: "feat_1".to_string(),
            title: "Feature\x1b[31m\r\x07\t\u{009b}".to_string(),
            pushed: false,
            external_id: None,
        }];
        app.project.components[0].markdown_body =
            "# Spec\x1b[31m\r\nLine 1\x07\twith\u{009b}tab\nLine 2 long markdown body with \x1b[31m and \x07 and \u{009b}"
                .to_string();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();

        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let sym = buffer[(x, y)].symbol();
                assert!(
                    !sym.chars().any(char::is_control),
                    "found control character in rendered cell ({x}, {y}): {sym:?}"
                );
                text.push_str(sym);
            }
            text.push('\n');
        }

        assert!(text.contains("Rocket"));
        assert!(text.contains("SafeTitle"));
    }

    #[test]
    fn component_list_scrolls_selection_into_view_on_overflow() {
        let mut app = make_test_app();
        app.project.components.clear();
        for idx in 0..20u16 {
            app.project.components.push(Component {
                id: format!("comp_{idx:02}"),
                name: format!("C{idx:02}"),
                kind: ComponentKind::Logic,
                parent_id: None,
                description: String::new(),
                relative_path: None,
                position: GridPos {
                    col: idx % 8,
                    row: idx / 8,
                },
                tags: Vec::new(),
                tech_stack: Vec::new(),
                files: Vec::new(),
                flags: Vec::new(),
                features: Vec::new(),
                external_tasks: Vec::new(),
                markdown_body: String::new(),
            });
        }
        app.selected_index = 18;

        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();

        assert!(
            app.list_state.offset() > 0,
            "expected list_state offset > 0 on overflow, got {}",
            app.list_state.offset()
        );

        let buffer = terminal.backend().buffer();
        let list_rect = column_layout(Rect::new(0, 3, 80, 8))[0];
        let mut list_text = String::new();
        for y in list_rect.y..list_rect.y.saturating_add(list_rect.height) {
            for x in list_rect.x..list_rect.x.saturating_add(list_rect.width) {
                list_text.push_str(buffer[(x, y)].symbol());
            }
            list_text.push('\n');
        }
        assert!(
            list_text.contains("C18"),
            "expected selected component C18 in left column:\n{list_text}"
        );
    }

    #[test]
    fn detail_scroll_clamps_to_wrapped_height_and_resets_on_clamp_selection() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = make_test_app();
        app.project.components.truncate(1);
        let parent_id = app.project.components[0].id.clone();
        let long_md = (0..40)
            .map(|i| format!("Line {i}: Detailed specification paragraph that wraps across narrow detail panel columns."))
            .collect::<Vec<_>>()
            .join("\n");
        app.project.components.push(Component {
            id: "bus_worker".to_string(),
            name: "Bus Worker".to_string(),
            kind: ComponentKind::Logic,
            parent_id: Some(parent_id.clone()),
            description: "Worker sub-component".to_string(),
            relative_path: None,
            position: GridPos { col: 1, row: 1 },
            tags: Vec::new(),
            tech_stack: Vec::new(),
            files: Vec::new(),
            flags: Vec::new(),
            features: Vec::new(),
            external_tasks: Vec::new(),
            markdown_body: long_md,
        });
        app.expanded_components.insert(parent_id.clone());
        app.selected_index = 1;
        app.focus = FocusColumn::Detail;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();

        let max_scroll = app
            .last_detail_max_scroll
            .expect("render sets last_detail_max_scroll");
        assert!(max_scroll > 0 && max_scroll < 500);

        for _ in 0..500 {
            app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        }
        assert_eq!(app.detail_scroll, max_scroll);
        assert!(app.detail_scroll < 500);

        // Collapse parent so clamp_selection changes the active component
        app.expanded_components.remove(&parent_id);
        app.clamp_selection();
        assert_eq!(app.selected_index, 0);
        assert_eq!(app.detail_scroll, 0);
    }

    #[test]
    fn render_multi_edge_modals_fit_at_80x24_with_long_names() {
        use crate::model::{Connection, EdgeKind};

        let long_a = "a".repeat(60);
        let long_b = "b".repeat(60);
        let long_c = "c".repeat(60);
        let long_d = "d".repeat(60);
        let candidates = vec![
            Connection {
                from: long_a.clone(),
                to: long_b,
                kind: EdgeKind::DependsOn,
                label: Some("x".repeat(60)),
            },
            Connection {
                from: long_a.clone(),
                to: long_c,
                kind: EdgeKind::DataFlow,
                label: Some("y".repeat(60)),
            },
            Connection {
                from: long_a,
                to: long_d,
                kind: EdgeKind::Invokes,
                label: None,
            },
        ];

        let mut app = make_test_app();
        app.modal = Some(ModalState::edit_connections(candidates.clone()));
        let backend_edit = TestBackend::new(80, 24);
        let mut terminal_edit = Terminal::new(backend_edit).unwrap();
        terminal_edit.draw(|frame| render(frame, &mut app)).unwrap();
        let buf_edit = terminal_edit.backend().buffer();
        let mut edit_text = String::new();
        for y in 0..buf_edit.area.height {
            for x in 0..buf_edit.area.width {
                edit_text.push_str(buf_edit[(x, y)].symbol());
            }
            edit_text.push('\n');
        }
        assert!(
            edit_text.contains("(1/3)"),
            "expected (1/3) counter in EditConnection modal at 80x24:\n{edit_text}"
        );
        assert!(
            edit_text.contains("[/]"),
            "expected [/] cycling hint in EditConnection modal at 80x24:\n{edit_text}"
        );

        app.modal = Some(ModalState::confirm_delete_connections(candidates));
        let backend_del = TestBackend::new(80, 24);
        let mut terminal_del = Terminal::new(backend_del).unwrap();
        terminal_del.draw(|frame| render(frame, &mut app)).unwrap();
        let buf_del = terminal_del.backend().buffer();
        let mut del_text = String::new();
        for y in 0..buf_del.area.height {
            for x in 0..buf_del.area.width {
                del_text.push_str(buf_del[(x, y)].symbol());
            }
            del_text.push('\n');
        }
        assert!(
            del_text.contains("(1/3)"),
            "expected (1/3) counter in ConfirmDelete modal at 80x24:\n{del_text}"
        );
        assert!(
            del_text.contains("↑/↓"),
            "expected ↑/↓ cycling hint in ConfirmDelete modal at 80x24:\n{del_text}"
        );
    }
}
