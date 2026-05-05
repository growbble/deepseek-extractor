use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub id: String,
    pub path: String,
    pub name: String,
    pub language: String,
    pub content: String,
    pub size: u64,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub files: Vec<FileEntry>,
    pub unparsed: String,
    pub source: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    pub context_before: String,
    pub header_line: String,
    pub file_entry: FileEntry,
    pub user_corrected: bool,
    pub features: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub regex: String,
    pub path_group: usize,
    pub content_group: usize,
    pub language_hint: Option<String>,
    pub confidence: f64,
    pub usage_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningModel {
    pub patterns: Vec<LearnedPattern>,
    pub feature_weights: Vec<f64>,
    pub training_examples: Vec<TrainingExample>,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveInfo {
    pub file_count: u32,
    pub total_original: u64,
    pub total_compressed: u64,
    pub entries: Vec<ArchiveEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub name: String,
    pub original_size: u64,
    pub compressed_size: u64,
}
