use clap::Parser;
use std::path::PathBuf;

/// dupe-killer — find and manage duplicate files.
#[derive(Parser, Debug)]
#[command(
    name = "dupe-killer",
    version = env!("CARGO_PKG_VERSION"),
    about = "Find and manage duplicate files",
    long_about = "A fast duplicate file finder that uses a four-layer \
                  progressive hashing algorithm to identify duplicate \
                  files with minimal I/O. Supports colored terminal output, \
                  JSON export, and deletion script generation."
)]
pub struct Args {
    /// Directory path to scan recursively
    #[arg(value_name = "PATH")]
    pub path: PathBuf,

    /// Minimum file size, e.g. "1MB", "500KB", "100B"
    #[arg(long, value_name = "SIZE", value_parser = parse_min_size)]
    pub min_size: Option<u64>,

    /// Output results as JSON
    #[arg(long)]
    pub json: bool,

    /// Dry run: show results without deleting files
    #[arg(long)]
    pub dry_run: bool,

    /// Generate a deletion script (PowerShell on Windows, bash on Unix)
    #[arg(long, value_name = "FILE")]
    pub delete_script: Option<PathBuf>,
}

/// Parse a size string like "1MB", "500KB", "100" into bytes.
///
/// ```
/// # use dupe_killer::cli::parse_min_size;
/// assert_eq!(parse_min_size("1MB").unwrap(), 1_000_000);
/// assert_eq!(parse_min_size("500KB").unwrap(), 500_000);
/// ```
pub fn parse_min_size(input: &str) -> Result<u64, String> {
    let input = input.trim().to_uppercase();

    let (num_str, suffix) = {
        let idx = input
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(input.len());
        let num = &input[..idx];
        let suf = input[idx..].trim();
        (num, suf)
    };

    let number: f64 = num_str
        .parse()
        .map_err(|e| format!("invalid number '{num_str}': {e}"))?;

    if number < 0.0 {
        return Err("size must be non-negative".to_string());
    }

    let multiplier: u64 = match suffix {
        "" | "B" => 1,
        "K" | "KB" => 1_000,
        "M" | "MB" => 1_000_000,
        "G" | "GB" => 1_000_000_000,
        "KI" | "KIB" => 1_024,
        "MI" | "MIB" => 1_048_576,
        "GI" | "GIB" => 1_073_741_824,
        other => {
            return Err(format!(
                "unknown size suffix '{other}'. Supported: B, KB, MB, GB (or KiB, MiB, GiB)"
            ));
        }
    };

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok((number * multiplier as f64) as u64)
}
