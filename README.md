# dupe-killer

基于 Rust 的命令行重复文件查找器，采用**四层渐进式哈希算法**，在最小化磁盘 I/O 的同时精准识别重复文件。

## 功能特点

- **四层查重算法**：文件大小 → 头部 8KB 快速哈希 → 完整 Blake3 哈希 → 逐字节比对
- **并行计算**：使用 `rayon` 多核并行计算文件哈希
- **彩色终端输出**：简洁的表格展示重复组、文件大小和可节省空间
- **JSON 导出**：支持结构化 JSON 输出，方便脚本处理
- **安全删除**：生成删除脚本（Windows 生成 `.ps1`，Unix 生成 `.sh`），人工审核后再执行

## 安装

```bash
git clone https://github.com/xiongamazing/dupe-killer.git
cd dupe-killer
cargo build --release
```

编译后的可执行文件位于 `target/release/dupe-killer`（Windows 下为 `dupe-killer.exe`）。

## 使用方法

```bash
# 基本扫描
dupe-killer /path/to/directory

# 只扫描大于 1MB 的文件
dupe-killer /path/to/directory --min-size 1MB

# JSON 格式输出
dupe-killer /path/to/directory --json

# 预览模式，不执行删除
dupe-killer /path/to/directory --dry-run

# 生成删除脚本，审核后手动执行
dupe-killer /path/to/directory --delete-script cleanup.sh
```

### 参数说明

| 参数 | 说明 |
|------|------|
| `<PATH>` | 要扫描的目录路径（必填） |
| `--min-size <SIZE>` | 最小文件大小，如 `1MB`、`500KB`、`100B` |
| `--json` | 以 JSON 格式输出结果 |
| `--dry-run` | 只预览结果，不执行删除 |
| `--delete-script <FILE>` | 生成删除脚本供人工审核 |

## 算法原理

```
第一层：按文件大小分组        →  过滤掉大小唯一的文件
第二层：计算头部 8KB 快速哈希  →  过滤掉开头不同的文件
第三层：计算完整 Blake3 哈希   →  识别疑似重复文件
第四层：逐字节比对验证         →  消除哈希碰撞的可能
```

## 示例输出

```
=== Duplicate Files Report ===

Group 1 1.5 MB × 3 files duplicates — waste: 3.0 MB
  keep : /photos/original.jpg
  del  : /backup/original.jpg
  del  : /downloads/original.jpg

─────────────────────────────────────────
Scan Summary:
  Total files scanned : 1,234
  Total data scanned  : 5.2 GB
  Duplicate groups    : 12
  Wasted space        : 340 MB
  (6.5% of total data is duplicate)
```

## 项目结构

```
src/
├── main.rs        # 程序入口
├── lib.rs         # 模块声明
├── cli.rs         # 命令行参数解析 (clap)
├── scanner.rs     # 递归目录扫描 (walkdir)
├── hasher.rs      # Blake3 哈希计算 (rayon 并行)
├── duplicates.rs  # 四层渐进式查重算法
├── output.rs      # 终端表格 / JSON / 删除脚本输出
└── types.rs       # 共享数据结构
```

## 环境要求

- Rust 1.80+

## 许可证

MIT
