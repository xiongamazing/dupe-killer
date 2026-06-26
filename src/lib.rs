//! # dupe-killer — 命令行重复文件查找器
//!
//! 基于四层渐进式哈希算法（大小 → 快速哈希 → 完整哈希 → 逐字节比对）。
//!
//! ## 模块
//!
//! - `types` — 数据结构定义
//! - `cli` — 命令行参数解析
//! - `scanner` — 目录递归扫描
//! - `hasher` — Blake3 哈希计算 + rayon 并行
//! - `duplicates` — 四层查重核心算法
//! - `output` — 终端表格 / JSON / 删除脚本输出

pub mod cli;
pub mod duplicates;
pub mod hasher;
pub mod output;
pub mod scanner;
pub mod types;
