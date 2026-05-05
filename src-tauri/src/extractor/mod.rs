pub mod regex_pat;
pub mod markdown;
pub mod learning;

use crate::models::{ExtractResult, FileEntry, LearningModel};
use regex_pat::extract_with_regex;
use markdown::extract_markdown_blocks;

pub fn extract(text: &str, model: &LearningModel) -> ExtractResult {
    let mut files: Vec<FileEntry> = Vec::new();
    let mut found_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 1. Try regex patterns first (most reliable)
    let regex_files = extract_with_regex(text);
    for f in regex_files {
        if found_ids.insert(f.id.clone()) {
            files.push(f);
        }
    }

    // 2. Try learned patterns
    for pattern in &model.patterns {
        let learned_files = regex_pat::extract_with_pattern(text, pattern);
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

fn build_unparsed_text(text: &str, files: &[FileEntry]) -> String {
    let mut result = text.to_string();
    for f in files {
        result = result.replace(&f.content, "");
    }
    result.trim().to_string()
}

pub fn extract_from_text(text: &str, source: &str, model: &LearningModel) -> ExtractResult {
    let mut result = extract(text, model);
    result.source = source.to_string();
    result
}
