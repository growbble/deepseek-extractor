use std::path::PathBuf;
use tauri::AppHandle;

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
        length.min(200.0) / 200.0,
        has_comment,
        has_file_keyword,
        has_path_separator,
        spec_ratio.min(1.0),
        has_known_extension,
        not_empty,
        position_normalized,
    ]
}

pub fn classify(features: &[f64], weights: &[f64]) -> bool {
    if features.len() != weights.len() {
        return false;
    }
    let threshold = 0.5;
    let sum: f64 = features.iter()
        .zip(weights.iter())
        .map(|(f, w)| f * w)
        .sum();
    sum > threshold
}

pub fn train(examples: &[TrainingExample]) -> Vec<f64> {
    // Simple perceptron-like training
    let mut weights = vec![0.125; 8]; // uniform starting weights
    let learning_rate = 0.01;
    let epochs = 100;

    for _epoch in 0..epochs {
        for example in examples {
            let prediction = classify(&example.features, &weights);
            let target = example.user_corrected as usize as f64;
            let error = target - if prediction { 1.0 } else { 0.0 };

            if error.abs() > 0.01 {
                for (w, f) in weights.iter_mut().zip(example.features.iter()) {
                    *w += learning_rate * error * f;
                }
            }
        }
    }

    weights
}

pub fn load_model(app: &AppHandle) -> LearningModel {
    let model_path = get_model_path(app);
    if model_path.exists() {
        match std::fs::read_to_string(&model_path) {
            Ok(content) => {
                serde_json::from_str(&content).unwrap_or_else(|_| default_model())
            }
            Err(_) => default_model(),
        }
    } else {
        default_model()
    }
}

pub fn save_model(app: &AppHandle, model: &LearningModel) -> Result<(), String> {
    let model_path = get_model_path(app);
    if let Some(parent) = model_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create model dir: {}", e))?;
    }
    let content = serde_json::to_string_pretty(model)
        .map_err(|e| format!("Failed to serialize model: {}", e))?;
    std::fs::write(&model_path, content)
        .map_err(|e| format!("Failed to write model: {}", e))?;
    Ok(())
}

pub fn get_model_path(app: &AppHandle) -> PathBuf {
    let path = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    path.join("models").join("learning_model.json")
}

pub fn default_model() -> LearningModel {
    LearningModel {
        patterns: vec![
            LearnedPattern {
                regex: r"(?m)^\s*//\s*File:\s*(.+)$".to_string(),
                path_group: 1,
                content_group: 0,
                language_hint: None,
                confidence: 0.9,
                usage_count: 0,
            },
            LearnedPattern {
                regex: r"(?m)^\s*#\s*File:\s*(.+)$".to_string(),
                path_group: 1,
                content_group: 0,
                language_hint: None,
                confidence: 0.9,
                usage_count: 0,
            },
            LearnedPattern {
                regex: r"\*\*File:\*\*\s*(.+?)(?:\n|$)".to_string(),
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

pub fn add_training_example(
    context_before: &str,
    header_line: &str,
    file_entry: &FileEntry,
    user_corrected: bool,
    model: &mut LearningModel,
) {
    let features = extract_features(header_line, 0.5);
    let example = TrainingExample {
        context_before: context_before.to_string(),
        header_line: header_line.to_string(),
        file_entry: file_entry.clone(),
        user_corrected,
        features,
    };
    model.training_examples.push(example);

    // Retrain with new data
    if model.training_examples.len() >= 5 {
        model.feature_weights = train(&model.training_examples);
    }
}
