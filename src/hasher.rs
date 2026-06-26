use crate::types::{FullHash, HashBytes, PartialHash};
use rayon::prelude::*;
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

const QUICK_HASH_BYTES: usize = 8192;

/// 计算文件头部 8KB 的 Blake3 哈希
pub fn compute_quick_hash(path: &Path, size: u64) -> anyhow::Result<PartialHash> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::with_capacity(QUICK_HASH_BYTES, file);
    let mut buffer = vec![0u8; QUICK_HASH_BYTES.min(size as usize)];

    reader.read_exact(&mut buffer)?;

    let quick_hash = HashBytes::from_blake3(blake3::hash(&buffer));

    Ok(PartialHash {
        path: path.to_path_buf(),
        size,
        quick_hash,
    })
}

/// 计算整个文件的 Blake3 哈希
pub fn compute_full_hash(path: &Path, size: u64) -> anyhow::Result<FullHash> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 65536];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = HashBytes::from_blake3(hasher.finalize());

    Ok(FullHash {
        path: path.to_path_buf(),
        size,
        hash,
    })
}

/// 逐字节比对两个文件是否完全一致（第四层验证）
pub fn verify_identical(a: &Path, b: &Path) -> anyhow::Result<bool> {
    let file_a = fs::File::open(a)?;
    let file_b = fs::File::open(b)?;

    let mut reader_a = BufReader::with_capacity(65536, file_a);
    let mut reader_b = BufReader::with_capacity(65536, file_b);

    let mut buf_a = [0u8; 65536];
    let mut buf_b = [0u8; 65536];

    loop {
        let n_a = reader_a.read(&mut buf_a)?;
        let n_b = reader_b.read(&mut buf_b)?;

        if n_a != n_b {
            return Ok(false);
        }
        if n_a == 0 {
            return Ok(true);
        }

        if buf_a[..n_a] != buf_b[..n_b] {
            return Ok(false);
        }
    }
}

/// 并行计算一批文件的快速哈希
pub fn compute_quick_hashes_parallel(entries: &[PathBuf], size: u64) -> Vec<PartialHash> {
    entries
        .par_iter()
        .filter_map(|path| match compute_quick_hash(path, size) {
            Ok(h) => Some(h),
            Err(e) => {
                eprintln!("警告: 跳过 {}: {e}", path.display());
                None
            }
        })
        .collect()
}

/// 并行计算一批文件的完整哈希
pub fn compute_full_hashes_parallel(paths: &[PathBuf], size: u64) -> Vec<FullHash> {
    paths
        .par_iter()
        .filter_map(|path| match compute_full_hash(path, size) {
            Ok(h) => Some(h),
            Err(e) => {
                eprintln!("警告: 跳过完整哈希 {}: {e}", path.display());
                None
            }
        })
        .collect()
}

/// 验证一组文件是否全部逐字节一致
pub fn verify_group_identical(files: &[PathBuf]) -> anyhow::Result<bool> {
    if files.len() < 2 {
        return Ok(true);
    }

    let first = &files[0];
    for other in &files[1..] {
        if !verify_identical(first, other)? {
            return Ok(false);
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_quick_hash_consistency() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let file_path = dir.path().join("test.txt");

        let content = "hello world this is a test file for hashing";
        let mut f = fs::File::create(&file_path).unwrap();
        f.write_all(content.as_bytes()).unwrap();

        let hash1 = compute_quick_hash(&file_path, content.len() as u64).expect("hash failed");
        let hash2 = compute_quick_hash(&file_path, content.len() as u64).expect("hash failed");

        assert_eq!(hash1.quick_hash, hash2.quick_hash);
    }

    #[test]
    fn test_quick_hash_different_files() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");

        let file_a = dir.path().join("a.txt");
        let mut f = fs::File::create(&file_a).unwrap();
        f.write_all(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .unwrap();

        let file_b = dir.path().join("b.txt");
        let mut f = fs::File::create(&file_b).unwrap();
        f.write_all(b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
            .unwrap();

        let hash_a = compute_quick_hash(&file_a, 40).expect("hash failed");
        let hash_b = compute_quick_hash(&file_b, 40).expect("hash failed");

        assert_ne!(hash_a.quick_hash, hash_b.quick_hash);
    }

    #[test]
    fn test_verify_identical() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");

        let content = b"some test content that is identical across files";
        let file_a = dir.path().join("a.txt");
        let file_b = dir.path().join("b.txt");

        fs::write(&file_a, content).unwrap();
        fs::write(&file_b, content).unwrap();

        assert!(verify_identical(&file_a, &file_b).expect("verify failed"));
    }

    #[test]
    fn test_verify_different() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");

        let file_a = dir.path().join("a.txt");
        let file_b = dir.path().join("b.txt");

        fs::write(&file_a, b"content one").unwrap();
        fs::write(&file_b, b"content two").unwrap();

        assert!(!verify_identical(&file_a, &file_b).expect("verify failed"));
    }

    #[test]
    fn test_full_hash_same_content() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");

        let content = b"this is the full content of both files, they are identical";
        let file_a = dir.path().join("a.txt");
        let file_b = dir.path().join("b.txt");

        fs::write(&file_a, content).unwrap();
        fs::write(&file_b, content).unwrap();

        let hash_a = compute_full_hash(&file_a, content.len() as u64).expect("hash failed");
        let hash_b = compute_full_hash(&file_b, content.len() as u64).expect("hash failed");

        assert_eq!(hash_a.hash, hash_b.hash);
    }
}
