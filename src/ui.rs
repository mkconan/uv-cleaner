use std::path::PathBuf;

use chrono::{DateTime, Local};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::model::App;

const BAR_WIDTH: usize = 20;
const MB: u64 = 1024 * 1024;

pub fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(f.area());

    render_input(f, app, chunks[0]);
    render_list(f, app, chunks[1]);
    render_footer(f, app, chunks[2]);

    if app.input_mode && !app.suggestions.is_empty() {
        render_suggestion_popup(f, app, chunks[0]);
    }
}

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.input_mode {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    let title = if app.input_mode {
        "Scan root  [Enter: apply/scan  Tab/↑↓: suggest  Esc: cancel]"
    } else {
        "Scan root  [Tab: edit]"
    };
    let widget = Paragraph::new(app.input.as_str()).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style),
    );
    f.render_widget(widget, area);
}

fn render_list(f: &mut Frame, app: &App, area: Rect) {
    let max_size = app
        .items
        .iter()
        .map(|p| p.venv_size)
        .max()
        .unwrap_or(1)
        .max(1);

    // Compute visible window (each item = 2 lines; subtract 2 for borders)
    let visible = (area.height as usize).saturating_sub(2) / 2;
    let start = app.scroll_offset;
    let end = (start + visible).min(app.items.len());

    let items: Vec<ListItem> = app.items[start..end]
        .iter()
        .enumerate()
        .map(|(rel_i, p)| {
            let abs_i = rel_i + start;
            let checkbox = if p.selected { "[x]" } else { "[ ]" };
            let dt: DateTime<Local> = p.last_modified.into();
            let header = format!(
                "{} {} ({})",
                checkbox,
                p.path.display(),
                dt.format("%Y-%m-%d")
            );

            let row_style = if abs_i == app.index {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let filled = (p.venv_size * BAR_WIDTH as u64 / max_size) as usize;
            let unfilled = BAR_WIDTH - filled;
            let color = bar_color(p.venv_size);

            let bar_line = Line::from(vec![
                Span::raw("    "),
                Span::styled("█".repeat(filled), Style::default().fg(color)),
                Span::styled("░".repeat(unfilled), Style::default().fg(Color::DarkGray)),
                Span::raw(format!("  {}", size_label(p.venv_size))),
            ]);

            let text = Text::from(vec![Line::from(Span::styled(header, row_style)), bar_line]);
            ListItem::new(text)
        })
        .collect();

    let total = app.selected_total();
    let scroll_indicator = if app.items.len() > visible && visible > 0 {
        format!("  [{}-{}/{}]", start + 1, end, app.items.len())
    } else {
        String::new()
    };
    let list_title = format!(
        "uv venv cleaner  [{}/{} selected  {}]{}",
        app.selected_count(),
        app.items.len(),
        if total > 0 {
            format!("{} to free", size_label(total))
        } else {
            "none selected".to_string()
        },
        scroll_indicator,
    );

    let list = List::new(items).block(Block::default().title(list_title).borders(Borders::ALL));
    f.render_widget(list, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let (text, style) = if let Some(ref err) = app.last_error {
        (format!("Error: {}", err), Style::default().fg(Color::Red))
    } else if app.confirm {
        (
            format!(
                "Delete {} items ({})? (y/n)",
                app.selected_count(),
                size_label(app.selected_total()),
            ),
            Style::default(),
        )
    } else if app.input_mode {
        (
            "Type path  Tab/↓: next suggest  ↑: prev  Enter: apply/scan  Esc: cancel".to_string(),
            Style::default(),
        )
    } else {
        (
            "↑↓: move  Space: select  a: all/none  d: delete  Tab: edit path  q: quit".to_string(),
            Style::default(),
        )
    };

    let footer = Paragraph::new(text)
        .style(style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, area);
}

fn render_suggestion_popup(f: &mut Frame, app: &App, input_area: Rect) {
    let popup_height = (app.suggestions.len() as u16 + 2).min(10);
    let popup_area = Rect {
        x: input_area.x,
        y: input_area.y + input_area.height,
        width: input_area.width,
        height: popup_height,
    };

    let suggestion_items: Vec<ListItem> = app
        .suggestions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = if Some(i) == app.suggestion_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            let display = PathBuf::from(s)
                .file_name()
                .map(|n| format!("{}/", n.to_string_lossy()))
                .unwrap_or_else(|| s.clone());
            ListItem::new(Line::from(Span::styled(display, style)))
        })
        .collect();

    let list = List::new(suggestion_items).block(
        Block::default()
            .title("Suggestions")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(Clear, popup_area);
    f.render_widget(list, popup_area);
}

fn size_label(bytes: u64) -> String {
    if bytes >= 1024 * MB {
        format!("{:.2} GB", bytes as f64 / (1024.0 * MB as f64))
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{} KB", bytes / 1024)
    }
}

fn bar_color(size: u64) -> Color {
    if size >= 500 * MB {
        Color::Red
    } else if size >= 100 * MB {
        Color::Yellow
    } else {
        Color::Green
    }
}
