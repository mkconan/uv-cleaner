use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Local};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use walkdir::WalkDir;

const BAR_WIDTH: usize = 20;
const MB: u64 = 1024 * 1024;

#[derive(Clone)]
struct Project {
    path: PathBuf,
    venv_path: PathBuf,
    last_modified: SystemTime,
    venv_size: u64,
    selected: bool,
}

struct App {
    items: Vec<Project>,
    index: usize,
    confirm: bool,
    input: String,
    input_mode: bool,
}

fn main() -> Result<(), io::Error> {
    let root = dirs_next::home_dir().unwrap().join("Development");
    let root_str = root.to_string_lossy().to_string();
    let projects = scan_projects(&root, 30);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, projects, root_str);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

fn scan_projects(root: &Path, days: u64) -> Vec<Project> {
    let mut projects = vec![];
    let threshold = SystemTime::now() - Duration::from_secs(days * 86400);

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_name() == "pyproject.toml" {
            let project_dir = entry.path().parent().unwrap();
            let venv = project_dir.join(".venv");

            if venv.exists() {
                if let Ok(meta) = fs::metadata(&venv) {
                    if let Ok(mtime) = meta.modified() {
                        if mtime < threshold {
                            let venv_size = dir_size(&venv);
                            projects.push(Project {
                                path: project_dir.to_path_buf(),
                                venv_path: venv,
                                last_modified: mtime,
                                venv_size,
                                selected: false,
                            });
                        }
                    }
                }
            }
        }
    }

    projects
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    items: Vec<Project>,
    root: String,
) -> io::Result<()> {
    let mut app = App {
        items,
        index: 0,
        confirm: false,
        input: root,
        input_mode: false,
    };

    loop {
        terminal.draw(|f| ui(f, &app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if app.input_mode {
                    match key.code {
                        KeyCode::Enter => {
                            let path = PathBuf::from(&app.input);
                            app.items = scan_projects(&path, 30);
                            app.index = 0;
                            app.confirm = false;
                            app.input_mode = false;
                        }
                        KeyCode::Esc => {
                            app.input_mode = false;
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),

                        KeyCode::Tab => {
                            app.input_mode = true;
                        }

                        KeyCode::Down => {
                            if app.index + 1 < app.items.len() {
                                app.index += 1;
                            }
                        }
                        KeyCode::Up => {
                            if app.index > 0 {
                                app.index -= 1;
                            }
                        }

                        KeyCode::Char(' ') => {
                            if let Some(p) = app.items.get_mut(app.index) {
                                p.selected = !p.selected;
                            }
                        }

                        KeyCode::Char('d') => {
                            if app.selected_count() > 0 {
                                app.confirm = true;
                            }
                        }

                        KeyCode::Char('y') => {
                            if app.confirm {
                                delete_selected(&mut app);
                                app.confirm = false;
                            }
                        }

                        KeyCode::Char('n') => {
                            app.confirm = false;
                        }

                        _ => {}
                    }
                }
            }
        }
    }
}

fn delete_selected(app: &mut App) {
    app.items.retain(|p| {
        if p.selected {
            let _ = fs::remove_dir_all(&p.venv_path);
            false
        } else {
            true
        }
    });

    if app.index >= app.items.len() && app.index > 0 {
        app.index -= 1;
    }
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

fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(f.area());

    // --- パス入力欄 ---
    let input_border_style = if app.input_mode {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let input_title = if app.input_mode {
        "Scan root  [Enter: scan  Esc: cancel]"
    } else {
        "Scan root  [Tab: edit]"
    };
    let input_widget = Paragraph::new(app.input.as_str()).block(
        Block::default()
            .title(input_title)
            .borders(Borders::ALL)
            .border_style(input_border_style),
    );
    f.render_widget(input_widget, chunks[0]);

    // --- プロジェクトリスト ---
    let max_size = app
        .items
        .iter()
        .map(|p| p.venv_size)
        .max()
        .unwrap_or(1)
        .max(1);

    let items: Vec<ListItem> = app
        .items
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let checkbox = if p.selected { "[x]" } else { "[ ]" };
            let dt: DateTime<Local> = p.last_modified.into();
            let header = format!(
                "{} {} ({})",
                checkbox,
                p.path.display(),
                dt.format("%Y-%m-%d")
            );

            let row_style = if i == app.index {
                Style::default().fg(Color::Yellow)
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

    let list = List::new(items).block(
        Block::default()
            .title("uv venv cleaner")
            .borders(Borders::ALL),
    );

    f.render_widget(list, chunks[1]);

    // --- フッター ---
    let help = if app.confirm {
        format!("Delete {} items? (y/n)", app.selected_count())
    } else if app.input_mode {
        "Type path  Enter: scan  Esc: cancel".to_string()
    } else {
        "↑↓: move  Space: select  d: delete  Tab: edit path  q: quit".to_string()
    };

    let footer = Paragraph::new(help).block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}

impl App {
    fn selected_count(&self) -> usize {
        self.items.iter().filter(|p| p.selected).count()
    }
}
