use std::{fs, path::PathBuf};

pub fn compute_suggestions(input: &str) -> Vec<String> {
    // ホームディレクトリの展開
    let expanded = if let Some(stripped) = input.strip_prefix('~') {
        if let Some(home) = dirs_next::home_dir() {
            home.to_string_lossy().to_string() + stripped
        } else {
            input.to_string()
        }
    } else {
        input.to_string()
    };

    // ディレクトリとプレフィックスの分割
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

    // ディレクトリが存在しない場合は空の候補を返す
    let Ok(entries) = fs::read_dir(&dir_to_list) else {
        return vec![];
    };

    // ディレクトリ内のエントリをフィルタリングして、プレフィックスにマッチするディレクトリを収集
    let mut suggestions: Vec<String> = entries
        .filter_map(|e| e.ok()) // 読み取りエラーを無視
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false)) // ディレクトリのみ
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string(); // ファイル名を文字列に変換
            !name.starts_with('.') && name.starts_with(&prefix) // プレフィックスにマッチし、隠しファイルでないもの
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
