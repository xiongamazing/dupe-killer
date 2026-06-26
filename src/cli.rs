use clap::Parser;
use std::path::PathBuf;

/// 重复文件查找器
#[derive(Parser, Debug)]
#[command(
    name = "dupe-killer",
    version = env!("CARGO_PKG_VERSION"),
    about = "查找并管理重复文件",
    long_about = "基于四层渐进式哈希算法的重复文件查找工具，支持彩色终端输出、JSON 导出和删除脚本生成。"
)]
pub struct Args {
    /// 要扫描的目录路径
    #[arg(value_name = "PATH")]
    pub path: PathBuf,

    /// 最小文件大小，如 1MB、500KB、100B
    #[arg(long, value_name = "SIZE", value_parser = parse_min_size)]
    pub min_size: Option<u64>,

    /// 以 JSON 格式输出
    #[arg(long)]
    pub json: bool,

    /// 预览模式，不实际删除
    #[arg(long)]
    pub dry_run: bool,

    /// 生成删除脚本（Windows 下为 .ps1，Unix 下为 .sh）
    #[arg(long, value_name = "FILE")]
    pub delete_script: Option<PathBuf>,
}

/// 把 "1MB"、"500KB" 这样的字符串解析成字节数
///
/// ```
/// # use dupe_killer::cli::parse_min_size;
/// assert_eq!(parse_min_size("1MB").unwrap(), 1_000_000);
/// assert_eq!(parse_min_size("500KB").unwrap(), 500_000);
/// ```
pub fn parse_min_size(input: &str) -> Result<u64, String> {
    let input = input.trim().to_uppercase();

    // 把数字部分和单位部分拆开
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
        .map_err(|e| format!("无效的数字 '{num_str}': {e}"))?;

    if number < 0.0 {
        return Err("文件大小不能为负数".to_string());
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
                "不支持的单位 '{other}'，支持: B, KB, MB, GB（或 KiB, MiB, GiB）"
            ));
        }
    };

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok((number * multiplier as f64) as u64)
}
