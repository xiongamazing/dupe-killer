# dupe-killer

A fast command-line duplicate file finder written in Rust, using a **four-layer progressive hashing algorithm** to identify duplicates with minimal I/O.

## Features

- **Four-layer dedup**: size → quick hash (8 KB) → full Blake3 hash → byte-by-byte verify
- **Parallel hashing**: uses `rayon` for multi-core file hashing
- **Colored terminal output**: clean table with groups, sizes, and wasted space
- **JSON export**: structured output for scripting
- **Safe deletion**: generates reviewable scripts (`.ps1` on Windows, `.sh` on Unix)

## Installation

```bash
git clone https://github.com/xiongamazing/dupe-killer.git
cd dupe-killer
cargo build --release
```

The binary will be at `target/release/dupe-killer` (or `dupe-killer.exe` on Windows).

## Usage

```bash
# Basic scan
dupe-killer /path/to/directory

# Only files larger than 1 MB
dupe-killer /path/to/directory --min-size 1MB

# JSON output
dupe-killer /path/to/directory --json

# Preview without deleting
dupe-killer /path/to/directory --dry-run

# Generate deletion script
dupe-killer /path/to/directory --delete-script cleanup.sh
```

### Options

| Option | Description |
|--------|-------------|
| `<PATH>` | Directory to scan (required) |
| `--min-size <SIZE>` | Minimum file size, e.g. `1MB`, `500KB`, `100B` |
| `--json` | Output results as JSON |
| `--dry-run` | Show results without deleting |
| `--delete-script <FILE>` | Generate a deletion script for review |

## How It Works

```
Layer 1: Group by file size        →  discard unique-size files
Layer 2: Quick hash (first 8 KB)   →  discard files with different beginnings
Layer 3: Full Blake3 hash          →  identify probable duplicates
Layer 4: Byte-by-byte verification →  eliminate hash collisions
```

## Example Output

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

## Project Structure

```
src/
├── main.rs        # Entry point
├── lib.rs         # Module declarations
├── cli.rs         # CLI argument parsing (clap)
├── scanner.rs     # Recursive directory scanning (walkdir)
├── hasher.rs      # Blake3 hashing with rayon parallelism
├── duplicates.rs  # Four-layer dedup algorithm
├── output.rs      # Table / JSON / delete script output
└── types.rs       # Shared data structures
```

## Requirements

- Rust 1.80+

## License

MIT
