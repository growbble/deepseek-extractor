use std::sync::LazyLock;
use regex::Regex;
use uuid::Uuid;

use crate::models::FileEntry;

// Universal File: pattern regexes for all AI formats
static RE_FILE_DOUBLE_SLASH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*//\s*[Ff]ile:\s*(.+)$").unwrap());
static RE_FILE_HASH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*#\s*[Ff]ile:\s*(.+)$").unwrap());
static RE_FILE_C_STYLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*/\*\s*[Ff]ile:\s*(.+?)\s*\*/\s*$").unwrap());
static RE_FILE_HTML: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*<!--\s*[Ff]ile:\s*(.+?)\s*-->\s*$").unwrap());
static RE_FILE_DASH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*--\s*[Ff]ile:\s*(.+)$").unwrap());
static RE_FILE_SEMI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*;\s*[Ff]ile:\s*(.+)$").unwrap());
static RE_FILE_BOLD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*\*\*[Ff]ile:\*\*\s*(.+)$").unwrap());
static RE_FILE_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*[Ff]ile\s*[Nn]ame:\s*(.+)$").unwrap());
static RE_FILE_AT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*@[Ff]ile\s+(.+)$").unwrap());
static RE_FILE_HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\s*##\s*[Ff]ile:\s*(.+)$").unwrap());

// Combined pattern to detect any file header line — used to stop content collection
static RE_ANY_FILE_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(//|#|/\*|<!--|--|;|\*\*|@|##)\s*[Ff]ile").unwrap()
});

/// Extract files from text using regex patterns for `File: path` headers.
/// Each matched header is followed by content collection until the next header or code fence.
pub(crate) fn extract_with_regex(text: &str) -> Vec<FileEntry> {
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

            // Collect content after the header line
            let content = collect_content(&lines, i + 1);
            let name = extract_filename(&clean_path);
            let lang = detect_language_from_path(&clean_path);

            let content_len: u64 = content.len().try_into().unwrap_or(0);
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

pub(crate) fn detect_language_from_path(path: &str) -> String {
    let ext = path.rsplit('.').next().map(|s| s.to_lowercase()).unwrap_or_default();
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

pub(crate) fn extract_filename(path: &str) -> String {
    if let Some(idx) = path.rfind('/') {
        path[idx + 1..].to_string()
    } else if let Some(idx) = path.rfind('\\') {
        path[idx + 1..].to_string()
    } else {
        path.to_string()
    }
}

pub(crate) fn language_to_extension(language: &str) -> &str {
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
