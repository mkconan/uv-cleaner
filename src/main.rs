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
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use walkdir::WalkDir;

#[derive(Clone)]
struct Project {
    path: PathBuf,
    venv_path: PathBuf,
    last_modified: SystemTime,
    selected: bool,
}

struct App {
    items: Vec<Project>,
    index: usize,
    confirm: bool,
}

fn main() -> Result<(), io::Error> {
    let root = dirs_next::home_dir().unwrap().join("Development"); // 好きに変えてOK
    let projects = scan_projects(&root, 30);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, projects);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
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
                            projects.push(Project {
                                path: project_dir.to_path_buf(),
                                venv_path: venv,
                                last_modified: mtime,
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
) -> io::Result<()> {
    let mut app = App {
        items,
        index: 0,
        confirm: false,
    };

    loop {
        terminal.draw(|f| ui(f, &app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),

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

fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(f.area());

    let items: Vec<ListItem> = app
        .items
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let checkbox = if p.selected { "[x]" } else { "[ ]" };

            let dt: DateTime<Local> = p.last_modified.into();
            let text = format!(
                "{} {} ({})",
                checkbox,
                p.path.display(),
                dt.format("%Y-%m-%d")
            );

            let style = if i == app.index {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("uv venv cleaner")
            .borders(Borders::ALL),
    );

    f.render_widget(list, chunks[0]);

    let help = if app.confirm {
        format!("Delete {} items? (y/n)", app.selected_count())
    } else {
        "↑↓: move  space: select  d: delete  q: quit".to_string()
    };

    let footer = Paragraph::new(help).block(Block::default().borders(Borders::ALL));

    f.render_widget(footer, chunks[1]);
}

impl App {
    fn selected_count(&self) -> usize {
        self.items.iter().filter(|p| p.selected).count()
    }
}
