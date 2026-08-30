//! ZIP create/extract, fully in memory, wasm32-safe.
//!
//! Security guards:
//! - Zip-slip: entry names are flattened to their final path component, so a
//!   crafted "../../etc/passwd" entry can never influence where a caller
//!   saves the file.
//! - Zip bombs: extraction stops past hard total/per-file limits instead of
//!   exhausting browser memory.

#![forbid(unsafe_code)]

use std::io::{Cursor, Read, Write};

use pz_core::{OutputFile, PzError};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Hard ceilings for in-memory extraction (browser tabs get ~2-4 GB).
const MAX_TOTAL_UNCOMPRESSED: u64 = 1 << 30; // 1 GiB
const MAX_FILE_UNCOMPRESSED: u64 = 512 << 20; // 512 MiB

/// Bundle files into one deflate-compressed ZIP.
pub fn create(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>, PzError> {
    if files.is_empty() {
        return Err(PzError::Invalid("add at least one file".into()));
    }
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, bytes) in files {
        let safe_name = sanitize_path(name);
        writer
            .start_file(safe_name, options)
            .map_err(|e| PzError::Failed(format!("could not add \"{name}\": {e}")))?;
        writer
            .write_all(bytes)
            .map_err(|e| PzError::Failed(format!("could not write \"{name}\": {e}")))?;
    }
    let cursor = writer
        .finish()
        .map_err(|e| PzError::Failed(format!("could not finalize archive: {e}")))?;
    Ok(cursor.into_inner())
}

/// Extract every file entry of a ZIP archive.
pub fn extract(bytes: &[u8]) -> Result<Vec<OutputFile>, PzError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| PzError::Failed(format!("could not read archive: {e}")))?;
    let mut outputs = Vec::new();
    let mut total: u64 = 0;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| PzError::Failed(format!("could not read entry {i}: {e}")))?;
        if entry.is_dir() {
            continue;
        }
        // Fast reject on the declared sizes — but headers can lie, so the
        // same ceilings are enforced again on the actual inflated bytes.
        if entry.size() > MAX_FILE_UNCOMPRESSED {
            return Err(PzError::Unsupported(format!(
                "\"{}\" is larger than the in-browser limit ({} MB)",
                entry.name(),
                MAX_FILE_UNCOMPRESSED >> 20
            )));
        }
        if total.saturating_add(entry.size()) > MAX_TOTAL_UNCOMPRESSED {
            return Err(PzError::Unsupported(
                "archive expands past the in-browser limit (1 GB)".into(),
            ));
        }
        let name = sanitize(entry.name());
        let budget = MAX_FILE_UNCOMPRESSED.min(MAX_TOTAL_UNCOMPRESSED - total);
        let declared = entry.size();
        // Capacity from the declared size, capped: a lying header must not
        // be able to force a huge allocation for a tiny entry.
        let mut data = Vec::with_capacity(declared.min(1 << 20) as usize);
        (&mut entry)
            .take(budget + 1)
            .read_to_end(&mut data)
            .map_err(|e| PzError::Failed(format!("could not extract \"{name}\": {e}")))?;
        if data.len() as u64 > budget || data.len() as u64 > declared {
            return Err(PzError::Unsupported(format!(
                "\"{name}\" inflates past its declared size — refusing (possible zip bomb)"
            )));
        }
        total += data.len() as u64;
        outputs.push(OutputFile {
            name,
            mime: "application/octet-stream",
            bytes: data,
        });
    }
    if outputs.is_empty() {
        return Err(PzError::Invalid("archive contains no files".into()));
    }
    Ok(outputs)
}

/// Neutralize a path for ARCHIVE ENTRY names while keeping its folder
/// structure: forward slashes only, and every empty/`.`/`..`/drive-ish
/// component dropped. Folder drops feed real relative paths ("photos/
/// raw/img.png") into Create ZIP, and flattening them would collide
/// same-named files from different subfolders.
fn sanitize_path(name: &str) -> String {
    let joined = name
        .replace('\\', "/")
        .split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != ".." && !s.ends_with(':'))
        .collect::<Vec<_>>()
        .join("/");
    if joined.is_empty() {
        "file".to_string()
    } else {
        joined
    }
}

/// Keep only the final path component and strip anything path-traversal-ish.
/// Extraction stays maximally strict: entries become flat browser
/// downloads, so structure is worthless there and hostile names aren't.
fn sanitize(name: &str) -> String {
    name.replace('\\', "/")
        .split('/')
        .rfind(|s| !s.is_empty() && *s != "." && *s != "..")
        .map(str::to_string)
        .unwrap_or_else(|| "file".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_unzip_roundtrip() {
        let files = vec![
            ("a.txt".to_string(), b"hello".to_vec()),
            ("b.bin".to_string(), vec![0u8; 1000]),
        ];
        let archive = create(&files).unwrap();
        let out = extract(&archive).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "a.txt");
        assert_eq!(out[0].bytes, b"hello");
        assert_eq!(out[1].bytes.len(), 1000);
    }

    #[test]
    fn deflate_actually_compresses() {
        let files = vec![("zeros.bin".to_string(), vec![0u8; 100_000])];
        let archive = create(&files).unwrap();
        assert!(archive.len() < 10_000);
    }

    #[test]
    fn sanitizes_traversal_names() {
        assert_eq!(sanitize("../../etc/passwd"), "passwd");
        assert_eq!(sanitize("dir/sub/file.txt"), "file.txt");
        assert_eq!(sanitize("windows\\path\\x.doc"), "x.doc");
        assert_eq!(sanitize("///"), "file");
    }

    #[test]
    fn create_keeps_structure_but_never_escapes() {
        assert_eq!(sanitize_path("photos/raw/img.png"), "photos/raw/img.png");
        assert_eq!(sanitize_path("../../etc/passwd"), "etc/passwd");
        assert_eq!(sanitize_path("/abs/path.txt"), "abs/path.txt");
        assert_eq!(sanitize_path("a/./b.txt"), "a/b.txt");
        assert_eq!(sanitize_path("C:\\evil\\x.doc"), "evil/x.doc");
        assert_eq!(sanitize_path("../.."), "file");
        let files = vec![
            ("dir/sub/a.txt".to_string(), b"x".to_vec()),
            ("dir/a.txt".to_string(), b"y".to_vec()),
        ];
        let archive = create(&files).unwrap();
        // Extraction flattens by design; both survive as distinct entries.
        assert_eq!(extract(&archive).unwrap().len(), 2);
    }

    #[test]
    fn rejects_empty_input() {
        assert!(create(&[]).is_err());
    }

    #[test]
    fn rejects_lying_size_headers() {
        let files = vec![("zeros.bin".to_string(), vec![0u8; 100_000])];
        let mut archive = create(&files).unwrap();
        // Understate the uncompressed size in the local file header
        // (offset 22) and the central directory entry (offset 24): the
        // declared-size checks pass, so the actual-bytes guard must fire.
        let lie = 10u32.to_le_bytes();
        let local = archive
            .windows(4)
            .position(|w| w == [0x50, 0x4b, 0x03, 0x04])
            .unwrap();
        archive[local + 22..local + 26].copy_from_slice(&lie);
        let central = archive
            .windows(4)
            .position(|w| w == [0x50, 0x4b, 0x01, 0x02])
            .unwrap();
        archive[central + 24..central + 28].copy_from_slice(&lie);
        let err = extract(&archive).unwrap_err();
        assert!(format!("{err:?}").contains("declared size"), "{err:?}");
    }
}
