use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::audit::{compute_card_status, task_progress};
use crate::tui::app::{App, FocusColumn, ListFilter};
use crate::tui::style;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame.render_widget(Block::default().style(style::base_style(&app.theme)), area);

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    render_header(frame, chunks[0], app);

    let cols = Layout::horizontal([
        Constraint::Percentage(24),
        Constraint::Percentage(48),
        Constraint::Percentage(28),
    ])
    .split(chunks[1]);

    render_component_list(frame, cols[0], app);
    render_canvas_placeholder(frame, cols[1], app);
    render_detail_panel(frame, cols[2], app);

    render_footer(frame, chunks[2], app);
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
            format!(" ROCKET FACTORY :: {}", app.project.config.name),
            style::header_style(&app.theme),
        ),
        Span::styled(
            format!(" | MODE: {:?} | ", app.mode),
            style::base_style(&app.theme),
        ),
        Span::styled(format!("GAPS: {gap_count}"), gap_style),
        Span::styled(
            format!(" | {}", app.status_message),
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

fn render_component_list(frame: &mut Frame, area: Rect, app: &App) {
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
        let empty = Paragraph::new(" (no components) ")
            .style(style::muted_style(&app.theme))
            .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .filter_map(|(row_idx, &comp_idx)| {
            let comp = app.project.components.get(comp_idx)?;
            let status = compute_card_status(comp);
            let glyph = style::status_glyph(status);
            let progress = task_progress(comp);
            let label = format!(
                "{glyph} {} ({}/{})",
                comp.name, progress.complete, progress.total
            );
            let item_style = if row_idx == app.selected_index {
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
    frame.render_widget(list, area);
}

fn render_canvas_placeholder(frame: &mut Frame, area: Rect, app: &App) {
    let grid = app.project.config.grid;
    let title = format!(" Snap Grid ({}x{}) ", grid.cols, grid.rows);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(style::panel_border_style(
            &app.theme,
            app.focus == FocusColumn::Canvas,
        ))
        .style(style::base_style(&app.theme));

    let count = app.project.components.len();
    let edges = app.project.config.connections.len();

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{count} staged component(s), {edges} connection(s)"),
            style::base_style(&app.theme).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Snap Grid Canvas — interactive cards & connectors in Phase 4",
            style::muted_style(&app.theme),
        )),
    ];

    let placeholder = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
        .block(block);
    frame.render_widget(placeholder, area);
}

fn render_detail_panel(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Component Details ")
        .borders(Borders::ALL)
        .border_style(style::panel_border_style(
            &app.theme,
            app.focus == FocusColumn::Detail,
        ))
        .style(style::base_style(&app.theme));

    let Some(comp) = app.selected_component() else {
        let empty = Paragraph::new("Select a component to inspect details.")
            .style(style::muted_style(&app.theme))
            .wrap(Wrap { trim: false })
            .block(block);
        frame.render_widget(empty, area);
        return;
    };

    let status = compute_card_status(comp);
    let path_display = comp.relative_path.as_deref().unwrap_or("-");
    let tags_display = if comp.tags.is_empty() {
        "-".to_string()
    } else {
        comp.tags.join(", ")
    };

    let heading_style = Style::default()
        .fg(style::to_color(app.theme.palette.accent))
        .add_modifier(Modifier::BOLD);
    let base = style::base_style(&app.theme);
    let muted = style::muted_style(&app.theme);

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            format!("Name: {} ({})", comp.name, comp.id),
            base.add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(
                "Kind: {:?} | Status: {:?} | Grid: ({}, {})",
                comp.kind, status, comp.position.col, comp.position.row
            ),
            Style::default().fg(style::card_border_color(&app.theme, status)),
        )),
        Line::from(Span::styled(format!("Path: {path_display}"), base)),
        Line::from(Span::styled(format!("Tags: {tags_display}"), base)),
        Line::from(Span::styled(
            format!("Description: {}", comp.description),
            base,
        )),
        Line::from(""),
        Line::from(Span::styled("Connections:", heading_style)),
    ];

    let mut has_connections = false;
    for conn in &app.project.config.connections {
        if conn.from == comp.id {
            has_connections = true;
            lines.push(Line::from(Span::styled(
                format!("  -> {} [{:?}]", conn.to, conn.kind),
                base,
            )));
        }
        if conn.to == comp.id {
            has_connections = true;
            lines.push(Line::from(Span::styled(
                format!("  <- {} [{:?}]", conn.from, conn.kind),
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
                format!("  {state} {}", feature.title),
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
                format!("  [{:?}] {} {}", task.status, task.external_id, task.title),
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
            lines.push(Line::from(Span::styled(
                md_line.to_string(),
                style::markdown_line_style(&app.theme, md_line),
            )));
        }
    }

    let detail = Paragraph::new(lines)
        .style(base)
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0))
        .block(block);
    frame.render_widget(detail, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let footer = Paragraph::new(
        " [Tab] Focus  [j/k ↑/↓] Navigate/Scroll  [f] Gaps Filter  [m] Move  [c] Connect  [q] Quit",
    )
    .style(style::muted_style(&app.theme));
    frame.render_widget(footer, area);
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

    fn render_to_string(app: &App) -> String {
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
        let app = make_test_app();
        let text = render_to_string(&app);

        assert!(text.contains("ROCKET FACTORY"));
        assert!(text.contains("GAPS: 1"));
        assert!(text.contains("Components"));
        assert!(text.contains("Snap Grid"));
        assert!(text.contains("Component Details"));
        assert!(text.contains("view_box"));
    }

    #[test]
    fn render_gaps_filter_hides_tracked_component() {
        let mut app = make_test_app();
        app.toggle_gaps_filter();
        let text = render_to_string(&app);

        assert!(text.contains("Components [GAPS]"));
        assert!(text.contains("view_box"));
        assert!(!text.contains("core_bus"));
    }
}
