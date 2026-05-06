use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::models::{FileEntry, LearnedPattern, LearningModel, TrainingExample};

const KNOWN_EXTENSIONS: &[&str] = &[
    ".rs", ".py", ".js", ".ts", ".go", ".rb", ".c", ".cpp", ".h", ".hpp",
    ".java", ".kt", ".swift", ".cs", ".php", ".vue", ".svelte", ".tsx", ".jsx",
    ".json", ".yaml", ".yml", ".toml", ".md", ".sql", ".sh", ".bash", ".ps1",
    ".bat", ".dockerfile", ".ini", ".cfg", ".env", ".txt", ".xml", ".svg",
    ".css", ".scss", ".less", ".html", ".r", ".lua", ".pl", ".ex", ".exs",
    ".clj", ".cljs", ".zig",
];

fn extract_features(line: &str, position_normalized: f64) -> Vec<f64> {
    let length = line.len() as f64;
    let has_comment = if line.contains("//") || line.starts_with('#') || line.contains("/*")
        || line.contains("--") || line.starts_with(';')
    {
        1.0
    } else {
        0.0
    };

    let has_file_keyword = if line.contains("File:") || line.contains("file:")
        || line.contains("path:") || line.contains("name:")
        || line.contains("@file") || line.contains("// File")
    {
        1.0
    } else {
        0.0
    };

    let has_path_separator = if line.contains('/') || line.contains('\\') {
        1.0
    } else {
        0.0
    };

    let spec_ratio = if length > 0.0 {
        line.chars().filter(|c| !c.is_alphanumeric() && !c.is_whitespace()).count() as f64 / length
    } else {
        0.0
    };

    let has_known_extension = if KNOWN_EXTENSIONS.iter().any(|ext| line.contains(ext)) {
        1.0
    } else {
        0.0
    };

    let not_empty = if line.trim().is_empty() { 0.0 } else { 1.0 };

    vec![
        (length.min(200.0) / 200.0).clamp(0.0, 1.0),
        has_comment,
        has_file_keyword,
        has_path_separator,
        spec_ratio.min(1.0),
        has_known_extension,
        not_empty,
        position_normalized.clamp(0.0, 1.0),
    ]
}

pub(crate) fn classify(features: &[f64], weights: &[f64]) -> bool {
    if features.len() != weights.len() || features.is_empty() || weights.is_empty() {
        return false;
    }
    let threshold = 0.5;
    let sum: f64 = features.iter()
        .zip(weights.iter())
        .map(|(f, w)| f * w)
        .sum();
    // Clip to reasonable range to prevent NaN
    sum > threshold
}

pub(crate) fn train(examples: &[TrainingExample]) -> Vec<f64> {
    if examples.is_empty() {
        return vec![0.125; 8];
    }

    let mut weights = vec![0.125; 8];
    let learning_rate = 0.01;
    let epochs = 100;

    for _epoch in 0..epochs {
        for example in examples {
            // Validate feature vector
            if example.features.len() != weights.len() {
                continue;
            }
            let prediction = classify(&example.features, &weights);
            let target = if example.user_corrected { 1.0 } else { 0.0 };
            let error: f64 = target - if prediction { 1.0 } else { 0.0 };

            if error.abs() > 0.01 {
                for (w, f) in weights.iter_mut().zip(example.features.iter()) {
                    *w += learning_rate * error * f;
                    // Clip weights to prevent divergence
                    *w = w.clamp(-5.0, 5.0);
                }
            }
        }
    }

    weights
}

/// Load model from disk atomically.
pub(crate) fn load_model(app: &AppHandle) -> LearningModel {
    let model_path = get_model_path(app);
    if model_path.exists() {
        match std::fs::read_to_string(&model_path) {
            Ok(content) => {
                serde_json::from_str(&content).unwrap_or_else(|e| {
                    eprintln!("Failed to parse learning model, using defaults: {}", e);
                    default_model()
                })
            }
            Err(e) => {
                eprintln!("Failed to read learning model, using defaults: {}", e);
                default_model()
            }
        }
    } else {
        default_model()
    }
}

/// Save model atomically: write to temp file, then rename.
pub(crate) fn save_model(app: &AppHandle, model: &LearningModel) -> Result<(), String> {
    let model_path = get_model_path(app);
    if let Some(parent) = model_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create model dir: {}", e))?;
    }

    // Atomic write: write to .tmp then rename
    let tmp_path = model_path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(model)
        .map_err(|e| format!("Failed to serialize model: {}", e))?;
    std::fs::write(&tmp_path, &content)
        .map_err(|e| format!("Failed to write model: {}", e))?;
    std::fs::rename(&tmp_path, &model_path)
        .map_err(|e| format!("Failed to rename model file: {}", e))?;

    Ok(())
}

fn get_model_path(app: &AppHandle) -> PathBuf {
    let path = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    path.join("models").join("learning_model.json")
}

pub(crate) fn default_model() -> LearningModel {
    LearningModel {
        patterns: vec![
            LearnedPattern {
                regex: r"(?m)^\s*//\s*[Ff]ile:\s*(.+)$".to_string(),
                path_group: 1,
                content_group: 0,
                language_hint: None,
                confidence: 0.9,
                usage_count: 0,
            },
            LearnedPattern {
                regex: r"(?m)^\s*#\s*[Ff]ile:\s*(.+)$".to_string(),
                path_group: 1,
                content_group: 0,
                language_hint: None,
                confidence: 0.9,
                usage_count: 0,
            },
            LearnedPattern {
                regex: r"\*\*[Ff]ile:\*\*\s*(.+?)(?:\n|$)".to_string(),
                path_group: 1,
                content_group: 0,
                language_hint: None,
                confidence: 0.7,
                usage_count: 0,
            },
        ],
        feature_weights: vec![0.125; 8],
        training_examples: Vec::new(),
        version: 1,
    }
}

/// Add a training example, deduplicate, and retrain when enough examples.
pub(crate) fn add_training_example(
    context_before: &str,
    header_line: &str,
    file_entry: &FileEntry,
    user_corrected: bool,
    model: &mut LearningModel,
) {
    let features = extract_features(header_line, 0.5);

    // Deduplicate: check if identical example already exists
    let is_duplicate = model.training_examples.iter().any(|e|
        e.header_line == header_line
        && e.context_before == context_before
        && e.user_corrected == user_corrected
    );
    if is_duplicate {
        return;
    }

    let example = TrainingExample {
        context_before: context_before.to_string(),
        header_line: header_line.to_string(),
        file_entry: file_entry.clone(),
        user_corrected,
        features,
    };

    const MAX_EXAMPLES: usize = 1000;
    model.training_examples.push(example);
    if model.training_examples.len() > MAX_EXAMPLES {
        model.training_examples.remove(0);
    }

    // Retrain when we have enough examples
    if model.training_examples.len() >= 5 {
        model.feature_weights = train(&model.training_examples);
    }
}
