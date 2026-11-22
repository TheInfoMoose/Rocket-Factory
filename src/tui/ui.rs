use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::audit::{compute_card_status, task_progress};
use crate::theme::Theme;
use crate::tui::app::{App, FocusColumn, ListFilter};
use crate::tui::modal::{edge_kind_label, ModalState};
use crate::tui::style;

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

    let cols = Layout::horizontal([
        Constraint::Percentage(24),
        Constraint::Percentage(48),
        Constraint::Percentage(28),
    ])
    .split(chunks[1]);

    if let Some(selected_pos) = app.selected_component().map(|selected| selected.position) {
        let canvas_inner = Block::default().borders(Borders::ALL).inner(cols[1]);
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
        " [Tab] Focus [n/e/E] Edit [v] Edit MD [f] Gaps [m] Move [c] Conn [r/S] Sync [p] Push [t] Theme [?] Help [q] Quit",
    )
    .style(style::muted_style(&app.theme));
    frame.render_widget(footer, area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x.saturating_add(area.width.saturating_sub(w) / 2);
    let y = area.y.saturating_add(area.height.saturating_sub(h) / 2);
    Rect::new(x, y, w, h)
}

fn modal_field_line(label: &str, value: &str, active: bool, theme: &Theme) -> Line<'static> {
    let prefix = if active { "> " } else { "  " };
    let field_style = if active {
        style::selected_row_style(theme)
    } else {
        style::base_style(theme)
    };
    Line::from(Span::styled(
        format!("{prefix}{label}: {value}"),
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
                format!("  ID: {}", form.component_id),
                muted,
            )));
            lines.push(modal_field_line(
                "Name",
                &form.name,
                form.active_field == 0,
                theme,
            ));
            lines.push(modal_field_line(
                "Description",
                &form.description,
                form.active_field == 1,
                theme,
            ));
            lines.push(modal_field_line(
                "Path",
                &form.relative_path,
                form.active_field == 2,
                theme,
            ));
            lines.push(modal_field_line(
                "Tags",
                &form.tags,
                form.active_field == 3,
                theme,
            ));
            lines.push(modal_field_line(
                "Add Feature",
                &form.new_feature,
                form.active_field == 4,
                theme,
            ));
            form.error.as_deref()
        }
        ModalState::EditConnection(form) => {
            lines.push(Line::from(Span::styled(
                format!("  Edge: {} -> {}", form.from, form.to),
                muted,
            )));
            lines.push(modal_field_line(
                "Kind",
                &format!("< {} >", edge_kind_label(form.selected_kind())),
                form.active_field == 0,
                theme,
            ));
            lines.push(modal_field_line(
                "Label",
                &form.label,
                form.active_field == 1,
                theme,
            ));
            form.error.as_deref()
        }
        ModalState::Help => {
            lines.push(Line::from(Span::styled("  Navigation", heading_style)));
            lines.push(Line::from(Span::styled(
                "    Tab / Shift+Tab   Cycle focus (List / Canvas / Detail)",
                base,
            )));
            lines.push(Line::from(Span::styled(
                "    hjkl / Arrows     Move selection / Scroll detail",
                base,
            )));
            lines.push(Line::from(Span::styled(
                "    f                 Toggle Gaps Only filter",
                base,
            )));
            lines.push(Line::from(Span::styled(
                "  Editing & Canvas",
                heading_style,
            )));
            lines.push(Line::from(Span::styled(
                "    n / e / E         New component / Edit component / Edit edge",
                base,
            )));
            lines.push(Line::from(Span::styled(
                "    m / c             Move card mode / Connect components",
                base,
            )));
            lines.push(Line::from(Span::styled(
                "    v                 Open component Markdown in $EDITOR",
                base,
            )));
            lines.push(Line::from(Span::styled("  Sync & Display", heading_style)));
            lines.push(Line::from(Span::styled(
                "    r / S             Pull status from active adapters",
                base,
            )));
            lines.push(Line::from(Span::styled(
                "    p                 Push selected component & features",
                base,
            )));
            lines.push(Line::from(Span::styled(
                "    t                 Switch RON theme",
                base,
            )));
            lines.push(Line::from(Span::styled("  General", heading_style)));
            lines.push(Line::from(Span::styled(
                "    ?                 Toggle this keyboard help",
                base,
            )));
            lines.push(Line::from(Span::styled(
                "    Esc / q           Cancel mode / Quit",
                base,
            )));
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
                        format!("{prefix}{theme_name}{suffix}"),
                        row_style,
                    )));
                }
            }
            form.error.as_deref()
        }
    };

    if let Some(err) = error_text {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            error_style,
        )));
    }

    let footer_hint = match modal {
        ModalState::Help => "  [Esc/?/Enter/q] Close",
        ModalState::ThemeSwitcher(_) => "  [↑/↓] Select  [Enter] Apply  [Esc] Cancel",
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
        .clamp(54, 76)
        .min(max_popup_width);
    let popup_height = (lines.len() as u16).saturating_add(2);
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

        assert!(text.contains("ROCKET FACTORY"));
        assert!(text.contains("GAPS: 1"));
        assert!(text.contains("Components"));
        assert!(text.contains("Snap Grid"));
        assert!(text.contains("Component Details"));
        assert!(text.contains("view_box"));
        assert!(text.contains("[v] Edit MD"));
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
        assert!(base_text.contains("[r/S] Sync"));
        assert!(base_text.contains("[p] Push"));
        assert!(base_text.contains("[t] Theme"));
        assert!(base_text.contains("[?] Help"));

        // Help modal
        app.modal = Some(ModalState::help());
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
    fn render_help_modal_fits_80x24_without_wrapping_or_footer_clipping() {
        let mut app = make_test_app();
        app.modal = Some(ModalState::help());

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
        assert!(text.contains("New component / Edit component / Edit edge"));
        assert!(text.contains("[Esc/?/Enter/q] Close"));
    }
}
