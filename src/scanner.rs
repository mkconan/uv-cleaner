use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};

use walkdir::WalkDir;

use crate::model::Project;

pub const SCAN_DAYS: u64 = 30;

const SKIP_DIRS: &[&str] = &[
    ".git",
    ".venv",
    "__pycache__",
    "node_modules",
    "target",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
];

pub fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

fn project_last_modified(project_dir: &Path) -> SystemTime {
    WalkDir::new(project_dir)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                !SKIP_DIRS.iter().any(|s| name == *s)
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

pub fn scan_projects(root: &Path, days: u64) -> Vec<Project> {
    let mut projects = vec![];
    let threshold = SystemTime::now() - Duration::from_secs(days * 86400);

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_name() == "pyproject.toml" {
            let project_dir = entry.path().parent().unwrap();
            let venv = project_dir.join(".venv");

            if venv.exists()
                && let Ok(meta) = fs::metadata(&venv)
                && let Ok(venv_mtime) = meta.modified()
                && venv_mtime < threshold
            {
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

    // 古い順（プロジェクト全体の最終更新日が古い順）にソート
    projects.sort_by_key(|a| a.last_modified);
    projects
}
