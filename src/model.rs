use std::{path::PathBuf, time::SystemTime};

#[derive(Clone)]
pub struct Project {
    pub path: PathBuf,
    pub venv_path: PathBuf,
    pub last_modified: SystemTime,
    pub venv_size: u64,
    pub selected: bool,
}

pub struct App {
    pub items: Vec<Project>,
    pub index: usize,
    pub confirm: bool,
    pub input: String,
    pub input_mode: bool,
    pub suggestions: Vec<String>,
    pub suggestion_index: Option<usize>,
    pub suggestion_scroll_offset: usize,
    pub scroll_offset: usize,
    pub last_error: Option<String>,
}
