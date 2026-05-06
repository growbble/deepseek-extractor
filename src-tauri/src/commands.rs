use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::extractor::{learning, extract_from_text};
use crate::models::{ExtractResult, FileEntry, ArchiveInfo, LearningModel};
use crate::preview;
use crate::packer;
use crate::url_extractor;

pub(crate) struct AppState {
    pub model: std::sync::Mutex<LearningModel>,
}

/// Check that a string path doesn't escape the intended base directory.
/// Returns an error if path contains `..` or starts with `/` or `\`.
fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.contains("..") {
        return Err("Path traversal detected: '..' not allowed".into());
    }
    if path.contains('\0') {
        return Err("Path contains null byte".into());
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return Err("Absolute paths not allowed".into());
    }
    Ok(())
}

/// Validate base_path for save operations.
/// Must be absolute, no null bytes, not a git/config directory.
fn validate_base_path(base_path: &str) -> Result<&Path, String> {
    let path = Path::new(base_path);
    if !path.is_absolute() {
        return Err("Base path must be absolute".into());
    }
    if base_path.contains('\0') {
        return Err("Base path contains null byte".into());
    }
    // Prevent writing to git internals or sensitive config dirs
    let lower = base_path.to_lowercase();
    if lower.contains("/.git") || lower.contains("\\.git") {
        return Err("Cannot write inside .git directory".into());
    }
    Ok(path)
}

/// Allowed domains for SSRF protection.
/// Only URLs matching these exact domains (or subdomains) are permitted.
fn is_allowed_domain(url: &str) -> bool {
    let allowed = [
        "chat.deepseek.com",
        "chatgpt.com",
        "chat.openai.com",
        "claude.ai",
        "x.ai",
        "grok.com",
        "z.ai",
    ];

    // Parse the URL to extract host
    let url_lower = url.to_lowercase();
    // Quick check: must start with http:// or https://
    if !url_lower.starts_with("http://") && !url_lower.starts_with("https://") {
        return false;
    }

    // Strip protocol
    let rest = if let Some(s) = url_lower.strip_prefix("https://") {
        s
    } else if let Some(s) = url_lower.strip_prefix("http://") {
        s
    } else {
        return false;
    };

    // Extract host (up to first / or ? or :port)
    let host = rest.split('/').next()
        .and_then(|h| h.split('?').next())
        .and_then(|h| h.split(':').next())
        .unwrap_or("");

    // Reject IP addresses entirely (no bare IP fetching)
    if host.parse::<std::net::IpAddr>().is_ok() {
        return false;
    }
    // Reject localhost/loopback
    if host == "localhost" || host == "127.0.0.1" || host == "::1" || host == "0.0.0.0" {
        return false;
    }
    // Reject internal RFC1918 ranges by checking the first octet
    if let Some(rest_after_scheme) = url_lower.strip_prefix("http://") {
        if let Some(ip_candidate) = rest_after_scheme.split('/').next() {
            if let Some(first_octet) = ip_candidate.split('.').next() {
                if let Ok(octet) = first_octet.parse::<u8>() {
                    if octet == 10 || octet == 172 || octet == 192 {
                        return false;
                    }
                }
            }
        }
    }

    allowed.iter().any(|a| host == *a || host.ends_with(&format!(".{}", a)))
}

#[tauri::command]
pub(crate) async fn extract_from_clipboard(app: AppHandle) -> Result<ExtractResult, String> {
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
pub(crate) async fn extract_from_url(url: String, app: AppHandle) -> Result<ExtractResult, String> {
    // SSRF protection: only allowlisted domains
    if !is_allowed_domain(&url) {
        return Err("URL domain not in allowlist. Supported: chat.deepseek.com, chatgpt.com, claude.ai, x.ai, grok.com, z.ai".to_string());
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
pub(crate) async fn save_files(files: Vec<FileEntry>, base_path: String) -> Result<u32, String> {
    let base = validate_base_path(&base_path)?;

    // Limit number of files to prevent DoS
    if files.len() > 10_000 {
        return Err("Too many files to save (max 10,000)".into());
    }

    let mut saved = 0u32;

    for entry in &files {
        if !entry.is_safe_path() {
            return Err(format!("Unsafe path rejected: {}", entry.path));
        }

        let full_path = base.join(&entry.path);

        // Canonicalize target dir once to verify we're writing within it
        //
        // We resolve the target directory first, then check that the full path
        // starts with that canonicalized directory as a second layer of defense
        // against path traversal via symlinks.
        let base_canonical = std::fs::canonicalize(base)
            .unwrap_or_else(|_| base.to_path_buf());

        // Resolve the parent of the target file (if it exists) or use base as fallback
        let parent = full_path.parent().ok_or_else(|| "Invalid path".to_string())?;

        // Create parent dirs before canonicalizing (the file might not exist yet)
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create directory {}: {}", parent.display(), e))?;

        // Canonicalize the parent now that it exists
        let parent_canonical = std::fs::canonicalize(parent)
            .map_err(|e| format!("Cannot resolve path {}: {}", parent.display(), e))?;

        // Archive slip protection: parent must be inside base
        if !parent_canonical.starts_with(&base_canonical) {
            return Err(format!("Path traversal detected: {} escapes base directory", entry.path));
        }

        std::fs::write(&full_path, &entry.content)
            .map_err(|e| format!("Cannot write {}: {}", full_path.display(), e))?;
        saved += 1;
    }

    Ok(saved)
}

#[tauri::command]
pub(crate) async fn create_archive(files: Vec<FileEntry>, save_path: String) -> Result<ArchiveInfo, String> {
    if files.is_empty() {
        return Err("No files to archive".into());
    }
    validate_relative_path(&save_path)?;
    if !save_path.ends_with(".cpk") {
        return Err("Archive path must end with .cpk".into());
    }

    let path = PathBuf::from(&save_path);

    let info = tokio::task::spawn_blocking(move || {
        packer::pack(&files, &path, true)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    Ok(info)
}

#[tauri::command]
pub(crate) async fn extract_archive(archive_path: String, output_dir: String) -> Result<Vec<FileEntry>, String> {
    validate_base_path(&output_dir)?;
    validate_relative_path(&archive_path)?;

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
pub(crate) async fn get_archive_info(archive_path: String) -> Result<ArchiveInfo, String> {
    validate_relative_path(&archive_path)?;
    let path = PathBuf::from(&archive_path);

    tokio::task::spawn_blocking(move || {
        packer::get_archive_info(&path)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub(crate) async fn update_entry(
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
pub(crate) async fn load_model(app: AppHandle) -> Result<LearningModel, String> {
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
pub(crate) async fn save_model(app: AppHandle, model: LearningModel) -> Result<(), String> {
    {
        let state = app.state::<AppState>();
        let mut guard = state.model.lock().map_err(|e| format!("Lock error: {}", e))?;
        *guard = model.clone();
    }
    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        learning::save_model(&app_clone, &model)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub(crate) async fn preview_file(file: FileEntry) -> Result<String, String> {
    // Limit preview to 1MB to prevent memory DoS
    if file.content.len() > 1_048_576 {
        let truncated_content: String = file.content.chars().take(1_048_576).collect();
        return tokio::task::spawn_blocking(move || {
            preview::highlight_code(&truncated_content, &file.language)
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?;
    }

    tokio::task::spawn_blocking(move || {
        preview::highlight_code(&file.content, &file.language)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub(crate) fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub(crate) fn get_platform_names() -> Vec<String> {
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
        let output = std::process::Command::new("xclip")
            .args(["-o", "-selection", "clipboard"])
            .output()
            .map_err(|e| format!("xclip not available: {}", e))?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            if !text.trim().is_empty() {
                // Validate: limit clipboard to 10MB
                if text.len() > 10_485_760 {
                    return Err("Clipboard content too large (>10MB)".to_string());
                }
                return Ok(text);
            }
        }

        let output2 = std::process::Command::new("xsel")
            .args(["-o", "-b"])
            .output()
            .map_err(|e| format!("xsel not available: {}", e))?;

        if output2.status.success() {
            let text = String::from_utf8_lossy(&output2.stdout).to_string();
            if !text.trim().is_empty() {
                if text.len() > 10_485_760 {
                    return Err("Clipboard content too large (>10MB)".to_string());
                }
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
                if text.len() > 10_485_760 {
                    return Err("Clipboard content too large (>10MB)".to_string());
                }
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
                if text.len() > 10_485_760 {
                    return Err("Clipboard content too large (>10MB)".to_string());
                }
                return Ok(text);
            }
        }
    }

    Err("Clipboard is empty or no clipboard tool available".to_string())
}
