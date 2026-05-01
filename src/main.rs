mod app;
mod completion;
mod model;
mod scanner;
mod ui;

use std::io;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use scanner::{SCAN_DAYS, scan_projects};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = dirs_next::home_dir()
        .ok_or("Could not determine home directory")?
        .join("Development");

    if !root.exists() {
        eprintln!("Warning: {} does not exist", root.display());
    }

    let root_str = root.to_string_lossy().to_string();
    let projects = scan_projects(&root, SCAN_DAYS).unwrap_or_else(|e| {
        eprintln!("Scan error: {}", e);
        vec![]
    });

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    app::run_app(&mut terminal, projects, root_str)?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
