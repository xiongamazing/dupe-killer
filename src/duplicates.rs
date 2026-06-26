use crate::hasher;
use crate::types::{DuplicateGroup, FileEntry, ScanStats};
use std::collections::HashMap;
use std::path::PathBuf;

/// Four-layer progressive duplicate detection.
///
/// 1. Group by file size → 2. Quick hash (first 8 KB) → 3. Full hash → 4. Byte-by-byte verify.
///
/// Hashing failures for individual files are logged and skipped.
pub fn find_duplicates(
    entries: Vec<FileEntry>,
) -> anyhow::Result<(Vec<DuplicateGroup>, ScanStats)> {
    let total_files = entries.len() as u64;
    let total_size: u64 = entries.iter().map(|e| e.size).sum();

    // Layer 1: group by file size
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    for entry in &entries {
        by_size
            .entry(entry.size)
            .or_default()
            .push(entry.path.clone());
    }

    by_size.retain(|_, paths| paths.len() >= 2);

    // Layer 2: quick hash (first 8 KB)
    let mut quick_hash_groups: Vec<(u64, Vec<PathBuf>)> = Vec::new();

    for (&size, paths) in &by_size {
        let partials = hasher::compute_quick_hashes_parallel(paths, size);

        let mut by_quick: HashMap<[u8; 32], Vec<PathBuf>> = HashMap::new();
        for ph in partials {
            by_quick.entry(ph.quick_hash.0).or_default().push(ph.path);
        }

        for (_, group) in by_quick {
            if group.len() >= 2 {
                quick_hash_groups.push((size, group));
            }
        }
    }

    // Layer 3: full hash
    let mut full_hash_groups: Vec<(u64, Vec<PathBuf>)> = Vec::new();

    for (size, paths) in &quick_hash_groups {
        let fulls = hasher::compute_full_hashes_parallel(paths, *size);

        let mut by_full: HashMap<[u8; 32], Vec<PathBuf>> = HashMap::new();
        for fh in fulls {
            by_full.entry(fh.hash.0).or_default().push(fh.path);
        }

        for (_, group) in by_full {
            if group.len() >= 2 {
                full_hash_groups.push((*size, group));
            }
        }
    }

    // Layer 4: byte-by-byte verification
    let mut duplicate_groups: Vec<DuplicateGroup> = Vec::new();

    for (size, paths) in &full_hash_groups {
        if hasher::verify_group_identical(paths)? {
            let hash = hasher::compute_full_hash(&paths[0], *size)
                .map(|fh| fh.hash)
                .unwrap_or_else(|_| crate::types::HashBytes::from_blake3(blake3::hash(&[])));

            duplicate_groups.push(DuplicateGroup {
                size: *size,
                hash,
                files: paths.clone(),
            });
        }
    }

    let mut wasted_bytes: u64 = 0;
    for group in &duplicate_groups {
        wasted_bytes += group.size * (group.files.len() as u64 - 1);
    }

    let stats = ScanStats {
        total_files,
        total_size,
        duplicate_groups: duplicate_groups.len(),
        wasted_bytes,
    };

    Ok((duplicate_groups, stats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileEntry;
    use std::fs;
    use std::io::Write;
    use std::time::SystemTime;

    fn make_entry(path: &std::path::Path, content: &[u8]) -> FileEntry {
        let mut f = fs::File::create(path).unwrap();
        f.write_all(content).unwrap();
        FileEntry {
            path: path.to_path_buf(),
            size: content.len() as u64,
            modified: SystemTime::now(),
        }
    }

    #[test]
    fn test_duplicate_grouping() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");

        let content_a = b"this is file A - duplicated content";
        let content_b = b"this is file B - different content entirely";

        let path_a1 = dir.path().join("a1.txt");
        let path_a2 = dir.path().join("a2.txt");
        let path_b1 = dir.path().join("b1.txt");

        let entries = vec![
            make_entry(&path_a1, content_a),
            make_entry(&path_a2, content_a),
            make_entry(&path_b1, content_b),
        ];

        let (groups, stats) = find_duplicates(entries).expect("find_duplicates failed");

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 2);
        assert_eq!(stats.total_files, 3);
        assert_eq!(stats.duplicate_groups, 1);
        assert_eq!(stats.wasted_bytes, content_a.len() as u64);
    }

    #[test]
    fn test_different_files_not_grouped() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");

        let path1 = dir.path().join("file1.txt");
        let path2 = dir.path().join("file2.txt");
        let path3 = dir.path().join("file3.txt");

        fs::write(&path1, b"111111111111111111111111111111111111111111").unwrap();
        fs::write(&path2, b"222222222222222222222222222222222222222222").unwrap();
        fs::write(&path3, b"333333333333333333333333333333333333333333").unwrap();

        let entries = vec![
            FileEntry {
                path: path1,
                size: 42,
                modified: SystemTime::now(),
            },
            FileEntry {
                path: path2,
                size: 42,
                modified: SystemTime::now(),
            },
            FileEntry {
                path: path3,
                size: 42,
                modified: SystemTime::now(),
            },
        ];

        let (groups, _stats) = find_duplicates(entries).expect("find_duplicates failed");
        assert!(groups.is_empty(), "no duplicates should be found");
    }

    #[test]
    fn test_no_duplicates_with_unique_files() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");

        let path1 = dir.path().join("small.txt");
        let path2 = dir.path().join("big.txt");

        fs::write(&path1, b"tiny").unwrap();
        fs::write(
            &path2,
            b"this is a much larger file with completely different content",
        )
        .unwrap();

        let entries = vec![
            FileEntry {
                path: path1,
                size: 4,
                modified: SystemTime::now(),
            },
            FileEntry {
                path: path2,
                size: 60,
                modified: SystemTime::now(),
            },
        ];

        let (groups, stats) = find_duplicates(entries).expect("find_duplicates failed");
        assert!(groups.is_empty());
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.duplicate_groups, 0);
    }
}
