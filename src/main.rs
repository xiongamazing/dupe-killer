use clap::Parser;
use colored::Colorize;
use dupe_killer::cli::Args;
use dupe_killer::{duplicates, output, scanner};
use std::process;

fn main() {
    if let Err(err) = run() {
        eprintln!("错误: {err}");
        process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    let min_size = args.min_size.unwrap_or(0);

    if !args.path.exists() {
        anyhow::bail!("路径不存在: {}", args.path.display());
    }
    if !args.path.is_dir() {
        anyhow::bail!("路径不是目录: {}", args.path.display());
    }

    eprintln!(
        "正在扫描 {} (最小文件大小: {} 字节)...",
        args.path.display(),
        min_size
    );
    let entries = scanner::scan(&args.path, min_size)?;
    eprintln!("找到 {} 个文件待分析。", entries.len());

    if entries.is_empty() {
        println!("没有找到符合条件的文件。");
        return Ok(());
    }

    eprintln!("正在分析重复文件...");
    let (groups, stats) = duplicates::find_duplicates(entries)?;

    if let Some(ref script_path) = args.delete_script {
        output::generate_delete_script(&groups, script_path)?;
    }

    if args.json {
        output::print_json(&groups, &stats)?;
    } else {
        output::print_table(&groups, &stats);
    }

    if args.dry_run {
        println!("{} {}", "ℹ".cyan(), "预览模式 — 没有文件被删除。".dimmed());
    }

    Ok(())
}
