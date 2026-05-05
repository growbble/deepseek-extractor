use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::extractor::{learning, extract_from_text};
use crate::models::{ExtractResult, FileEntry, ArchiveInfo, LearningModel};
use crate::preview;
use crate::packer;
use crate::url_extractor;

pub struct AppState {
    pub model: std::sync::Mutex<LearningModel>,
}

#[tauri::command]
pub async fn extract_from_clipboard(app: AppHandle) -> Result<ExtractResult, String> {
    // Run blocking clipboard read off the main thread
    let text = tokio::task::spawn_blocking(|| read_clipboard())
        .await
        .map_err(|e| format!("Task join error: {}", e))?
        .map_err(|e| format!("Clipboard error: {}", e))?;

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
    // Basic URL validation — prevent SSRF, only allow http/https
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Only http and https URLs are allowed".to_string());
    }

    let model = {
        let state = app.state::<AppState>();
        let guard = state.model.lock().map_err(|e| format!("Lock error: {}", e))?;
        guard.clone()
    };

    // Apply timeout to the whole fetch+extract operation
    let fetch_future = url_extractor::fetch_and_extract(&url, &model);
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), fetch_future)
        .await
        .map_err(|_| "Request timed out after 30 seconds".to_string())??;

    Ok(result)
}

#[tauri::command]
pub async fn save_files(files: Vec<FileEntry>, base_path: String) -> Result<u32, String> {
    let base = PathBuf::from(&base_path);
    let mut saved = 0u32;

    for entry in &files {
        // Path traversal protection
        if !entry.is_safe_path() {
            return Err(format!("Unsafe path rejected: {}", entry.path));
        }

        let full_path = base.join(&entry.path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create directory {}: {}", parent.display(), e))?;
        }
        std::fs::write(&full_path, &entry.content)
            .map_err(|e| format!("Cannot write {}: {}", full_path.display(), e))?;
        saved += 1;
    }

    Ok(saved)
}

#[tauri::command]
pub async fn create_archive(files: Vec<FileEntry>, save_path: String) -> Result<ArchiveInfo, String> {
    let path = PathBuf::from(&save_path);

    // Run blocking archive pack off main thread
    let info = tokio::task::spawn_blocking(move || {
        packer::pack(&files, &path, true)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    Ok(info)
}

#[tauri::command]
pub async fn extract_archive(archive_path: String, output_dir: String) -> Result<Vec<FileEntry>, String> {
    let archive = PathBuf::from(&archive_path);
    let output = PathBuf::from(&output_dir);

    let files = tokio::task::spawn_blocking(move || {
        packer::unpack(&archive, &output)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    Ok(files)
}

#[tauri::command]
pub async fn get_archive_info(archive_path: String) -> Result<ArchiveInfo, String> {
    let path = PathBuf::from(&archive_path);

    tokio::task::spawn_blocking(move || {
        packer::get_archive_info(&path)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
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
    // Run file I/O off main thread
    let app_clone = app.clone();
    let model = tokio::task::spawn_blocking(move || {
        learning::load_model(&app_clone)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

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
    // Persist to disk off main thread
    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        learning::save_model(&app_clone, &model)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn preview_file(file: FileEntry) -> Result<String, String> {
    // Run highlighting off main thread (CPU-bound with syntect)
    tokio::task::spawn_blocking(move || {
        preview::highlight_code(&file.content, &file.language)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
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

/// Synchronous clipboard reader — runs inside spawn_blocking.
/// Supports Linux (xclip, xsel), macOS (pbpaste), Windows (PowerShell).
fn read_clipboard() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        // Try xclip first
        let output = std::process::Command::new("xclip")
            .args(["-o", "-selection", "clipboard"])
            .output()
            .map_err(|e| format!("xclip not available: {}", e))?;

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
            .map_err(|e| format!("xsel not available: {}", e))?;

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
            .map_err(|e| format!("PowerShell failed: {}", e))?;

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
            .map_err(|e| format!("pbpaste failed: {}", e))?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            if !text.trim().is_empty() {
                return Ok(text);
            }
        }
    }

    Err("Clipboard is empty or no clipboard tool available".to_string())
}
