use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, Component};
use sha2::{Sha256, Digest};

use crate::models::{ArchiveEntry, ArchiveInfo, FileEntry};

const MAGIC: &[u8; 4] = b"CPK\0";
const VERSION: u16 = 1;

/// CPK format layout:
/// [HEADER]    4B magic CPK\0 | 2B version | 1B flags | 4B entry_count
/// [TOC]       entry_count × (2B name_len | name_bytes | 4B reserved | 8B orig_size | 8B comp_size | 8B data_offset)
/// [DATA]      entry_count × comp_data (variable)
/// [FOOTER]    32B SHA-256 of TOC

const HEADER_SIZE: u64 = 4 + 2 + 1 + 4; // 11
const TOC_ENTRY_SIZE: u64 = 2 + 4 + 8 + 8 + 8; // 30

const MAX_ENTRIES: usize = 10_000;
const MAX_NAME_LEN: usize = 4_096;
const MAX_ENTRY_SIZE: u64 = 256 * 1024 * 1024; // 256 MB
const MIN_FILE_SIZE: u64 = HEADER_SIZE + 32; // header + minimal hash

/// Normalize a path to prevent archive slip — resolves `.` and `..` components.
/// Returns None if the path would escape its root via `..`.
fn normalize_path(path: &str) -> Option<String> {
    let mut result = Vec::new();
    for component in Path::new(path).components() {
        match component {
            Component::ParentDir => {
                if result.pop().is_none() {
                    // Attempted to escape above root
                    return None;
                }
            }
            Component::Normal(part) => {
                let s = part.to_str()?;
                if !s.is_empty() {
                    result.push(s.to_string());
                }
            }
            Component::CurDir => { /* skip `.` */ }
            Component::RootDir | Component::Prefix(_) => {
                // Absolute components not allowed
                return None;
            }
        }
    }
    Some(result.join("/"))
}

pub(crate) fn pack(entries: &[FileEntry], save_path: &Path, compress: bool) -> Result<ArchiveInfo, String> {
    if entries.is_empty() {
        return Err("No files to archive".into());
    }
    if entries.len() > MAX_ENTRIES {
        return Err(format!("Too many entries (max {})", MAX_ENTRIES));
    }

    let file = std::fs::File::create(save_path)
        .map_err(|e| format!("Cannot create archive: {}", e))?;
    let mut writer = std::io::BufWriter::new(file);

    let count: u32 = entries.len().try_into()
        .map_err(|_| "Too many entries".to_string())?;
    let flags: u8 = if compress { 1 } else { 0 } | 2; // compressed + has_hash

    writer.write_all(MAGIC).map_err(|e| format!("Write error: {}", e))?;
    writer.write_all(&VERSION.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;
    writer.write_all(&flags.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;
    writer.write_all(&count.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;

    struct TocEntry {
        name: String,
        orig_size: u64,
        comp_size: u64,
        data: Vec<u8>,
    }

    let mut toc_entries: Vec<TocEntry> = Vec::with_capacity(entries.len());
    let mut seen_names = std::collections::HashSet::new();

    for entry in entries {
        // Deduplicate entry names
        if !seen_names.insert(entry.path.clone()) {
            return Err(format!("Duplicate entry name: {}", entry.path));
        }

        let data = entry.content.as_bytes();
        let orig_size: u64 = data.len().try_into()
            .map_err(|_| "Entry content too large".to_string())?;

        if orig_size > MAX_ENTRY_SIZE {
            return Err(format!("Entry '{}' too large ({} > {} max)", entry.path, orig_size, MAX_ENTRY_SIZE));
        }

        let compressed = if compress && orig_size > 0 {
            let bound = zstd::zstd_safe::compress_bound(orig_size as usize);
            let mut cbuf = vec![0u8; bound];
            let written = zstd::zstd_safe::compress(&mut cbuf, data, 1)
                .map_err(|e| format!("Compression error: {}", e))?;
            if written < orig_size as usize {
                cbuf[..written].to_vec()
            } else {
                data.to_vec()
            }
        } else {
            data.to_vec()
        };

        let comp_size: u64 = compressed.len().try_into()
            .map_err(|_| "Compressed size overflow".to_string())?;

        toc_entries.push(TocEntry {
            name: entry.path.clone(),
            orig_size,
            comp_size,
            data: compressed,
        });
    }

    let mut data_offset = HEADER_SIZE.saturating_add((count as u64).saturating_mul(TOC_ENTRY_SIZE));
    let mut toc_hasher = Sha256::new();

    // Write TOC
    for te in &toc_entries {
        let name_bytes = te.name.as_bytes();
        let name_len: u16 = name_bytes.len().try_into()
            .map_err(|_| "Entry name too long".to_string())?;

        toc_hasher.update(&name_len.to_le_bytes());
        toc_hasher.update(name_bytes);
        toc_hasher.update(&te.orig_size.to_le_bytes());
        toc_hasher.update(&te.comp_size.to_le_bytes());

        writer.write_all(&name_len.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;
        writer.write_all(name_bytes).map_err(|e| format!("Write error: {}", e))?;
        writer.write_all(&[0u8; 4]).map_err(|e| format!("Write error: {}", e))?;
        writer.write_all(&te.orig_size.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;
        writer.write_all(&te.comp_size.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;
        writer.write_all(&data_offset.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;

        data_offset = data_offset.saturating_add(te.data.len() as u64);
    }

    writer.flush().map_err(|e| format!("Flush error: {}", e))?;

    // Write data blocks
    for te in &toc_entries {
        writer.write_all(&te.data).map_err(|e| format!("Write error: {}", e))?;
    }

    // Write SHA-256 hash of TOC as footer
    let footer_hash = toc_hasher.finalize();
    writer.write_all(&footer_hash).map_err(|e| format!("Write error: {}", e))?;
    writer.flush().map_err(|e| format!("Flush error: {}", e))?;
    drop(writer);

    let total_original: u64 = toc_entries.iter().fold(0u64, |a, e| a.saturating_add(e.orig_size));
    let total_compressed: u64 = toc_entries.iter().fold(0u64, |a, e| a.saturating_add(e.comp_size));

    let arch_entries: Vec<ArchiveEntry> = toc_entries.iter().map(|te| ArchiveEntry {
        name: te.name.clone(),
        original_size: te.orig_size,
        compressed_size: te.comp_size,
    }).collect();

    Ok(ArchiveInfo {
        file_count: count,
        total_original,
        total_compressed,
        entries: arch_entries,
    })
}

pub(crate) fn unpack(archive_path: &Path, output_dir: &Path) -> Result<Vec<FileEntry>, String> {
    let file = std::fs::File::open(archive_path)
        .map_err(|e| format!("Cannot open archive: {}", e))?;
    let file_len: u64 = file.metadata().map(|m| m.len()).unwrap_or(0);

    if file_len < MIN_FILE_SIZE {
        return Err("File too small to be a valid archive".into());
    }

    let mut reader = std::io::BufReader::new(file);

    // Read magic
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic).map_err(|e| format!("Read error: {}", e))?;
    if &magic != MAGIC {
        return Err("Invalid archive format: bad magic".into());
    }

    let mut version_buf = [0u8; 2];
    reader.read_exact(&mut version_buf).map_err(|e| format!("Read error: {}", e))?;
    let _version = u16::from_le_bytes(version_buf);

    let mut flags_buf = [0u8; 1];
    reader.read_exact(&mut flags_buf).map_err(|e| format!("Read error: {}", e))?;
    let flags = flags_buf[0];
    let has_hash = (flags & 2) != 0;

    let mut count_buf = [0u8; 4];
    reader.read_exact(&mut count_buf).map_err(|e| format!("Read error: {}", e))?;
    let count = u32::from_le_bytes(count_buf) as usize;

    if count == 0 || count > MAX_ENTRIES {
        return Err(format!("Invalid entry count: {}", count));
    }

    let expected_toc_size: u64 = (count as u64).saturating_mul(TOC_ENTRY_SIZE);
    let expected_min = HEADER_SIZE.saturating_add(expected_toc_size).saturating_add(if has_hash { 32 } else { 0 });
    if file_len < expected_min {
        return Err("Archive too small for claimed entries".into());
    }

    let mut toc_hasher = Sha256::new();

    // Read TOC
    let mut toc_entries: Vec<(String, u64, u64, u64)> = Vec::with_capacity(count);
    for _ in 0..count {
        let mut name_len_buf = [0u8; 2];
        reader.read_exact(&mut name_len_buf).map_err(|e| format!("Read error: {}", e))?;
        let name_len = u16::from_le_bytes(name_len_buf) as usize;

        if name_len == 0 || name_len > MAX_NAME_LEN {
            return Err(format!("Invalid entry name length: {}", name_len));
        }

        let mut name_bytes = vec![0u8; name_len];
        reader.read_exact(&mut name_bytes).map_err(|e| format!("Read error: {}", e))?;
        let name = String::from_utf8(name_bytes.clone())
            .map_err(|_| "Invalid UTF-8 in entry name".to_string())?;

        let mut _reserved = [0u8; 4];
        reader.read_exact(&mut _reserved).map_err(|e| format!("Read error: {}", e))?;

        let mut orig_size_buf = [0u8; 8];
        reader.read_exact(&mut orig_size_buf).map_err(|e| format!("Read error: {}", e))?;
        let orig_size = u64::from_le_bytes(orig_size_buf);

        let mut comp_size_buf = [0u8; 8];
        reader.read_exact(&mut comp_size_buf).map_err(|e| format!("Read error: {}", e))?;
        let comp_size = u64::from_le_bytes(comp_size_buf);

        let mut data_offset_buf = [0u8; 8];
        reader.read_exact(&mut data_offset_buf).map_err(|e| format!("Read error: {}", e))?;
        let data_offset = u64::from_le_bytes(data_offset_buf);

        // Update TOC hash
        toc_hasher.update(&name_len_buf);
        toc_hasher.update(&name_bytes);
        toc_hasher.update(&orig_size_buf);
        toc_hasher.update(&comp_size_buf);

        toc_entries.push((name, orig_size, comp_size, data_offset));
    }

    // Verify TOC hash
    if has_hash {
        let computed_hash = toc_hasher.finalize();
        let mut stored_hash = [0u8; 32];
        reader.seek(SeekFrom::End(-32)).map_err(|e| format!("Seek error: {}", e))?;
        reader.read_exact(&mut stored_hash).map_err(|e| format!("Read hash error: {}", e))?;

        if computed_hash[..] != stored_hash[..] {
            return Err("Archive hash mismatch: TOC integrity check failed".into());
        }

        let data_start = HEADER_SIZE.saturating_add((count as u64).saturating_mul(TOC_ENTRY_SIZE));
        reader.seek(SeekFrom::Start(data_start)).map_err(|e| format!("Seek error: {}", e))?;
    }

    // Read and decompress data
    let mut files = Vec::with_capacity(toc_entries.len());

    for (name, orig_size, comp_size, data_offset) in &toc_entries {
        let offset = *data_offset;
        let csize: u64 = *comp_size;
        let osize: u64 = *orig_size;

        if osize > MAX_ENTRY_SIZE {
            return Err(format!("Entry '{}' original size {} exceeds maximum {}", name, osize, MAX_ENTRY_SIZE));
        }

        let data_end = offset.saturating_add(csize);
        if data_end > file_len {
            return Err(format!("Corrupt archive: entry '{}' exceeds file length", name));
        }

        let toc_end = HEADER_SIZE.saturating_add((count as u64).saturating_mul(TOC_ENTRY_SIZE));
        if offset < toc_end {
            return Err(format!("Corrupt archive: entry '{}' overlaps TOC", name));
        }

        reader.seek(SeekFrom::Start(offset))
            .map_err(|e| format!("Seek error at '{}': {}", name, e))?;

        let csize_usize: usize = (csize).try_into()
            .map_err(|_| format!("Compressed size overflow in '{}'", name))?;
        let mut compressed_data = vec![0u8; csize_usize];
        reader.read_exact(&mut compressed_data)
            .map_err(|e| format!("Read error at '{}': {}", name, e))?;

        let is_compressed = csize < osize;
        let content: String = if is_compressed {
            let osize_usize: usize = osize.try_into()
                .map_err(|_| format!("Original size overflow in '{}'", name))?;
            let mut decompressed = vec![0u8; osize_usize];
            zstd::zstd_safe::decompress(&mut decompressed, &compressed_data)
                .map_err(|e| format!("Decompression error in '{}': {}", name, e))?;
            String::from_utf8(decompressed)
                .map_err(|_| format!("Invalid UTF-8 in '{}'", name))?
        } else {
            String::from_utf8(compressed_data)
                .map_err(|_| format!("Invalid UTF-8 in '{}'", name))?
        };

        let file_name = extract_name(name);
        let language = extract_lang(name);

        files.push(FileEntry {
            id: uuid::Uuid::new_v4().to_string(),
            path: name.clone(),
            name: file_name,
            language,
            content,
            size: *orig_size,
            selected: true,
        });
    }

    // Create output dir
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Cannot create output dir: {}", e))?;

    // Canonicalize output dir for archive slip protection
    let output_canonical = std::fs::canonicalize(output_dir)
        .map_err(|e| format!("Cannot resolve output dir: {}", e))?;

    for entry in &files {
        // First: normalize path (resolve .. and . components)
        let normalized = normalize_path(&entry.path)
            .ok_or_else(|| format!("Unsafe path in archive: {}", entry.path))?;
        let normalized_path = Path::new(&normalized);

        // Check is_safe_path for secondary validation
        if !entry.is_safe_path() {
            return Err(format!("Unsafe entry rejected: {}", entry.path));
        }

        let full_path = output_canonical.join(normalized_path);

        // **Archive slip protection**: verify the resolved path is still within output_dir
        let full_canonical = std::fs::canonicalize(full_path.parent().unwrap_or(&full_path))
            .unwrap_or_else(|_| full_path.clone());

        if !full_canonical.starts_with(&output_canonical) {
            // If canonicalization failed or path escapes, use the safer check
            if !full_path.starts_with(&output_canonical) {
                return Err(format!("Archive slip detected: '{}' escapes output directory", entry.path));
            }
        }

        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create dir {}: {}", parent.display(), e))?;
        }
        std::fs::write(&full_path, &entry.content)
            .map_err(|e| format!("Cannot write {}: {}", full_path.display(), e))?;
    }

    Ok(files)
}

pub(crate) fn get_archive_info(archive_path: &Path) -> Result<ArchiveInfo, String> {
    let file = std::fs::File::open(archive_path)
        .map_err(|e| format!("Cannot open archive: {}", e))?;
    let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);

    if file_len < MIN_FILE_SIZE {
        return Err("File too small to be a valid archive".into());
    }

    let mut reader = std::io::BufReader::new(file);

    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic).map_err(|e| format!("Read error: {}", e))?;
    if &magic != MAGIC {
        return Err("Invalid archive format".into());
    }

    let mut _version_buf = [0u8; 2];
    reader.read_exact(&mut _version_buf).map_err(|e| format!("Read error: {}", e))?;

    let mut _flags_buf = [0u8; 1];
    reader.read_exact(&mut _flags_buf).map_err(|e| format!("Read error: {}", e))?;

    let mut count_buf = [0u8; 4];
    reader.read_exact(&mut count_buf).map_err(|e| format!("Read error: {}", e))?;
    let count = u32::from_le_bytes(count_buf);

    if count > MAX_ENTRIES as u32 {
        return Err("Archive has too many entries".into());
    }

    let mut entries = Vec::with_capacity(count as usize);
    let mut total_orig: u64 = 0;
    let mut total_comp: u64 = 0;

    for _ in 0..count {
        let mut name_len_buf = [0u8; 2];
        reader.read_exact(&mut name_len_buf).map_err(|e| format!("Read error: {}", e))?;
        let name_len = u16::from_le_bytes(name_len_buf) as usize;

        if name_len == 0 || name_len > MAX_NAME_LEN {
            return Err(format!("Invalid entry name length: {}", name_len));
        }

        let mut name_bytes = vec![0u8; name_len];
        reader.read_exact(&mut name_bytes).map_err(|e| format!("Read error: {}", e))?;
        let name = String::from_utf8(name_bytes).map_err(|_| "Invalid UTF-8".to_string())?;

        let mut _reserved = [0u8; 4];
        reader.read_exact(&mut _reserved).map_err(|e| format!("Read error: {}", e))?;

        let mut orig_buf = [0u8; 8];
        reader.read_exact(&mut orig_buf).map_err(|e| format!("Read error: {}", e))?;
        let orig_size = u64::from_le_bytes(orig_buf);

        let mut comp_buf = [0u8; 8];
        reader.read_exact(&mut comp_buf).map_err(|e| format!("Read error: {}", e))?;
        let comp_size = u64::from_le_bytes(comp_buf);

        let mut _offset_buf = [0u8; 8];
        reader.read_exact(&mut _offset_buf).map_err(|e| format!("Read error: {}", e))?;

        total_orig = total_orig.saturating_add(orig_size);
        total_comp = total_comp.saturating_add(comp_size);

        entries.push(ArchiveEntry {
            name,
            original_size: orig_size,
            compressed_size: comp_size,
        });
    }

    Ok(ArchiveInfo {
        file_count: count,
        total_original: total_orig,
        total_compressed: total_comp,
        entries,
    })
}

fn extract_name(path: &str) -> String {
    if let Some(idx) = path.rfind('/') {
        path[idx + 1..].to_string()
    } else if let Some(idx) = path.rfind('\\') {
        path[idx + 1..].to_string()
    } else {
        path.to_string()
    }
}

fn extract_lang(path: &str) -> String {
    let ext = path.rsplit('.').next().map(|s| s.to_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "go" => "go",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "md" => "markdown",
        _ => &ext,
    }.to_string()
}
