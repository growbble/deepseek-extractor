pub mod regex_pat;
pub mod markdown;
pub mod learning;

use std::collections::HashSet;
use crate::models::{ExtractResult, FileEntry, LearningModel};
use regex_pat::extract_with_regex;
use markdown::extract_markdown_blocks;

pub fn extract(text: &str, model: &LearningModel) -> ExtractResult {
    let mut files: Vec<FileEntry> = Vec::new();
    let mut found_ids: HashSet<String> = HashSet::new();

    // 1. Try regex patterns first (most reliable)
    let regex_files = extract_with_regex(text);
    for f in regex_files {
        if found_ids.insert(f.id.clone()) {
            files.push(f);
        }
    }

    // 2. Try learned patterns
    for pattern in &model.patterns {
        let learned_files = extract_with_pattern(text, pattern);
        for f in learned_files {
            if found_ids.insert(f.id.clone()) {
                files.push(f);
            }
        }
    }

    // 3. Try markdown code blocks
    let md_files = extract_markdown_blocks(text);
    for f in md_files {
        if found_ids.insert(f.id.clone()) {
            files.push(f);
        }
    }

    // 4. Build unparsed text (text outside code blocks)
    let unparsed = build_unparsed_text(text, &files);

    let confidence = if files.is_empty() {
        0.0
    } else {
        let base = 0.5;
        let bonus = (files.len() as f64).min(10.0) / 20.0;
        let has_paths = files.iter().filter(|f| f.path.len() > f.name.len()).count() as f64 / files.len() as f64 * 0.3;
        (base + bonus + has_paths).min(1.0)
    };

    ExtractResult {
        files,
        unparsed,
        source: "clipboard".to_string(),
        confidence,
    }
}

/// Remove all file contents from the original text, collecting what remains.
/// Uses repeated replace to handle multiple occurrences of the same content.
fn build_unparsed_text(text: &str, files: &[FileEntry]) -> String {
    let mut result = text.to_string();
    for f in files {
        // Replace only one occurrence at a time to avoid touching duplicate filenames in non-content
        result = result.replacen(&f.content, "", result.len().min(f.content.len()));
    }
    let trimmed = result.trim().to_string();
    // If nothing left or only whitespace, return empty
    if trimmed.is_empty() || trimmed.chars().all(|c| c.is_whitespace()) {
        String::new()
    } else {
        trimmed
    }
}

pub fn extract_from_text(text: &str, source: &str, model: &LearningModel) -> ExtractResult {
    let mut result = extract(text, model);
    result.source = source.to_string();
    result
}

/// Extract files using a learned pattern's regex.
/// Used by both the cascade in extract() and the learning module.
pub fn extract_with_pattern(text: &str, pattern: &crate::models::LearnedPattern) -> Vec<FileEntry> {
    use uuid::Uuid;
    use crate::extractor::regex_pat::{detect_language_from_path, extract_filename};

    let mut files = Vec::new();
    if let Ok(re) = regex::Regex::new(&pattern.regex) {
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
