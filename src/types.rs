use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

/// 扫描到的文件信息
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
}

/// 文件头部 8KB 的快速哈希结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialHash {
    pub path: PathBuf,
    pub size: u64,
    pub quick_hash: HashBytes,
}

/// 完整文件的哈希结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullHash {
    pub path: PathBuf,
    pub size: u64,
    pub hash: HashBytes,
}

/// 一组重复文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub size: u64,
    pub hash: HashBytes,
    pub files: Vec<PathBuf>,
}

/// 扫描统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStats {
    pub total_files: u64,
    pub total_size: u64,
    pub duplicate_groups: usize,
    pub wasted_bytes: u64,
}

/// 32 字节哈希值，包装了 Blake3 的 digest
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HashBytes(pub [u8; 32]);

impl HashBytes {
    pub fn from_blake3(h: blake3::Hash) -> Self {
        Self(h.into())
    }

    pub fn to_blake3(self) -> blake3::Hash {
        blake3::Hash::from(self.0)
    }
}

impl Serialize for HashBytes {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self)
    }
}

impl<'de> Deserialize<'de> for HashBytes {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let bytes = hex_to_bytes(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom("HashBytes must be 32 bytes"));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl std::fmt::Display for HashBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err("hex string must have even length".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}
