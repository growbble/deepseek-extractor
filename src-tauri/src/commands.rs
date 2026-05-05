use std::path::PathBuf;
use tauri::{AppHandle, State, Manager};

use crate::extractor::{learning, extract_from_text, extract};
use crate::models::{ExtractResult, FileEntry, ArchiveInfo, LearningModel};
use crate::preview;
use crate::packer;
use crate::url_extractor;

pub struct AppState {
    pub model: std::sync::Mutex<LearningModel>,
}

#[tauri::command]
pub async fn extract_from_clipboard(app: AppHandle) -> Result<ExtractResult, String> {
    let text = read_clipboard()?;
    let model = {
        let state = app.state::<AppState>();
        let guard = state.model.lock().map_err(|e| format!("Lock error: {}", e))?;
        guard.clone()
    };
    let result = extract_from_text(&text, "clipboard", &model);
    Ok(result)
}

#[tauri::command]
pub async fn extract_from_url(url: String, app: AppHandle) -> Result<ExtractResult, String> {
    let model = {
        let state = app.state::<AppState>();
        let guard = state.model.lock().map_err(|e| format!("Lock error: {}", e))?;
        guard.clone()
    };
    let result = url_extractor::fetch_and_extract(&url, &model).await?;
    Ok(result)
}

#[tauri::command]
pub async fn save_files(files: Vec<FileEntry>, base_path: String) -> Result<String, String> {
    let base = PathBuf::from(&base_path);
    let mut saved = 0u32;

    for entry in &files {
        let full_path = base.join(&entry.path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create directory {}: {}", parent.display(), e))?;
        }
        std::fs::write(&full_path, &entry.content)
            .map_err(|e| format!("Cannot write {}: {}", full_path.display(), e))?;
        saved += 1;
    }

    Ok(format!("{}", saved))
}

#[tauri::command]
pub async fn create_archive(files: Vec<FileEntry>, save_path: String) -> Result<ArchiveInfo, String> {
    let path = PathBuf::from(&save_path);
    let info = packer::pack(&files, &path, true)?;
    Ok(info)
}

#[tauri::command]
pub async fn extract_archive(archive_path: String, output_dir: String) -> Result<Vec<FileEntry>, String> {
    let archive = PathBuf::from(&archive_path);
    let output = PathBuf::from(&output_dir);
    let files = packer::unpack(&archive, &output)?;
    Ok(files)
}

#[tauri::command]
pub async fn get_archive_info(archive_path: String) -> Result<ArchiveInfo, String> {
    let path = PathBuf::from(&archive_path);
    let info = packer::get_archive_info(&path)?;
    Ok(info)
}

#[tauri::command]
pub async fn update_entry(
    old_entry: FileEntry,
    new_entry: FileEntry,
    app: AppHandle,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut guard = state.model.lock().map_err(|e| format!("Lock error: {}", e))?;

    learning::add_training_example(
        &old_entry.path,
        &old_entry.name,
        &new_entry,
        old_entry.path != new_entry.path,
        &mut guard,
    );

    Ok(())
}

#[tauri::command]
pub async fn load_model(app: AppHandle) -> Result<LearningModel, String> {
    let model = learning::load_model(&app);
    // Also update the managed state
    let state = app.state::<AppState>();
    let mut guard = state.model.lock().map_err(|e| format!("Lock error: {}", e))?;
    *guard = model.clone();
    Ok(model)
}

#[tauri::command]
pub async fn save_model(app: AppHandle, model: LearningModel) -> Result<(), String> {
    // Update managed state
    {
        let state = app.state::<AppState>();
        let mut guard = state.model.lock().map_err(|e| format!("Lock error: {}", e))?;
        *guard = model.clone();
    }
    // Persist to disk
    learning::save_model(&app, &model)?;
    Ok(())
}

#[tauri::command]
pub async fn preview_file(file: FileEntry) -> Result<String, String> {
    let html = preview::highlight_code(&file.content, &file.language)?;
    Ok(html)
}

#[tauri::command]
pub fn get_version() -> String {
    "1.0.0".to_string()
}

#[tauri::command]
pub fn get_platform_names() -> Vec<String> {
    vec![
        "DeepSeek".to_string(),
        "ChatGPT".to_string(),
        "Claude".to_string(),
        "Grok".to_string(),
        "Z.ai".to_string(),
    ]
}

fn read_clipboard() -> Result<String, String> {
    // Try xclip first (Linux), then powershell (Windows)
    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("xclip")
            .args(["-o", "-selection", "clipboard"])
            .output()
            .map_err(|e| format!("Failed to run xclip: {}. Is xclip installed?", e))?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            if !text.trim().is_empty() {
                return Ok(text);
            }
        }

        // Try xsel as fallback
        let output2 = std::process::Command::new("xsel")
            .args(["-o", "-b"])
            .output()
            .map_err(|e| format!("Failed to run xsel: {}", e))?;

        if output2.status.success() {
            let text = String::from_utf8_lossy(&output2.stdout).to_string();
            if !text.trim().is_empty() {
                return Ok(text);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("powershell")
            .args(["-command", "Get-Clipboard"])
            .output()
            .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            if !text.trim().is_empty() {
                return Ok(text);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("pbpaste")
            .output()
            .map_err(|e| format!("Failed to run pbpaste: {}", e))?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            if !text.trim().is_empty() {
                return Ok(text);
            }
        }
    }

    Err("Clipboard is empty or no clipboard tool available".to_string())
}
