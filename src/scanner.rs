use crate::types::FileEntry;
use std::path::Path;
use walkdir::WalkDir;

/// Recursively scan `dir` and collect [`FileEntry`] metadata for files larger than `min_size`.
///
/// Per-file errors (e.g. permission denied) are logged and skipped.
pub fn scan(dir: &Path, min_size: u64) -> anyhow::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| match e {
            Ok(entry) => Some(entry),
            Err(err) => {
                eprintln!("Warning: skipping unreadable entry: {err}");
                None
            }
        })
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                eprintln!(
                    "Warning: cannot read metadata for {}: {e}",
                    entry.path().display()
                );
                continue;
            }
        };

        let size = metadata.len();

        if size < min_size {
            continue;
        }

        let modified = match metadata.modified() {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "Warning: cannot read modification time for {}: {e}",
                    entry.path().display()
                );
                continue;
            }
        };

        entries.push(FileEntry {
            path: entry.path().to_path_buf(),
            size,
            modified,
        });
    }

    if entries.is_empty() && min_size == 0 {
        eprintln!("Note: no files found in {}", dir.display());
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_min_size_filter() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let dir_path = dir.path();

        let small_path = dir_path.join("small.txt");
        fs::write(&small_path, b"tiny").unwrap();

        let big_path = dir_path.join("big.txt");
        fs::write(&big_path, b"this is a much bigger file").unwrap();

        let entries = scan(dir_path, 10).expect("scan failed");

        let paths: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert!(
            paths.contains(&"big.txt"),
            "big file should be included. Found: {paths:?}"
        );
        assert!(
            !paths.contains(&"small.txt"),
            "small file should be excluded"
        );
        assert_eq!(entries.len(), 1, "expected 1 entry, got {entries:?}");
    }

    #[test]
    fn test_scan_skips_directories() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let dir_path = dir.path();

        let subdir = dir_path.join("subdir");
        fs::create_dir(&subdir).unwrap();

        let file_path = dir_path.join("file.txt");
        fs::write(&file_path, b"content").unwrap();

        let entries = scan(dir_path, 0).expect("scan failed");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, file_path);
    }
}
