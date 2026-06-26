use crate::hasher;
use crate::types::{DuplicateGroup, FileEntry, ScanStats};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn find_duplicates(
    entries: Vec<FileEntry>,
) -> anyhow::Result<(Vec<DuplicateGroup>, ScanStats)> {
    let total_files = entries.len() as u64;
    let total_size: u64 = entries.iter().map(|e| e.size).sum();
    // 第一层：按文件大小分组，大小唯一的直接排除
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    for entry in &entries {
        by_size
            .entry(entry.size)
            .or_default()
            .push(entry.path.clone());
    }
    by_size.retain(|_, v| v.len() >= 2);
    // 第二层：计算头部 8KB 的快速哈希来进一步筛选
    let mut after_quick: Vec<(u64, Vec<PathBuf>)> = Vec::new();
    for (&size, paths) in &by_size {
        let partial_hashes = hasher::compute_quick_hashes_parallel(paths, size);
        let mut groups: HashMap<[u8; 32], Vec<PathBuf>> = HashMap::new();
        for h in partial_hashes {
            groups.entry(h.quick_hash.0).or_default().push(h.path);
        }
        // 只保留快速哈希也相同的组（至少2个文件）
        for (_, files) in groups {
            if !files.is_empty() && files.len() > 1 {
                after_quick.push((size, files));
            }
        }
    }
    // 第三层：完整文件哈希 — 对通过前两层筛选的文件做全量哈希
    let mut after_full: Vec<(u64, Vec<PathBuf>)> = Vec::new();
    for (size, paths) in &after_quick {
        let full_hashes = hasher::compute_full_hashes_parallel(paths, *size);
        let mut map: HashMap<[u8; 32], Vec<PathBuf>> = HashMap::new();
        for fh in full_hashes {
            let entry = map.entry(fh.hash.0).or_default();
            entry.push(fh.path);
        }
        for (_, files) in map.drain() {
            if files.len() >= 2 {
                after_full.push((*size, files));
            }
        }
    }
    // 第四层：逐字节比对，防止哈希碰撞（概率极低但做最终确认）
    let mut result: Vec<DuplicateGroup> = Vec::new();
    for (size, files) in &after_full {
        if !hasher::verify_group_identical(files)? {
            continue;
        }
        let rep_hash = hasher::compute_full_hash(&files[0], *size)
            .map(|fh| fh.hash)
            .unwrap_or_else(|_| crate::types::HashBytes::from_blake3(blake3::hash(b"")));
        result.push(DuplicateGroup {
            size: *size,
            hash: rep_hash,
            files: files.clone(),
        });
    }
    // 统计可节省的空间
    let wasted_bytes = result
        .iter()
        .map(|g| g.size * (g.files.len() as u64 - 1))
        .sum();

    let stats = ScanStats {
        total_files,
        total_size,
        duplicate_groups: result.len(),
        wasted_bytes,
    };

    Ok((result, stats))
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
