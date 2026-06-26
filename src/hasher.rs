use crate::types::{FullHash, HashBytes, PartialHash};
use rayon::prelude::*;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const HEADER_SIZE: usize = 8192;

pub fn compute_quick_hash(path: &Path, size: u64) -> anyhow::Result<PartialHash> {
    let mut f = fs::File::open(path)?;
    let len = HEADER_SIZE.min(size as usize);
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf)?;

    Ok(PartialHash {
        path: path.to_path_buf(),
        size,
        quick_hash: HashBytes::from_blake3(blake3::hash(&buf)),
    })
}

pub fn compute_full_hash(path: &Path, size: u64) -> anyhow::Result<FullHash> {
    let mut f = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 65536];

    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(FullHash {
        path: path.to_path_buf(),
        size,
        hash: HashBytes::from_blake3(hasher.finalize()),
    })
}

pub fn verify_identical(a: &Path, b: &Path) -> anyhow::Result<bool> {
    let mut fa = fs::File::open(a)?;
    let mut fb = fs::File::open(b)?;
    let mut ba = [0u8; 65536];
    let mut bb = [0u8; 65536];

    loop {
        let na = fa.read(&mut ba)?;
        let nb = fb.read(&mut bb)?;
        if na != nb {
            return Ok(false);
        }
        if na == 0 {
            break;
        }
        if ba[..na] != bb[..nb] {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn compute_quick_hashes_parallel(paths: &[PathBuf], size: u64) -> Vec<PartialHash> {
    paths
        .par_iter()
        .filter_map(|p| {
            compute_quick_hash(p, size)
                .map_err(|e| eprintln!("Warning: skipping {}: {e}", p.display()))
                .ok()
        })
        .collect()
}

pub fn compute_full_hashes_parallel(paths: &[PathBuf], size: u64) -> Vec<FullHash> {
    let mut results = Vec::with_capacity(paths.len());
    let hashes: Vec<_> = paths
        .par_iter()
        .filter_map(|p| match compute_full_hash(p, size) {
            Ok(h) => Some(h),
            Err(e) => {
                eprintln!("Warning: skipping full hash for {}: {e}", p.display());
                None
            }
        })
        .collect();
    results.extend(hashes);
    results
}

pub fn verify_group_identical(files: &[PathBuf]) -> anyhow::Result<bool> {
    if files.len() < 2 {
        return Ok(true);
    }
    let first = &files[0];
    for f in &files[1..] {
        if !verify_identical(first, f)? {
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
