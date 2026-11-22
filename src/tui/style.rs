use ratatui::style::{Color, Modifier, Style};

use crate::model::CardStatus;
use crate::theme::{RgbColor, Theme};

pub fn to_color(c: RgbColor) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

pub fn base_style(theme: &Theme) -> Style {
    Style::default()
        .fg(to_color(theme.palette.foreground))
        .bg(to_color(theme.palette.background))
}

pub fn muted_style(theme: &Theme) -> Style {
    Style::default()
        .fg(to_color(theme.palette.muted))
        .bg(to_color(theme.palette.background))
}

pub fn header_style(theme: &Theme) -> Style {
    Style::default()
        .fg(to_color(theme.palette.background))
        .bg(to_color(theme.palette.foreground))
        .add_modifier(Modifier::BOLD)
}

pub fn panel_border_style(theme: &Theme, focused: bool) -> Style {
    Style::default().fg(to_color(if focused {
        theme.palette.border_focused
    } else {
        theme.palette.border_default
    }))
}

pub fn selected_row_style(theme: &Theme) -> Style {
    Style::default()
        .fg(to_color(theme.palette.background))
        .bg(to_color(theme.palette.accent))
        .add_modifier(Modifier::BOLD)
}

pub fn card_border_color(theme: &Theme, status: CardStatus) -> Color {
    let rgb = match status {
        CardStatus::Gap => theme.palette.status_gap,
        CardStatus::Planned => theme.palette.status_planned,
        CardStatus::InProgress => theme.palette.status_in_progress,
        CardStatus::Blocked => theme.palette.status_blocked,
        CardStatus::Complete => theme.palette.status_complete,
    };
    to_color(rgb)
}

pub fn status_glyph(status: CardStatus) -> &'static str {
    match status {
        CardStatus::Gap => "[!]",
        CardStatus::Planned => "[ ]",
        CardStatus::InProgress => "[~]",
        CardStatus::Blocked => "[X]",
        CardStatus::Complete => "[✓]",
    }
}

pub fn markdown_line_style(theme: &Theme, line: &str) -> Style {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        Style::default()
            .fg(to_color(theme.palette.md_heading))
            .add_modifier(Modifier::BOLD)
    } else if trimmed.starts_with("```") || line.starts_with("    ") {
        Style::default().fg(to_color(theme.palette.md_code))
    } else if trimmed.contains("ERROR") || trimmed.contains("BLOCKED") {
        Style::default()
            .fg(to_color(theme.palette.alert_error))
            .add_modifier(Modifier::BOLD)
    } else if trimmed.contains("WARN") || trimmed.contains("TODO") {
        Style::default().fg(to_color(theme.palette.alert_warn))
    } else {
        Style::default().fg(to_color(theme.palette.foreground))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::default_theme;

    #[test]
    fn card_border_color_maps_complete_to_green_and_gap_to_gap_color() {
        let theme = default_theme().unwrap();
        assert_eq!(
            card_border_color(&theme, CardStatus::Complete),
            to_color(theme.palette.status_complete)
        );
        assert_eq!(
            card_border_color(&theme, CardStatus::Gap),
            to_color(theme.palette.status_gap)
        );
    }

    #[test]
    fn markdown_line_style_differentiates_headings_code_and_alerts() {
        let theme = default_theme().unwrap();
        assert_eq!(
            markdown_line_style(&theme, "# Overview").fg,
            Some(to_color(theme.palette.md_heading))
        );
        assert_eq!(
            markdown_line_style(&theme, "```rust").fg,
            Some(to_color(theme.palette.md_code))
        );
        assert_eq!(
            markdown_line_style(&theme, "BLOCKED: missing view").fg,
            Some(to_color(theme.palette.alert_error))
        );
    }
}
