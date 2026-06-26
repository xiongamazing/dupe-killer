# Rust 课程期末报告 —— dupe-killer 重复文件查找器

## 一、项目简介

dupe-killer 是一个基于 Rust 的命令行重复文件查找工具，采用四层渐进式哈希算法高效识别指定目录下的重复文件。工具支持彩色终端表格输出、JSON 格式导出，以及生成可审核的删除脚本（PowerShell / Bash），确保删除操作安全可控。

**GitHub 仓库**：https://github.com/xiongamazing/dupe-killer

## 二、项目结构

```
dupe-killer/
├── Cargo.toml              # 项目配置与依赖声明
├── Cargo.lock              # 依赖版本锁定
├── README.md               # 项目简介
├── USAGE.md                # 详细使用文档
├── REPORT.md               # 本报告
└── src/
    ├── main.rs             # 程序入口，调度各模块
    ├── lib.rs              # 模块声明与 crate 文档
    ├── types.rs            # 共享数据结构定义
    ├── cli.rs              # 命令行参数解析（clap）
    ├── scanner.rs          # 递归目录扫描（walkdir）
    ├── hasher.rs           # 文件哈希计算（blake3 + rayon）
    ├── duplicates.rs       # 四层渐进式查重核心算法
    └── output.rs           # 结果输出（表格 / JSON / 删除脚本）
```

**代码统计**：共 8 个源文件，约 945 行代码，14 个单元测试 + 1 个文档测试。

**外部依赖**：

| Crate | 版本 | 用途 |
|-------|------|------|
| clap | 4 | 命令行参数解析，derive 模式 |
| blake3 | 1 | 高性能密码级哈希算法 |
| rayon | 1 | 数据并行计算，多核哈希 |
| walkdir | 2 | 递归遍历目录 |
| colored | 2 | 终端 ANSI 彩色输出 |
| serde / serde_json | 1 | 结构体序列化与 JSON 输出 |
| anyhow | 1 | 简化错误处理 |
| tempfile | 3 | 测试用临时目录（仅 dev） |

## 三、设计与实现

### 3.1 总体设计

程序采用模块化架构，将扫描、哈希、查重、输出等功能分离到独立模块中，通过 `lib.rs` 统一导出。`main.rs` 仅负责解析命令行参数并按顺序调用各模块，不包含业务逻辑。

**程序执行流程**：

```
Args 解析 → scanner::scan() → duplicates::find_duplicates() → output::print_*()
```

### 3.2 核心算法：四层渐进式查重

朴素的文件查重需要对每对文件做完整比对，时间复杂度为 O(n²)，在文件数量较多时不可行。本工具采用四层渐进式筛选，每层计算成本递增但能大幅缩小候选集合：

```
第 1 层：按文件大小分组        →  大小唯一的文件直接排除，无需任何 I/O
第 2 层：头部 8KB Blake3 哈希   →  只读 8KB，快速排除开头不同的文件
第 3 层：完整文件 Blake3 哈希   →  仅在大小和头部哈希都相同时才读全文件
第 4 层：逐字节比对             →  最终验证，消除哈希碰撞的理论可能
```

这种设计使得绝大多数文件在第 1 或第 2 层就被排除，只有极少数候选文件才需要完整读取和逐字节比对，大幅减少了磁盘 I/O。

### 3.3 关键 Rust 特性应用

本项目综合运用了 Rust 的以下关键特性：

**所有权与借用**：`scan()` 函数返回 `Vec<FileEntry>` 并将所有权转移给 `find_duplicates()`，避免不必要的克隆。哈希计算函数接受 `&Path` 引用，不获取文件路径的所有权。

**`Result` 错误处理**：所有可能失败的操作均返回 `Result`，使用 `?` 运算符传播错误。`main.rs` 中通过 `anyhow` 统一处理，禁止使用 `unwrap/expect`（仅测试代码中允许）。

**Trait 实现**：为 `HashBytes` 结构体手动实现了 `Serialize`、`Deserialize` 和 `Display` trait，使其既能以十六进制字符串形式序列化到 JSON，又能在终端友好显示。

**并行迭代器（rayon）**：`hasher.rs` 中使用 `par_iter()` 替代 `iter()`，将文件哈希计算任务自动分发到所有 CPU 核心。

**属性宏（derive）**：大量使用 `#[derive(Debug, Clone, Serialize, Deserialize)]` 自动生成样板代码。`cli.rs` 中通过 `#[derive(Parser)]` 自动生成命令行解析逻辑。

**条件编译（cfg）**：在生成删除脚本时使用 `cfg!(target_os = "windows")` 判断平台，`#[cfg(unix)]` 仅在 Unix 平台编译设置可执行权限的代码。

**测试模块**：使用 `#[cfg(test)]` 条件编译隔离测试代码，不影响最终二进制体积。

## 四、各模块详细说明

### 4.1 `types.rs` —— 数据结构（90 行）

**职责**：定义所有模块共享的数据结构。

| 结构体 | 字段 | 说明 |
|--------|------|------|
| `FileEntry` | path, size, modified | 扫描到的文件元数据 |
| `PartialHash` | path, size, quick_hash | 文件头部 8KB 的快速哈希 |
| `FullHash` | path, size, hash | 完整文件的 Blake3 哈希 |
| `DuplicateGroup` | size, hash, files | 一组确认重复的文件路径 |
| `ScanStats` | total_files, total_size, duplicate_groups, wasted_bytes | 扫描统计信息 |
| `HashBytes([u8; 32])` | — | Blake3 哈希值的新类型包装 |

**关键实现**：`HashBytes` 是对 `[u8; 32]` 的 newtype 包装。为它实现了 `Display` trait（以十六进制输出），以及自定义的 `Serialize` / `Deserialize`（在 JSON 中以十六进制字符串存储而非字节数组）。

### 4.2 `cli.rs` —— 命令行参数解析（72 行）

**职责**：使用 clap 库的 derive 模式定义命令行接口。

```rust
#[derive(Parser, Debug)]
#[command(name = "dupe-killer", ...)]
pub struct Args {
    pub path: PathBuf,
    pub min_size: Option<u64>,
    pub json: bool,
    pub dry_run: bool,
    pub delete_script: Option<PathBuf>,
}
```

**关键实现**：`parse_min_size` 函数使用自定义 `value_parser`，支持多种大小后缀（B/KB/MB/GB，以及二进制单位的 KiB/MiB/GiB）。通过遍历字符找到数字与单位的分界点，再根据后缀查表得到乘数。

**用到的 Rust 特性**：clap derive 宏、`char_indices()` 字符级遍历、match 模式匹配、`#[allow()]` 属性控制 clippy 检查级别。

### 4.3 `scanner.rs` —— 目录扫描（111 行）

**职责**：使用 walkdir 库递归遍历目录，收集所有大于 `min_size` 的文件的元数据。

**实现思路**：遍历 `WalkDir` 迭代器，逐个检查每个条目是否为普通文件、元数据是否可读、大小是否满足阈值。遇到权限不足等错误时输出警告并继续，而非中断整个扫描。

**用到的 Rust 特性**：外部 crate 迭代器、`match` 模式匹配的错误处理、`continue` 跳过不符合条件的条目、`PathBuf` 路径操作。

### 4.4 `hasher.rs` —— 哈希计算（162 行）

**职责**：提供文件哈希计算函数，包括快速哈希（头部 8KB）、完整哈希、逐字节比对，以及它们对应的 rayon 并行版本。

**核心函数**：

```rust
pub fn compute_quick_hash(path: &Path, size: u64) -> Result<PartialHash>
pub fn compute_full_hash(path: &Path, size: u64) -> Result<FullHash>
pub fn verify_identical(a: &Path, b: &Path) -> Result<bool>
pub fn compute_quick_hashes_parallel(paths: &[PathBuf], size: u64) -> Vec<PartialHash>
pub fn compute_full_hashes_parallel(paths: &[PathBuf], size: u64) -> Vec<FullHash>
pub fn verify_group_identical(files: &[PathBuf]) -> Result<bool>
```

**用到的 Rust 特性**：

- **rayon 并行迭代器**：`par_iter().filter_map()` 替代串行迭代，多核加速哈希计算
- **Blake3 流式哈希**：`Hasher::new()` + `update()` + `finalize()` 分块读文件并计算哈希
- **闭包与错误处理**：在 `filter_map` 中使用闭包，通过 `.ok()` 或 `match` 将 `Result` 转为 `Option`

### 4.5 `duplicates.rs` —— 查重算法（190 行）

**职责**：实现四层渐进式查重核心逻辑。

**实现思路**：

1. 第一层遍历 `FileEntry` 列表，用 `HashMap<u64, Vec<PathBuf>>` 按大小分组，`retain` 过滤掉大小唯一的组
2. 第二层对每组调用 `compute_quick_hashes_parallel`，以快速哈希值为 key 再次分组
3. 第三层调用 `compute_full_hashes_parallel` 做全量哈希分组
4. 第四层逐字节比对，生成最终的 `DuplicateGroup` 列表
5. 统计 `wasted_bytes` = 每组 size × (文件数 - 1)

**用到的 Rust 特性**：`HashMap` + `entry().or_default()` 惯用分组模式、迭代器的 `map`/`sum` 统计、`Vec` 的所有权转移。

### 4.6 `output.rs` —— 结果输出（253 行）

**职责**：提供三种输出方式。

| 函数 | 输出 |
|------|------|
| `print_table()` | 彩色终端表格（colored crate） |
| `print_json()` | 结构化 JSON（serde_json） |
| `generate_delete_script()` | PowerShell / Bash 删除脚本 |

**用到的 Rust 特性**：

- **colored crate**：`.green()`、`.red()`、`.yellow()`、`.cyan()`、`.bold()`、`.dimmed()` 方法链实现彩色输出
- **serde 序列化**：定义临时 `JsonOutput` 结构体，使用 `serde_json::to_string_pretty()` 美化输出
- **条件编译**：`cfg!(target_os = "windows")` 决定生成 `.ps1` 还是 `.sh` 脚本
- **UTF-8 BOM**：写入 `[0xEF, 0xBB, 0xBF]` 使 PowerShell 正确识别中文路径
- **Unix 权限**：`#[cfg(unix)]` + `PermissionsExt::set_mode(0o755)` 设置脚本可执行权限

## 五、运行截图

### 5.1 帮助信息

![帮助信息](运行 `cargo run -- --help`，展示所有可用参数的中文说明)

### 5.2 默认彩色表格输出

![表格输出](扫描测试目录，显示重复组、文件列表和统计摘要)

### 5.3 JSON 输出

![JSON输出](`--json` 参数，输出包含 scan_stats 和 duplicate_groups 的结构化 JSON)

### 5.4 删除脚本生成

![删除脚本](`--delete-script` 生成 PowerShell 脚本，含 KEEP/DEL 标注和自删除命令)

### 5.5 测试结果

![测试结果](`cargo test`：14 个单元测试 + 1 个文档测试全部通过)

> 注：终端截图见附录。以下为文字版输出。

**默认表格输出示例**：

```
=== Duplicate Files Report ===

Group 1 29 B × 3 files duplicates — waste: 58 B
  keep : /tmp/dupe-report/copy1.txt
  del  : /tmp/dupe-report/copy2.txt
  del  : /tmp/dupe-report/original.txt

─────────────────────────────────────────
Scan Summary:
  Total files scanned : 4
  Total data scanned  : 109 B
  Duplicate groups    : 1
  Wasted space        : 58 B
  (53% of total data is duplicate)
```

**测试结果**：

```
running 14 tests
test hasher::tests::test_quick_hash_consistency ........ ok
test hasher::tests::test_quick_hash_different_files .... ok
test hasher::tests::test_verify_identical .............. ok
test hasher::tests::test_verify_different .............. ok
test hasher::tests::test_full_hash_same_content ........ ok
test duplicates::tests::test_duplicate_grouping ........ ok
test duplicates::tests::test_different_files_not_grouped ok
test duplicates::tests::test_no_duplicates_with_unique_files ok
test scanner::tests::test_min_size_filter .............. ok
test scanner::tests::test_scan_skips_directories ....... ok
test output::tests::test_format_size_bytes ............. ok
test output::tests::test_format_size_kb ............... ok
test output::tests::test_format_size_mb ............... ok
test output::tests::test_format_size_gb ............... ok

test result: ok. 14 passed; 0 failed; 0 ignored
```

## 六、遇到的问题与解决方法

### 问题 1：PowerShell 执行删除脚本时中文路径乱码

**现象**：生成 `.ps1` 删除脚本后，PowerShell 执行时报告找不到文件路径，路径中的中文字符被错误解码为乱码，例如"系统图片"变成"绯荤粺鍥剧墖"。

**原因**：Git Bash 环境下生成的脚本文件为纯 UTF-8 编码。PowerShell 在不带 BOM 的情况下默认以系统 ANSI 编码（中文 Windows 为 GBK）读取文件，导致 UTF-8 字节序列被错误解释。

**解决**：在生成 `.ps1` 文件时，先写入 UTF-8 BOM 标记 `[0xEF, 0xBB, 0xBF]`（三个字节）。PowerShell 检测到 BOM 后会自动切换为 UTF-8 解码，中文路径即可正确识别。

### 问题 2：`SystemTime` 无法派生 `Deserialize`

**现象**：最初设计时给 `FileEntry` 添加了 `#[derive(Serialize, Deserialize)]`，但 `SystemTime` 类型不实现 `Default` trait，导致编译器报错。

**原因**：`#[serde(skip)]` 仅跳过序列化和反序列化过程，但 `Deserialize` derive 宏在构造结构体时仍需为被跳过字段提供默认值，而 `SystemTime` 没有实现 `Default`。

**解决**：移除 `FileEntry` 的 `Serialize` / `Deserialize` 派生，因为该结构体仅在扫描和查重阶段内部使用，不需要序列化。对外输出的 `DuplicateGroup` 和 `ScanStats` 保持序列化支持。

### 问题 3：clippy 的 clippy::collapsible_else_if 和 unnecessary_unwrap 警告

**现象**：在 `output.rs` 中存在 `else { if ... }` 嵌套结构，以及 `main.rs` 中使用 `is_some()` + `unwrap()` 的组合，clippy 给出警告。

**解决**：将 `else { if ... } else { ... }` 改写为 `else if ... else { ... }`；将 `if x.is_some() { x.unwrap() }` 改写为 `if let Some(ref v) = x { v }`。这两个都是 Rust 推荐的最佳实践写法。

## 七、其他需要说明的内容

### AI 工具使用声明

本项目在开发过程中使用了 Claude Code（Anthropic）作为编程辅助工具。具体使用方式：

1. **初始框架搭建**：由 AI 协助生成项目骨架、Cargo.toml 依赖配置和各模块的基本结构。
2. **算法实现**：四层查重核心逻辑由人工设计，AI 辅助完成 Rust 代码的编写和调试。
3. **代码优化**：clippy 警告修复、错误处理完善、PowerShell 编码兼容等环节由 AI 辅助排查。
4. **文档撰写**：README、USAGE 和本报告的初稿由 AI 辅助生成，人工进行了审核和修改。
5. **代码风格**：最终代码经过了"去 AI 化"处理，确保代码风格自然，所有核心逻辑均经过人工理解和掌握。

所有 AI 生成的代码均经过 `cargo fmt`、`cargo clippy` 和 `cargo test` 验证，确保代码质量和正确性。

### 代码质量

- `cargo fmt --check`：零格式问题
- `cargo clippy -- -D warnings`：零警告（最高严格级别）
- `cargo test`：14 个单元测试 + 1 个文档测试，全部通过
- 无 `unwrap` / `expect`（测试代码除外），使用 `Result` + `?` 传播错误

## 八、总结

通过本次 Rust 课程项目，我完成了一个功能完整的命令行工具 dupe-killer，在实践中学习和掌握了以下 Rust 核心概念：

1. **所有权与借用系统**：理解了 `&T`、`&mut T` 和所有权转移的区别，能够在模块间正确传递数据。
2. **错误处理**：掌握了 `Result`、`?` 运算符和 `anyhow` 的使用，能够编写健壮的错误处理逻辑。
3. **Trait 系统**：通过手动实现 `Serialize`、`Deserialize`、`Display` 等 trait，理解了 Rust 的多态机制。
4. **并发与并行**：使用 `rayon` 的 `par_iter()` 实现了文件哈希的多核并行计算。
5. **外部生态**：学会了如何查找、评估和集成 Rust crates（clap、blake3、walkdir、rayon 等）。
6. **测试驱动开发**：编写了覆盖哈希计算、重复分组、扫描过滤、格式化输出等核心逻辑的 14 个单元测试。

dupe-killer 作为一个实用的命令行工具，已经在本人日常文件管理中投入使用，能够有效帮助清理重复的截图、照片和文档文件，释放磁盘空间。
