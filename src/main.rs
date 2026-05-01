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
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
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
    suggestions: Vec<String>,
    suggestion_index: Option<usize>,
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

fn project_last_modified(project_dir: &Path) -> SystemTime {
    let skip_dirs = [
        ".git",
        ".venv",
        "__pycache__",
        "node_modules",
        "target",
        ".tox",
        ".mypy_cache",
        ".pytest_cache",
    ];
    WalkDir::new(project_dir)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                !skip_dirs.iter().any(|s| name == *s)
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter_map(|m| m.modified().ok())
        .max()
        .unwrap_or(SystemTime::UNIX_EPOCH)
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
                    if let Ok(venv_mtime) = meta.modified() {
                        if venv_mtime < threshold {
                            let venv_size = dir_size(&venv);
                            let last_modified = project_last_modified(project_dir);
                            projects.push(Project {
                                path: project_dir.to_path_buf(),
                                venv_path: venv,
                                last_modified,
                                venv_size,
                                selected: false,
                            });
                        }
                    }
                }
            }
        }
    }

    // 古い順（プロジェクト全体の最終更新日が古い順）にソート
    projects.sort_by(|a, b| a.last_modified.cmp(&b.last_modified));
    projects
}

fn compute_suggestions(input: &str) -> Vec<String> {
    let expanded = if input.starts_with('~') {
        if let Some(home) = dirs_next::home_dir() {
            home.to_string_lossy().to_string() + &input[1..]
        } else {
            input.to_string()
        }
    } else {
        input.to_string()
    };

    let path = PathBuf::from(&expanded);
    let (dir_to_list, prefix) = if expanded.ends_with('/') {
        (path.clone(), String::new())
    } else {
        let parent = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("/"));
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        (parent, file_name)
    };

    let Ok(entries) = fs::read_dir(&dir_to_list) else {
        return vec![];
    };

    let mut suggestions: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with(&prefix)
        })
        .map(|e| {
            let mut s = dir_to_list
                .join(e.file_name())
                .to_string_lossy()
                .to_string();
            s.push('/');
            s
        })
        .collect();

    suggestions.sort();
    suggestions
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
        suggestions: vec![],
        suggestion_index: None,
    };

    loop {
        terminal.draw(|f| ui(f, &app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if app.input_mode {
                    match key.code {
                        KeyCode::Enter => {
                            if let Some(idx) = app.suggestion_index {
                                // 補完候補を確定してさらに編集を続ける
                                if let Some(s) = app.suggestions.get(idx) {
                                    app.input = s.clone();
                                    app.suggestions = compute_suggestions(&app.input);
                                    app.suggestion_index = None;
                                }
                            } else {
                                // パスでスキャン実行
                                let path = PathBuf::from(&app.input);
                                app.items = scan_projects(&path, 30);
                                app.index = 0;
                                app.confirm = false;
                                app.input_mode = false;
                                app.suggestions = vec![];
                                app.suggestion_index = None;
                            }
                        }
                        KeyCode::Esc => {
                            if app.suggestion_index.is_some() {
                                app.suggestion_index = None;
                            } else {
                                app.input_mode = false;
                                app.suggestions = vec![];
                            }
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                            app.suggestions = compute_suggestions(&app.input);
                            app.suggestion_index = None;
                        }
                        KeyCode::Tab => {
                            if !app.suggestions.is_empty() {
                                app.suggestion_index = Some(match app.suggestion_index {
                                    None => 0,
                                    Some(i) => (i + 1) % app.suggestions.len(),
                                });
                            }
                        }
                        KeyCode::Down => {
                            if !app.suggestions.is_empty() {
                                app.suggestion_index = Some(match app.suggestion_index {
                                    None => 0,
                                    Some(i) => (i + 1).min(app.suggestions.len() - 1),
                                });
                            }
                        }
                        KeyCode::Up => {
                            if let Some(i) = app.suggestion_index {
                                app.suggestion_index = if i == 0 { None } else { Some(i - 1) };
                            }
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                            app.suggestions = compute_suggestions(&app.input);
                            app.suggestion_index = None;
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),

                        KeyCode::Tab => {
                            app.suggestions = compute_suggestions(&app.input);
                            app.suggestion_index = None;
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

                        KeyCode::Char('a') => {
                            let all_selected = app.items.iter().all(|p| p.selected);
                            for p in &mut app.items {
                                p.selected = !all_selected;
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
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    let input_title = if app.input_mode {
        "Scan root  [Enter: apply/scan  Tab/↑↓: suggest  Esc: cancel]"
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

    let selected_total: u64 = app
        .items
        .iter()
        .filter(|p| p.selected)
        .map(|p| p.venv_size)
        .sum();

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

    let list_title = format!(
        "uv venv cleaner  [{}/{} selected  {}]",
        app.selected_count(),
        app.items.len(),
        if selected_total > 0 {
            format!("{} to free", size_label(selected_total))
        } else {
            "none selected".to_string()
        }
    );

    let list = List::new(items).block(Block::default().title(list_title).borders(Borders::ALL));
    f.render_widget(list, chunks[1]);

    // --- フッター ---
    let help = if app.confirm {
        format!(
            "Delete {} items ({})? (y/n)",
            app.selected_count(),
            size_label(
                app.items
                    .iter()
                    .filter(|p| p.selected)
                    .map(|p| p.venv_size)
                    .sum()
            )
        )
    } else if app.input_mode {
        "Type path  Tab/↓: next suggest  ↑: prev  Enter: apply/scan  Esc: cancel".to_string()
    } else {
        "↑↓: move  Space: select  a: all/none  d: delete  Tab: edit path  q: quit".to_string()
    };

    let footer = Paragraph::new(help).block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);

    // --- 補完候補ポップアップ ---
    if app.input_mode && !app.suggestions.is_empty() {
        let popup_height = (app.suggestions.len() as u16 + 2).min(10);
        let input_area = chunks[0];
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

        let suggestion_list = List::new(suggestion_items).block(
            Block::default()
                .title("Suggestions")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

        f.render_widget(Clear, popup_area);
        f.render_widget(suggestion_list, popup_area);
    }
}

impl App {
    fn selected_count(&self) -> usize {
        self.items.iter().filter(|p| p.selected).count()
    }
}
