use serde::{Deserialize, Serialize};

/// Maximum content size for a single file entry (10 MB)
pub(crate) const MAX_FILE_CONTENT_SIZE: u64 = 10 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileEntry {
    pub id: String,
    pub path: String,
    pub name: String,
    pub language: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub content: String,
    pub size: u64,
    #[serde(default = "default_selected")]
    pub selected: bool,
}

const fn default_selected() -> bool {
    true
}

/// Critical files that should never be overwritten by save operations.
const CRITICAL_FILES: &[&str] = &[
    ".env", ".git", "config", ".gitignore", ".gitattributes",
    ".ssh", "authorized_keys", "id_rsa", "id_rsa.pub",
];

impl FileEntry {
    /// Validate entry — reject path traversal, dangerous paths, and oversized content.
    /// Returns `true` if the path is safe to write.
    pub(crate) fn is_safe_path(&self) -> bool {
        // Null bytes
        if self.path.contains('\0') {
            return false;
        }
        // Control characters (except / . _ -)
        if self.path.bytes().any(|b| b > 0 && b < 32 && b != b'/' && b != b'.' && b != b'-' && b != b'_') {
            return false;
        }
        // Path traversal
        if self.path.contains("..") {
            return false;
        }
        // Absolute paths
        if self.path.starts_with('/') || self.path.starts_with('\\') {
            return false;
        }
        // Windows reserved names
        let stem = self.path.rsplit('/').next().unwrap_or(&self.path);
        let stem_lower = stem.to_lowercase();
        let without_ext = stem_lower.rsplit('.').next().unwrap_or(&stem_lower);
        if ["con", "prn", "aux", "nul", "com1", "com2", "com3", "com4",
            "lpt1", "lpt2", "lpt3", "lpt4"].contains(&without_ext) {
            return false;
        }
        // Critical files protection
        let lower = self.path.to_lowercase();
        if CRITICAL_FILES.iter().any(|f| lower == *f || lower.ends_with(&format!("/{}", f))) {
            return false;
        }
        // Check content size limit
        if self.content.len() as u64 > MAX_FILE_CONTENT_SIZE {
            return false;
        }
        true
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExtractResult {
    pub files: Vec<FileEntry>,
    pub unparsed: String,
    pub source: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TrainingExample {
    #[serde(default)]
    pub context_before: String,
    pub header_line: String,
    pub file_entry: FileEntry,
    #[serde(default)]
    pub user_corrected: bool,
    #[serde(default)]
    pub features: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LearnedPattern {
    pub regex: String,
    pub path_group: usize,
    pub content_group: usize,
    pub language_hint: Option<String>,
    pub confidence: f64,
    #[serde(default)]
    pub usage_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LearningModel {
    pub patterns: Vec<LearnedPattern>,
    pub feature_weights: Vec<f64>,
    pub training_examples: Vec<TrainingExample>,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArchiveInfo {
    pub file_count: u32,
    pub total_original: u64,
    pub total_compressed: u64,
    pub entries: Vec<ArchiveEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArchiveEntry {
    pub name: String,
    pub original_size: u64,
    pub compressed_size: u64,
}
