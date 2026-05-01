use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};

use thiserror::Error;
use walkdir::WalkDir;

use crate::model::Project;

#[derive(Debug, Error)]
pub enum ScannerError {
    #[error("Invalid project directory: {0}")]
    InvalidProjectDir(String),
}

pub type ScannerResult<T> = Result<T, ScannerError>;

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

/// Calculates the total size of files in a directory.
///
/// This function silently ignores any directory access errors (permission denied, deleted files, etc.)
/// and sums up the sizes of files that can be accessed. This error-tolerant approach ensures
/// that scanning can proceed even when some files or subdirectories are inaccessible, providing
/// a best-effort size estimate rather than failing completely.
pub fn dir_size(path: &Path) -> ScannerResult<u64> {
    let size = WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum();
    Ok(size)
}

fn project_last_modified(project_dir: &Path) -> SystemTime {
    // Helper to find the most recent modification time among all accessible files.
    // Errors during traversal are silently ignored to ensure partial results are returned.
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

fn get_project_dir(entry_path: &Path) -> ScannerResult<&Path> {
    entry_path
        .parent()
        .ok_or_else(|| ScannerError::InvalidProjectDir(entry_path.display().to_string()))
}

/// Scans a directory tree to find Python projects with unused virtual environments.
///
/// # Design Notes
/// This function uses an error-tolerant approach:
/// - Inaccessible directories are silently skipped (e.g., permission denied, deleted files)
/// - Scanning continues even if some files cannot be read
/// - Only successfully accessed `.venv` directories with modification times older than `days` are included
///
/// This design ensures the scanner provides useful results even in environments with permission
/// restrictions or changing file systems, rather than failing completely on partial access issues.
pub fn scan_projects(root: &Path, days: u64) -> ScannerResult<Vec<Project>> {
    let mut projects = vec![];
    let threshold = SystemTime::now() - Duration::from_secs(days * 86400);

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_name() == "pyproject.toml" {
            let project_dir = match get_project_dir(entry.path()) {
                Ok(dir) => dir,
                Err(_) => continue,
            };
            let venv = project_dir.join(".venv");

            if venv.exists()
                && let Ok(meta) = fs::metadata(&venv)
                && let Ok(venv_mtime) = meta.modified()
                && venv_mtime < threshold
            {
                let venv_size = dir_size(&venv).unwrap_or(0);
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
    Ok(projects)
}
