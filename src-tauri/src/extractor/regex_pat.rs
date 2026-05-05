use once_cell::sync::Lazy;
use regex::Regex;
use uuid::Uuid;

use crate::models::{FileEntry, LearnedPattern};

// Universal File: pattern regexes for all AI formats
static RE_FILE_DOUBLE_SLASH: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*//\s*[Ff]ile:\s*(.+)$").unwrap());
static RE_FILE_HASH: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*#\s*[Ff]ile:\s*(.+)$").unwrap());
static RE_FILE_C_STYLE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*/\*\s*[Ff]ile:\s*(.+?)\s*\*/\s*$").unwrap());
static RE_FILE_HTML: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*<!--\s*[Ff]ile:\s*(.+?)\s*-->\s*$").unwrap());
static RE_FILE_DASH: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*--\s*[Ff]ile:\s*(.+)$").unwrap());
static RE_FILE_SEMI: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*;\s*[Ff]ile:\s*(.+)$").unwrap());
static RE_FILE_BOLD: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*\*\*[Ff]ile:\*\*\s*(.+)$").unwrap());
static RE_FILE_NAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*[Ff]ile\s*[Nn]ame:\s*(.+)$").unwrap());
static RE_FILE_AT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*@[Ff]ile\s+(.+)$").unwrap());
static RE_FILE_HEADING: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*##\s*[Ff]ile:\s*(.+)$").unwrap());

// Combined pattern to detect any file header line — used to stop content collection
static RE_ANY_FILE_HEADER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^\s*(//|#|/\*|<!--|--|;|\*\*|@|##)\s*[Ff]ile").unwrap()
});

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

            // Collect content after the header line (until next file header, code fence, or end)
            let content = collect_content(&lines, i + 1);
            let name = extract_filename(&clean_path);
            let lang = detect_language_from_path(&clean_path);

            let content_len = content.len() as u64;
            files.push(FileEntry {
                id: Uuid::new_v4().to_string(),
                path: clean_path,
                name,
                language: lang,
                content,
                size: content_len,
                selected: true,
            });
        }
    }

    files
}

/// Extract files using a learned pattern's regex. Used by extractor/mod.rs.
pub fn extract_with_pattern(text: &str, pattern: &LearnedPattern) -> Vec<FileEntry> {
    let mut files = Vec::new();
    if let Ok(re) = Regex::new(&pattern.regex) {
        for cap in re.captures_iter(text) {
            let path = cap.get(pattern.path_group).map(|m| m.as_str().trim().to_string());
            let content = cap.get(pattern.content_group).map(|m| m.as_str().to_string());

            if let (Some(path), Some(content)) = (path, content) {
                let content_trimmed = content.trim().to_string();
                if content_trimmed.is_empty() {
                    continue;
                }
                let lang = pattern.language_hint.clone()
                    .or_else(|| Some(detect_language_from_path(&path)));
                files.push(FileEntry {
                    id: Uuid::new_v4().to_string(),
                    path: path.clone(),
                    name: extract_filename(&path),
                    language: lang.unwrap_or_else(|| "unknown".to_string()),
                    content: content_trimmed,
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

/// Collect content lines starting from `start` index.
/// Stops at: another file header, a code fence marker, or end of lines.
/// Filters content to skip lines that are *only* the file header.
fn collect_content(lines: &[&str], start: usize) -> String {
    let mut content = String::new();
    let mut in_content = false;

    for line in lines.iter().skip(start) {
        // Stop at code fences
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            // If we already have content, stop; otherwise skip the fence marker and continue
            if in_content {
                break;
            }
            continue;
        }

        // Stop at next file header
        if RE_ANY_FILE_HEADER.is_match(line) {
            if in_content || !content.trim().is_empty() {
                break;
            }
            continue;
        }

        in_content = true;
        content.push_str(line);
        content.push('\n');
    }

    content.trim().to_string()
}

pub fn detect_language_from_path(path: &str) -> String {
    let ext = path.rsplit('.').next().map(|s| s.to_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "tsx" => "tsx",
        "jsx" => "jsx",
        "go" => "go",
        "rb" => "ruby",
        "c" | "h" => "c",
        "cpp" | "hpp" | "cc" | "cxx" => "cpp",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "cs" => "csharp",
        "php" => "php",
        "vue" => "vue",
        "svelte" => "svelte",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "md" | "markdown" => "markdown",
        "sql" => "sql",
        "sh" | "bash" | "zsh" => "bash",
        "ps1" => "powershell",
        "bat" | "cmd" => "batch",
        "dockerfile" => "dockerfile",
        "ini" | "cfg" => "ini",
        "env" => "env",
        "txt" => "text",
        "xml" => "xml",
        "svg" => "svg",
        "css" => "css",
        "scss" | "sass" => "scss",
        "less" => "less",
        "html" | "htm" => "html",
        "r" => "r",
        "lua" => "lua",
        "pl" => "perl",
        "ex" | "exs" => "elixir",
        "clj" | "cljs" | "edn" => "clojure",
        "zig" => "zig",
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
