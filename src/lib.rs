//! # dupe-killer — fast duplicate file finder
//!
//! Uses a four-layer progressive hashing algorithm (size → quick hash → full hash → byte verify).
//!
//! ## Quick start
//!
//! ```bash
//! cargo run -- /path/to/scan
//! cargo run -- /path/to/scan --min-size 1MB --json
//! cargo run -- /path/to/scan --dry-run
//! cargo run -- /path/to/scan --delete-script cleanup.sh
//! ```

pub mod cli;
pub mod duplicates;
pub mod hasher;
pub mod output;
pub mod scanner;
pub mod types;
