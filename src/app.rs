use std::{fs, io, path::PathBuf, time::Duration};

use crossterm::event::{self, Event, KeyCode};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    completion::compute_suggestions,
    model::{App, Project},
    scanner::{SCAN_DAYS, scan_projects},
    ui::ui,
};

const POLL_MS: u64 = 200;

impl App {
    pub fn new(items: Vec<Project>, root: String) -> Self {
        App {
            items,
            index: 0,
            confirm: false,
            input: root,
            input_mode: false,
            suggestions: vec![],
            suggestion_index: None,
            scroll_offset: 0,
            last_error: None,
        }
    }

    pub fn selected_count(&self) -> usize {
        self.items.iter().filter(|p| p.selected).count()
    }

    pub fn selected_total(&self) -> u64 {
        self.items
            .iter()
            .filter(|p| p.selected)
            .map(|p| p.venv_size)
            .sum()
    }

    pub fn delete_selected(&mut self) {
        let mut errors = vec![];
        self.items.retain(|p| {
            if p.selected {
                if let Err(e) = fs::remove_dir_all(&p.venv_path) {
                    errors.push(format!("{}: {}", p.venv_path.display(), e));
                }
                false
            } else {
                true
            }
        });
        if self.index >= self.items.len() && self.index > 0 {
            self.index -= 1;
        }
        if self.scroll_offset > self.index {
            self.scroll_offset = self.index;
        }
        self.last_error = if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        };
    }

    pub fn ensure_cursor_visible(&mut self, visible: usize) {
        if visible == 0 {
            return;
        }
        if self.index < self.scroll_offset {
            self.scroll_offset = self.index;
        } else if self.index >= self.scroll_offset + visible {
            self.scroll_offset = self.index + 1 - visible;
        }
    }

    /// Returns `false` if the app should quit.
    fn handle_key_input_mode(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Enter => {
                if let Some(idx) = self.suggestion_index {
                    if let Some(s) = self.suggestions.get(idx) {
                        self.input = s.clone();
                        self.suggestions = compute_suggestions(&self.input);
                        self.suggestion_index = None;
                    }
                } else {
                    let path = PathBuf::from(&self.input);
                    self.items = scan_projects(&path, SCAN_DAYS);
                    self.index = 0;
                    self.scroll_offset = 0;
                    self.confirm = false;
                    self.input_mode = false;
                    self.suggestions = vec![];
                    self.suggestion_index = None;
                    self.last_error = None;
                }
            }
            KeyCode::Esc => {
                if self.suggestion_index.is_some() {
                    self.suggestion_index = None;
                } else {
                    self.input_mode = false;
                    self.suggestions = vec![];
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
                self.suggestions = compute_suggestions(&self.input);
                self.suggestion_index = None;
            }
            KeyCode::Tab => {
                if !self.suggestions.is_empty() {
                    self.suggestion_index = Some(match self.suggestion_index {
                        None => 0,
                        Some(i) => (i + 1) % self.suggestions.len(),
                    });
                }
            }
            KeyCode::Down => {
                if !self.suggestions.is_empty() {
                    self.suggestion_index = Some(match self.suggestion_index {
                        None => 0,
                        Some(i) => (i + 1).min(self.suggestions.len() - 1),
                    });
                }
            }
            KeyCode::Up => {
                if let Some(i) = self.suggestion_index {
                    self.suggestion_index = if i == 0 { None } else { Some(i - 1) };
                }
            }
            KeyCode::Char(c) => {
                self.input.push(c);
                self.suggestions = compute_suggestions(&self.input);
                self.suggestion_index = None;
            }
            _ => {}
        }
        true
    }

    /// Returns `false` if the app should quit.
    fn handle_key_normal(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') => return false,

            KeyCode::Tab => {
                self.suggestions = compute_suggestions(&self.input);
                self.suggestion_index = None;
                self.input_mode = true;
            }

            KeyCode::Down => {
                if self.index + 1 < self.items.len() {
                    self.index += 1;
                }
            }
            KeyCode::Up => {
                if self.index > 0 {
                    self.index -= 1;
                }
            }

            KeyCode::Char(' ') => {
                if let Some(p) = self.items.get_mut(self.index) {
                    p.selected = !p.selected;
                }
                self.last_error = None;
            }

            KeyCode::Char('a') => {
                let all_selected = self.items.iter().all(|p| p.selected);
                for p in &mut self.items {
                    p.selected = !all_selected;
                }
                self.last_error = None;
            }

            KeyCode::Char('d') => {
                if self.selected_count() > 0 {
                    self.confirm = true;
                }
            }

            KeyCode::Char('y') => {
                if self.confirm {
                    self.delete_selected();
                    self.confirm = false;
                }
            }

            KeyCode::Char('n') => {
                self.confirm = false;
            }

            _ => {}
        }
        true
    }
}

pub fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    items: Vec<Project>,
    root: String,
) -> io::Result<()> {
    let mut app = App::new(items, root);

    loop {
        terminal.draw(|f| ui(f, &app))?;

        if event::poll(Duration::from_millis(POLL_MS))?
            && let Event::Key(key) = event::read()?
        {
            let cont = if app.input_mode {
                app.handle_key_input_mode(key.code)
            } else {
                app.handle_key_normal(key.code)
            };

            if !cont {
                return Ok(());
            }

            // Keep cursor within visible window after any movement
            if let Ok(size) = terminal.size() {
                let list_height = (size.height as usize).saturating_sub(6);
                let visible = (list_height.saturating_sub(2) / 2).max(1);
                app.ensure_cursor_visible(visible);
            }
        }
    }
}
