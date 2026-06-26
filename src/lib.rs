//! # dupe-killer — 命令行重复文件查找器
//!
//! 基于四层渐进式哈希算法（大小 → 快速哈希 → 完整哈希 → 逐字节比对）。

pub mod cli;
pub mod duplicates;
pub mod hasher;
pub mod output;
pub mod scanner;
pub mod types;
