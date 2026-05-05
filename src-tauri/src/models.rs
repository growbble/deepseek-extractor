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

impl FileEntry {
    /// Validate entry — reject path traversal attempts
    pub fn is_safe_path(&self) -> bool {
        !self.path.contains("..") && !self.path.starts_with('/') && !self.path.starts_with('\\')
    }

    /// Sanitize path for display (strip null bytes, control chars)
    pub fn sanitize_path(&self) -> String {
        self.path.chars().filter(|&c| c != '\0' && !c.is_control() || c == '/' || c == '.' || c == '_' || c == '-').collect()
    }
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
