# dupe-killer

基于 Rust 的命令行重复文件查找器。采用**四层渐进式哈希算法**，在最小化磁盘 I/O 的同时精准识别重复文件。

## 功能特点

- **四层查重**：文件大小 → 头部 8KB 快速哈希 → 完整 Blake3 哈希 → 逐字节比对
- **并行计算**：`rayon` 多核并行哈希，充分利用 CPU
- **彩色终端表格**：直观展示重复组、文件大小和可节省空间
- **JSON 导出**：结构化输出，方便脚本处理或配合 `jq` 使用
- **安全删除**：只生成删除脚本，人工审核后再执行，绝不直接操作文件
- **中文路径支持**：PowerShell 脚本使用 UTF-8 BOM 编码，中文路径无乱码
- **执行后自清理**：脚本执行完毕后自动删除自身
- **跨平台**：Windows 生成 `.ps1`，Linux/macOS 生成 `.sh`

## 快速开始

```bash
git clone https://github.com/xiongamazing/dupe-killer.git
cd dupe-killer
cargo build --release
```

编译后运行：

```bash
# 用 cargo 运行
cargo run -- "D:\系统图片\Screenshots"

# 或用编译好的可执行文件
.\target\release\dupe-killer.exe "D:\系统图片\Screenshots"
```

> 注意：不要双击 exe，这是命令行工具，必须在终端中带参数运行。

## 使用方法

```bash
# 预览扫描（安全，不删文件）
dupe-killer /path/to/dir --dry-run

# 只扫描 1MB 以上的大文件
dupe-killer /path/to/dir --min-size 1MB

# JSON 格式输出
dupe-killer /path/to/dir --json

# 生成删除脚本 → 审核 → 执行
dupe-killer /path/to/dir --delete-script cleanup.ps1
powershell -ExecutionPolicy Bypass -File cleanup.ps1
```

### 参数一览

| 参数 | 说明 | 示例 |
|------|------|------|
| `<PATH>` | 要扫描的目录（必填） | `"D:\我的图片"` |
| `--min-size <SIZE>` | 最小文件大小 | `--min-size 1MB` |
| `--json` | JSON 格式输出 | `--json` |
| `--dry-run` | 预览模式，不执行删除 | `--dry-run` |
| `--delete-script <FILE>` | 生成删除脚本 | `--delete-script cleanup.ps1` |

### 安全删除四步走

```
① dupe-killer /path --dry-run           → 预览，看看有哪些重复
② dupe-killer /path --delete-script X   → 生成删除脚本
③ 用编辑器打开脚本，逐条审核           → 人工确认
④ powershell -File X   (或 bash X)     → 执行删除
```

## 算法原理

```
第一层：按文件大小分组        →  大小唯一的文件直接排除
第二层：头部 8KB Blake3 哈希   →  开头不同的文件快速排除
第三层：完整 Blake3 哈希       →  确认疑似重复
第四层：逐字节比对             →  消除哈希碰撞的理论可能
```

## 示例输出

```
=== Duplicate Files Report ===

Group 1 180 KB × 2 files duplicates — waste: 180 KB
  keep : D:\系统图片\Screenshots\屏幕截图.png
  del  : D:\系统图片\Screenshots\屏幕截图 - 副本.png

─────────────────────────────────────────
Scan Summary:
  Total files scanned : 41
  Total data scanned  : 6.8 MB
  Duplicate groups    : 1
  Wasted space        : 180 KB
  (2.6% of total data is duplicate)
```

## 项目结构

```
src/
├── main.rs        # 程序入口
├── lib.rs         # 模块声明
├── types.rs       # 数据结构定义
├── cli.rs         # 命令行参数解析 (clap)
├── scanner.rs     # 递归目录扫描 (walkdir)
├── hasher.rs      # Blake3 哈希 + rayon 并行
├── duplicates.rs  # 四层查重核心算法
└── output.rs      # 表格 / JSON / 删除脚本输出
```

## 质量保证

```bash
cargo test                      # 15 个测试全部通过
cargo fmt --check               # 零格式问题
cargo clippy -- -D warnings     # 零警告
```

## 详细文档

更多内容请阅读 [USAGE.md](USAGE.md)，包括：完整操作流程、输出字段详解、删除脚本解读、常见问题解答。

## 环境要求

- Rust 1.80+

## 许可证

MIT
