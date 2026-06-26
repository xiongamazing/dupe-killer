use clap::Parser;
use colored::Colorize;
use dupe_killer::cli::Args;
use dupe_killer::{duplicates, output, scanner};
use std::process;

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    let min_size = args.min_size.unwrap_or(0);

    if !args.path.exists() {
        anyhow::bail!("path does not exist: {}", args.path.display());
    }
    if !args.path.is_dir() {
        anyhow::bail!("path is not a directory: {}", args.path.display());
    }

    eprintln!(
        "Scanning {} (min size: {} bytes)...",
        args.path.display(),
        min_size
    );
    let entries = scanner::scan(&args.path, min_size)?;
    eprintln!("Found {} files to analyze.", entries.len());

    if entries.is_empty() {
        println!("No files found matching the criteria.");
        return Ok(());
    }

    eprintln!("Analyzing for duplicates...");
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
        println!(
            "{} {}",
            "ℹ".cyan(),
            "Dry run mode — no files were deleted.".dimmed()
        );
    }

    Ok(())
}
