use std::{fs, path::PathBuf};

pub fn compute_suggestions(input: &str) -> Vec<String> {
    let expanded = if let Some(stripped) = input.strip_prefix('~') {
        if let Some(home) = dirs_next::home_dir() {
            home.to_string_lossy().to_string() + stripped
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
            !name.starts_with('.') && name.starts_with(&prefix)
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
