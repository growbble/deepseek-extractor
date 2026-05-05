use std::io::{Read, Write};
use std::path::Path;
use sha2::{Sha256, Digest};

use crate::models::{ArchiveEntry, ArchiveInfo, FileEntry};

const MAGIC: &[u8; 4] = b"CPK\0";
const VERSION: u16 = 1;

pub fn pack(entries: &[FileEntry], save_path: &Path, compress: bool) -> Result<ArchiveInfo, String> {
    let file = std::fs::File::create(save_path)
        .map_err(|e| format!("Cannot create archive: {}", e))?;
    let mut writer = std::io::BufWriter::new(file);

    // Write header
    let count = entries.len() as u32;
    let flags: u8 = if compress { 1 } else { 0 } | 2; // bit 0: compressed, bit 1: has_hash

    writer.write_all(MAGIC).map_err(|e| format!("Write error: {}", e))?;
    writer.write_all(&VERSION.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;
    writer.write_all(&flags.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;
    writer.write_all(&count.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;

    // First pass: compress all data and build TOC entries
    struct TocEntry {
        name: String,
        orig_size: u64,
        comp_size: u64,
        data: Vec<u8>,
    }

    let mut toc_entries: Vec<TocEntry> = Vec::new();

    for entry in entries {
        let data = entry.content.as_bytes();
        let orig_size = data.len() as u64;

        let compressed = if compress {
            let bound = zstd::zstd_safe::compress_bound(orig_size as usize);
            let mut comp = Vec::with_capacity(bound);
            let mut cbuf = vec![0u8; bound];
            let written = zstd::zstd_safe::compress(&mut cbuf, data, 1)
                .map_err(|e| format!("Compression error: {}", e))?;
            comp.extend_from_slice(&cbuf[..written]);

            // If compression didn't help, store raw
            if (comp.len() as u64) < orig_size {
                comp
            } else {
                data.to_vec()
            }
        } else {
            data.to_vec()
        };

        toc_entries.push(TocEntry {
            name: entry.path.clone(),
            orig_size,
            comp_size: compressed.len() as u64,
            data: compressed,
        });
    }

    // Write TOC
    let mut toc_hasher = Sha256::new();
    let mut toc_offset = 4 + 2 + 1 + 4; // magic + version + flags + count
    let entry_header_size = 2 + 4 + 8 + 8 + 8; // name_len + padding + orig_size + comp_size + data_offset

    for (i, te) in toc_entries.iter().enumerate() {
        let name_bytes = te.name.as_bytes();
        let name_len = name_bytes.len() as u16;

        // Update TOC hash
        toc_hasher.update(&name_len.to_le_bytes());
        toc_hasher.update(name_bytes);
        toc_hasher.update(&te.orig_size.to_le_bytes());
        toc_hasher.update(&te.comp_size.to_le_bytes());

        writer.write_all(&name_len.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;
        writer.write_all(name_bytes).map_err(|e| format!("Write error: {}", e))?;
        // 4 byte padding (was NAME_LEN[2]+reserved[2])
        writer.write_all(&[0u8; 4]).map_err(|e| format!("Write error: {}", e))?;
        writer.write_all(&te.orig_size.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;
        writer.write_all(&te.comp_size.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;
        writer.write_all(&toc_offset.to_le_bytes()).map_err(|e| format!("Write error: {}", e))?;

        toc_offset += te.data.len() as u64;

        if i == 0 {
            toc_offset = toc_offset; // Initial TOC offset is right after header
        }
    }

    // Determine data section start: after TOC
    let data_start = 4 + 2 + 1 + 4
        + toc_entries.len() * (2 + 4 + 8 + 8 + 8);
    for (i, te) in toc_entries.iter().enumerate() {
        let entry_offset = data_start as u64
            + toc_entries[..i].iter().map(|e| e.data.len() as u64).sum::<u64>();
        // Rewrite the data_offset in TOC (we need to seek back)
        let toc_entry_pos = 4 + 2 + 1 + 4 + i * (2 + 4 + 8 + 8 + 8) + 2 + 4 + 8 + 8;
        writer.flush().map_err(|e| format!("Flush error: {}", e))?;

        // Use the file directly for seeking
        drop(writer);
        let mut file = std::fs::File::options()
            .write(true)
            .open(save_path)
            .map_err(|e| format!("Cannot reopen archive: {}", e))?;

        use std::io::Seek;
        file.seek(std::io::SeekFrom::Start(toc_entry_pos as u64))
            .map_err(|e| format!("Seek error: {}", e))?;
        file.write_all(&entry_offset.to_le_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
        drop(file);

        writer = std::io::BufWriter::new(
            std::fs::File::options()
                .append(true)
                .open(save_path)
                .map_err(|e| format!("Cannot reopen archive for append: {}", e))?
        );
    }

    // Write data
    drop(writer);
    let mut file = std::fs::File::options()
        .append(true)
        .open(save_path)
        .map_err(|e| format!("Cannot reopen archive: {}", e))?;

    for te in &toc_entries {
        file.write_all(&te.data).map_err(|e| format!("Write error: {}", e))?;
    }

    // Write SHA-256 footer hash of TOC
    let footer_hash = toc_hasher.finalize();
    file.write_all(&footer_hash).map_err(|e| format!("Write error: {}", e))?;

    drop(file);

    // Build archive info
    let total_original: u64 = toc_entries.iter().map(|e| e.orig_size).sum();
    let total_compressed: u64 = toc_entries.iter().map(|e| e.comp_size).sum();

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

pub fn unpack(archive_path: &Path, output_dir: &Path) -> Result<Vec<FileEntry>, String> {
    let mut file = std::fs::File::open(archive_path)
        .map_err(|e| format!("Cannot open archive: {}", e))?;

    // Read and verify magic
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).map_err(|e| format!("Read error: {}", e))?;
    if &magic != MAGIC {
        return Err("Invalid archive format (bad magic)".to_string());
    }

    // Read version
    let mut version_buf = [0u8; 2];
    file.read_exact(&mut version_buf).map_err(|e| format!("Read error: {}", e))?;
    let _version = u16::from_le_bytes(version_buf);

    // Read flags
    let mut flags_buf = [0u8; 1];
    file.read_exact(&mut flags_buf).map_err(|e| format!("Read error: {}", e))?;
    let flags = flags_buf[0];
    let compressed = (flags & 1) != 0;

    // Read count
    let mut count_buf = [0u8; 4];
    file.read_exact(&mut count_buf).map_err(|e| format!("Read error: {}", e))?;
    let count = u32::from_le_bytes(count_buf);

    // Read TOC
    let mut toc_entries = Vec::new();
    for _ in 0..count {
        let mut name_len_buf = [0u8; 2];
        file.read_exact(&mut name_len_buf).map_err(|e| format!("Read error: {}", e))?;
        let name_len = u16::from_le_bytes(name_len_buf) as usize;

        let mut name_bytes = vec![0u8; name_len];
        file.read_exact(&mut name_bytes).map_err(|e| format!("Read error: {}", e))?;
        let name = String::from_utf8(name_bytes).map_err(|_| "Invalid UTF-8 in archive".to_string())?;

        // Skip 4 bytes padding
        let mut padding = [0u8; 4];
        file.read_exact(&mut padding).map_err(|e| format!("Read error: {}", e))?;

        let mut orig_size_buf = [0u8; 8];
        file.read_exact(&mut orig_size_buf).map_err(|e| format!("Read error: {}", e))?;
        let orig_size = u64::from_le_bytes(orig_size_buf);

        let mut comp_size_buf = [0u8; 8];
        file.read_exact(&mut comp_size_buf).map_err(|e| format!("Read error: {}", e))?;
        let comp_size = u64::from_le_bytes(comp_size_buf);

        let mut data_offset_buf = [0u8; 8];
        file.read_exact(&mut data_offset_buf).map_err(|e| format!("Read error: {}", e))?;
        let data_offset = u64::from_le_bytes(data_offset_buf);

        toc_entries.push((name, orig_size, comp_size, data_offset));
    }

    // Determine filename from path
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
        let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
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

    // Read and decompress data
    let mut files = Vec::new();
    for (name, orig_size, comp_size, data_offset) in toc_entries {
        use std::io::Seek;
        file.seek(std::io::SeekFrom::Start(data_offset))
            .map_err(|e| format!("Seek error: {}", e))?;

        let mut data = vec![0u8; comp_size as usize];
        file.read_exact(&mut data).map_err(|e| format!("Read error: {}", e))?;

        let content = if compressed && comp_size < orig_size {
            let mut decompressed = vec![0u8; orig_size as usize];
            zstd::zstd_safe::decompress(&mut decompressed, &data)
                .map_err(|e| format!("Decompression error: {}", e))?;
            String::from_utf8(decompressed).map_err(|_| "Invalid UTF-8 in archive".to_string())?
        } else {
            String::from_utf8(data).map_err(|_| "Invalid UTF-8 in archive".to_string())?
        };

        let file_name = extract_name(&name);

        files.push(FileEntry {
            id: uuid::Uuid::new_v4().to_string(),
            path: name,
            name: file_name,
            language: extract_lang(&name),
            content,
            size: orig_size,
            selected: true,
        });
    }

    // Create output directory and write files
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Cannot create output directory: {}", e))?;

    for entry in &files {
        let full_path = output_dir.join(&entry.path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create directory {}: {}", parent.display(), e))?;
        }
        std::fs::write(&full_path, &entry.content)
            .map_err(|e| format!("Cannot write file {}: {}", full_path.display(), e))?;
    }

    Ok(files)
}

pub fn get_archive_info(archive_path: &Path) -> Result<ArchiveInfo, String> {
    let mut file = std::fs::File::open(archive_path)
        .map_err(|e| format!("Cannot open archive: {}", e))?;

    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).map_err(|e| format!("Read error: {}", e))?;
    if &magic != MAGIC {
        return Err("Invalid archive format".to_string());
    }

    let mut version_buf = [0u8; 2];
    file.read_exact(&mut version_buf).map_err(|e| format!("Read error: {}", e))?;

    let mut _flags_buf = [0u8; 1];
    file.read_exact(&mut _flags_buf).map_err(|e| format!("Read error: {}", e))?;

    let mut count_buf = [0u8; 4];
    file.read_exact(&mut count_buf).map_err(|e| format!("Read error: {}", e))?;
    let count = u32::from_le_bytes(count_buf);

    let mut entries = Vec::new();
    let mut total_orig: u64 = 0;
    let mut total_comp: u64 = 0;

    for _ in 0..count {
        let mut name_len_buf = [0u8; 2];
        file.read_exact(&mut name_len_buf).map_err(|e| format!("Read error: {}", e))?;
        let name_len = u16::from_le_bytes(name_len_buf) as usize;

        let mut name_bytes = vec![0u8; name_len];
        file.read_exact(&mut name_bytes).map_err(|e| format!("Read error: {}", e))?;
        let name = String::from_utf8(name_bytes).map_err(|_| "Invalid UTF-8".to_string())?;

        let mut padding = [0u8; 4];
        file.read_exact(&mut padding).map_err(|e| format!("Read error: {}", e))?;

        let mut orig_buf = [0u8; 8];
        file.read_exact(&mut orig_buf).map_err(|e| format!("Read error: {}", e))?;
        let orig_size = u64::from_le_bytes(orig_buf);

        let mut comp_buf = [0u8; 8];
        file.read_exact(&mut comp_buf).map_err(|e| format!("Read error: {}", e))?;
        let comp_size = u64::from_le_bytes(comp_buf);

        let mut _offset_buf = [0u8; 8];
        file.read_exact(&mut _offset_buf).map_err(|e| format!("Read error: {}", e))?;

        total_orig += orig_size;
        total_comp += comp_size;

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
