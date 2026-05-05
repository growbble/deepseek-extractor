use once_cell::sync::Lazy;
use regex::Regex;
use uuid::Uuid;

use crate::models::{FileEntry, LearnedPattern};

// Universal File: pattern regexes for all AI formats
static RE_FILE_DOUBLE_SLASH: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*//\s*File:\s*(.+)$").unwrap());
static RE_FILE_HASH: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*#\s*File:\s*(.+)$").unwrap());
static RE_FILE_C_STYLE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*/\*\s*File:\s*(.+?)\s*\*/\s*$").unwrap());
static RE_FILE_HTML: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*<!--\s*File:\s*(.+?)\s*-->\s*$").unwrap());
static RE_FILE_DASH: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*--\s*File:\s*(.+)$").unwrap());
static RE_FILE_SEMI: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*;\s*File:\s*(.+)$").unwrap());
static RE_FILE_BOLD: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*\*\*File:\*\*\s*(.+)$").unwrap());
static RE_FILE_NAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*File\s*name:\s*(.+)$").unwrap());
static RE_FILE_AT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*@file\s+(.+)$").unwrap());
static RE_FILE_HEADING: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*##\s*File:\s*(.+)$").unwrap());

pub fn extract_with_regex(text: &str) -> Vec<FileEntry> {
    let mut files: Vec<FileEntry> = Vec::new();
    let mut seen_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    let lines: Vec<&str> = text.lines().collect();

    let patterns: Vec<&Regex> = vec![
        &RE_FILE_DOUBLE_SLASH,
        &RE_FILE_HASH,
        &RE_FILE_C_STYLE,
        &RE_FILE_HTML,
        &RE_FILE_DASH,
        &RE_FILE_SEMI,
        &RE_FILE_BOLD,
        &RE_FILE_NAME,
        &RE_FILE_AT,
        &RE_FILE_HEADING,
    ];

    for (i, line) in lines.iter().enumerate() {
        let path_opt = find_path_in_line(line, &patterns);
        if let Some(path) = path_opt {
            let clean_path = path.trim().to_string();
            if clean_path.is_empty() || seen_paths.contains(&clean_path) {
                continue;
            }
            seen_paths.insert(clean_path.clone());

            // Collect content after the header line (until next header or end)
            let content = collect_content(&lines, i + 1);
            let name = extract_filename(&clean_path);
            let lang = detect_language_from_path(&clean_path);

            files.push(FileEntry {
                id: Uuid::new_v4().to_string(),
                path: clean_path,
                name,
                language: lang,
                content,
                size: content.len() as u64,
                selected: true,
            });
        }
    }

    files
}

pub fn extract_with_pattern(text: &str, pattern: &LearnedPattern) -> Vec<FileEntry> {
    let mut files = Vec::new();
    if let Ok(re) = Regex::new(&pattern.regex) {
        for cap in re.captures_iter(text) {
            let path = cap.get(pattern.path_group).map(|m| m.as_str().trim().to_string());
            let content = cap.get(pattern.content_group).map(|m| m.as_str().to_string());

            if let (Some(path), Some(content)) = (path, content) {
                let lang = pattern.language_hint.clone()
                    .or_else(|| Some(detect_language_from_path(&path)));
                files.push(FileEntry {
                    id: Uuid::new_v4().to_string(),
                    path: path.clone(),
                    name: extract_filename(&path),
                    language: lang.unwrap_or_else(|| "unknown".to_string()),
                    content,
                    size: content.len() as u64,
                    selected: true,
                });
            }
        }
    }
    files
}

fn find_path_in_line<'a>(line: &str, patterns: &[&Regex]) -> Option<String> {
    for re in patterns {
        if let Some(cap) = re.captures(line) {
            if let Some(m) = cap.get(1) {
                return Some(m.as_str().trim().to_string());
            }
        }
    }
    None
}

fn collect_content(lines: &[&str], start: usize) -> String {
    let mut content = String::new();
    for line in lines.iter().skip(start) {
        // Stop at next File: header
        if line.contains("File:") || line.contains("File:") || line.contains("@file")
            || line.starts_with("```") || line.starts_with("~~~")
        {
            if content.len() > 0 && !content.trim().is_empty() {
                break;
            }
        }
        content.push_str(line);
        content.push('\n');
    }
    content.trim().to_string()
}

pub fn detect_language_from_path(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => "rust".to_string(),
        "py" => "python".to_string(),
        "js" => "javascript".to_string(),
        "ts" => "typescript".to_string(),
        "tsx" => "tsx".to_string(),
        "jsx" => "jsx".to_string(),
        "go" => "go".to_string(),
        "rb" => "ruby".to_string(),
        "c" | "h" => "c".to_string(),
        "cpp" | "hpp" | "cc" | "cxx" => "cpp".to_string(),
        "java" => "java".to_string(),
        "kt" | "kts" => "kotlin".to_string(),
        "swift" => "swift".to_string(),
        "cs" => "csharp".to_string(),
        "php" => "php".to_string(),
        "vue" => "vue".to_string(),
        "svelte" => "svelte".to_string(),
        "json" => "json".to_string(),
        "yaml" | "yml" => "yaml".to_string(),
        "toml" => "toml".to_string(),
        "md" | "markdown" => "markdown".to_string(),
        "sql" => "sql".to_string(),
        "sh" | "bash" | "zsh" => "bash".to_string(),
        "ps1" => "powershell".to_string(),
        "bat" | "cmd" => "batch".to_string(),
        "dockerfile" => "dockerfile".to_string(),
        "ini" | "cfg" => "ini".to_string(),
        "env" => "env".to_string(),
        "txt" => "text".to_string(),
        "xml" => "xml".to_string(),
        "svg" => "svg".to_string(),
        "css" => "css".to_string(),
        "scss" | "sass" => "scss".to_string(),
        "less" => "less".to_string(),
        "html" | "htm" => "html".to_string(),
        "r" => "r".to_string(),
        "lua" => "lua".to_string(),
        "pl" => "perl".to_string(),
        "ex" | "exs" => "elixir".to_string(),
        "clj" | "cljs" | "edn" => "clojure".to_string(),
        "zig" => "zig".to_string(),
        _ => ext,
    }
}

pub fn extract_filename(path: &str) -> String {
    if let Some(idx) = path.rfind('/') {
        path[idx + 1..].to_string()
    } else if let Some(idx) = path.rfind('\\') {
        path[idx + 1..].to_string()
    } else {
        path.to_string()
    }
}

pub fn language_to_extension(language: &str) -> &str {
    match language.to_lowercase().as_str() {
        "rust" => "rs",
        "python" => "py",
        "javascript" => "js",
        "typescript" => "ts",
        "tsx" => "tsx",
        "jsx" => "jsx",
        "go" => "go",
        "ruby" => "rb",
        "c" => "c",
        "cpp" => "cpp",
        "java" => "java",
        "kotlin" => "kt",
        "swift" => "swift",
        "csharp" => "cs",
        "php" => "php",
        "vue" => "vue",
        "svelte" => "svelte",
        "json" => "json",
        "yaml" => "yaml",
        "toml" => "toml",
        "markdown" | "md" => "md",
        "sql" => "sql",
        "bash" | "shell" | "sh" => "sh",
        "powershell" => "ps1",
        "batch" => "bat",
        "dockerfile" => "dockerfile",
        "ini" => "ini",
        "css" => "css",
        "scss" => "scss",
        "less" => "less",
        "html" => "html",
        "xml" => "xml",
        "svg" => "svg",
        "r" => "r",
        "lua" => "lua",
        "perl" => "pl",
        "elixir" => "ex",
        "clojure" => "clj",
        "zig" => "zig",
        "text" | "plain" => "txt",
        _ => "txt",
    }
}
