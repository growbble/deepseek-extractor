use once_cell::sync::Lazy;
use regex::Regex;
use uuid::Uuid;

use crate::models::FileEntry;
use crate::extractor::regex_pat::{detect_language_from_path, language_to_extension, extract_filename};

static RE_MD_BLOCK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?ms)^```(\w+)?(?::(.+?))?\s*\n(.*?)```").unwrap()
});
static RE_TILDE_BLOCK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?ms)^~~~(\w+)?(?::(.+?))?\s*\n(.*?)~~~").unwrap()
});

pub fn extract_markdown_blocks(text: &str) -> Vec<FileEntry> {
    let mut files: Vec<FileEntry> = Vec::new();
    let mut seen_content: std::collections::HashSet<String> = std::collections::HashSet::new();

    let all_lines: Vec<&str> = text.lines().collect();

    // Extract from ``` blocks
    for cap in RE_MD_BLOCK.captures_iter(text) {
        let lang_hint = cap.get(1).map(|m| m.as_str().to_lowercase());
        let path_hint = cap.get(2).map(|m| m.as_str().trim().to_string());
        let content = cap.get(3).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

        if content.is_empty() {
            continue;
        }

        let content_trimmed = content.trim().to_string();
        if content_trimmed.is_empty() || seen_content.contains(&content_trimmed) {
            continue;
        }
        seen_content.insert(content_trimmed.clone());

        let (path, name, language) = resolve_file_info(&path_hint, &lang_hint, &content_trimmed, &all_lines);

        files.push(FileEntry {
            id: Uuid::new_v4().to_string(),
            path,
            name,
            language,
            content: content_trimmed,
            size: content.len() as u64,
            selected: true,
        });
    }

    // Extract from ~~~ blocks
    for cap in RE_TILDE_BLOCK.captures_iter(text) {
        let lang_hint = cap.get(1).map(|m| m.as_str().to_lowercase());
        let path_hint = cap.get(2).map(|m| m.as_str().trim().to_string());
        let content = cap.get(3).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

        if content.is_empty() {
            continue;
        }

        let content_trimmed = content.trim().to_string();
        if content_trimmed.is_empty() || seen_content.contains(&content_trimmed) {
            continue;
        }
        seen_content.insert(content_trimmed.clone());

        let (path, name, language) = resolve_file_info(&path_hint, &lang_hint, &content_trimmed, &all_lines);

        files.push(FileEntry {
            id: Uuid::new_v4().to_string(),
            path,
            name,
            language,
            content: content_trimmed,
            size: content.len() as u64,
            selected: true,
        });
    }

    files
}

fn resolve_file_info(
    path_hint: &Option<String>,
    lang_hint: &Option<String>,
    content: &str,
    _all_lines: &[&str],
) -> (String, String, String) {
    // Try to find File: header in the lines before this block
    // Priority 1: Path from ```language:path annotation
    if let Some(path) = path_hint {
        let name = extract_filename(path);
        let lang = detect_language_from_path(path);
        return (path.clone(), name, lang);
    }

    // Priority 2: Language from block annotation + generate default name
    if let Some(lang) = lang_hint {
        let ext = language_to_extension(&lang);
        let name = format!("main.{}", ext);
        let path = name.clone();
        return (path, name, lang.clone());
    }

    // Priority 3: Try to detect from content
    let detected_lang = detect_language_from_content(content);
    let ext = language_to_extension(&detected_lang);
    let name = format!("code.{}", ext);
    let path = name.clone();

    (path, name, detected_lang)
}

fn detect_language_from_content(content: &str) -> String {
    let first_line = content.lines().next().unwrap_or("").trim();

    if first_line.starts_with("fn ") || first_line.starts_with("pub ") || content.contains("fn main(") {
        return "rust".to_string();
    }
    if first_line.starts_with("import ") || first_line.starts_with("def ") || first_line.starts_with("class ") {
        if content.contains(":") && !content.contains("->") {
            return "python".to_string();
        }
    }
    if content.contains("fn ") && (content.contains("let ") || content.contains("mut ")) {
        // Could be Rust or JS/TS — check for types
        if content.contains(": String") || content.contains("-> ") {
            return "rust".to_string();
        }
    }
    if content.contains("function ") || content.contains("const ") || content.contains("let ") {
        return "javascript".to_string();
    }
    if content.contains("package ") || content.contains("import ") && content.contains("\"") {
        return "go".to_string();
    }
    if content.contains("#include") {
        return "c".to_string();
    }
    if first_line.starts_with("<!DOCTYPE") || first_line.starts_with("<html") {
        return "html".to_string();
    }
    if first_line.starts_with("{") || first_line.starts_with("[") {
        // Check if it's valid JSON
        if content.starts_with("{") && content.contains("\"") {
            return "json".to_string();
        }
    }

    "text".to_string()
}
