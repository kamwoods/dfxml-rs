//! dedup - Detect and report duplicate files based on MD5 hashes in a DFXML file.
//!
//! This tool reads a DFXML file, groups files by their MD5 hash, and reports
//! statistics about duplicates. It can optionally list distinct files (unique
//! MD5s) or duplicate files.
//!
//! This is a Rust port of the Python `dedup.py` tool from the dfxml_python project.
//!
//! # Usage
//!
//! ```bash
//! dedup [OPTIONS] <DFXML_FILE>
//! ```
//!
//! # Examples
//!
//! ```bash
//! # Show summary statistics
//! dedup input.dfxml
//!
//! # List all distinct (unique) files
//! dedup --distinct input.dfxml
//!
//! # List all duplicate files with their duplicate count
//! dedup --dups input.dfxml
//!
//! # Filter output to files with a specific prefix
//! dedup --dups --prefix /home/user input.dfxml
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use clap::Parser;

use dfxml_rs::objects::HashType;
use dfxml_rs::reader::{DFXMLReader, Event};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Detect and report duplicate files based on MD5 hashes in a DFXML file.
#[derive(Parser, Debug)]
#[command(name = "dedup")]
#[command(version = VERSION)]
#[command(about = "Detect and report duplicate files based on MD5 hashes")]
#[command(
    long_about = "Reads a DFXML file, groups files by their MD5 hash, and reports \
    statistics about duplicates. This is a Rust port of the Python dedup.py tool."
)]
struct Args {
    /// Input DFXML file to process
    dfxml: String,

    /// Enable verbose output
    #[arg(long)]
    verbose: bool,

    /// Only output files with the given prefix
    #[arg(long)]
    prefix: Option<String>,

    /// Report the distinct (unique) files
    #[arg(long)]
    distinct: bool,

    /// Report the files that are duplicates, and give duplicate count
    #[arg(long)]
    dups: bool,
}

/// Tracks files grouped by their MD5 hash.
struct Dedup {
    /// Map from MD5 hash to list of filenames with that hash
    seen: HashMap<String, Vec<String>>,
    /// Total number of files processed
    files: usize,
    /// Number of files with MD5 hashes
    md5s: usize,
}

impl Dedup {
    fn new() -> Self {
        Self {
            seen: HashMap::new(),
            files: 0,
            md5s: 0,
        }
    }

    /// Process a file, recording its MD5 hash if present.
    fn process(&mut self, md5: Option<&str>, filename: Option<&str>) {
        self.files += 1;
        if let (Some(hash), Some(name)) = (md5, filename) {
            self.seen
                .entry(hash.to_string())
                .or_default()
                .push(name.to_string());
            self.md5s += 1;
        }
    }

    /// Returns the number of unique MD5 hashes seen.
    fn unique_count(&self) -> usize {
        self.seen.len()
    }

    /// Iterate over entries matching a predicate.
    fn report<F, C>(&self, predicate: F, mut callback: C)
    where
        F: Fn(&[String]) -> bool,
        C: FnMut(&str, &[String]),
    {
        for (md5, names) in &self.seen {
            if predicate(names) {
                callback(md5, names);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.verbose {
        eprintln!("Processing: {}", args.dfxml);
    }

    let file = File::open(&args.dfxml)?;
    let reader = BufReader::new(file);
    let dfxml_reader = DFXMLReader::from_reader(reader);

    let mut dedup = Dedup::new();

    for result in dfxml_reader {
        match result {
            Ok(Event::FileObject(fi)) => {
                let md5 = fi.hashes.get(HashType::Md5);
                let filename = fi.filename.as_deref();
                dedup.process(md5, filename);
            }
            Ok(_) => {
                // Ignore other events
            }
            Err(e) => {
                // Match Python behavior: continue on parse errors
                if args.verbose {
                    eprintln!("Warning: Parse error: {}", e);
                }
                break;
            }
        }
    }

    // Print summary statistics
    println!(
        "Total files: {}  total MD5s processed: {}  Unique MD5s: {}",
        format_number(dedup.files),
        format_number(dedup.md5s),
        format_number(dedup.unique_count())
    );

    // Report distinct files if requested
    if args.distinct {
        dedup.report(
            |names| names.len() == 1,
            |_md5, names| {
                let name = &names[0];
                if let Some(ref prefix) = args.prefix {
                    if !name.starts_with(prefix) {
                        return;
                    }
                }
                println!("distinct: {}", name);
            },
        );
    }

    // Report duplicate files if requested
    if args.dups {
        dedup.report(
            |names| names.len() > 1,
            |_md5, names| {
                for name in names {
                    if let Some(ref prefix) = args.prefix {
                        if !name.starts_with(prefix) {
                            continue;
                        }
                    }
                    println!("dups: {} {}", name, names.len());
                }
            },
        );
    }

    Ok(())
}

/// Format a number with thousand separators (matching Python's {:,} format).
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(1), "1");
        assert_eq!(format_number(12), "12");
        assert_eq!(format_number(123), "123");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(12345), "12,345");
        assert_eq!(format_number(123456), "123,456");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn test_dedup_new() {
        let dedup = Dedup::new();
        assert_eq!(dedup.files, 0);
        assert_eq!(dedup.md5s, 0);
        assert_eq!(dedup.unique_count(), 0);
    }

    #[test]
    fn test_dedup_process() {
        let mut dedup = Dedup::new();

        // Process file with MD5
        dedup.process(Some("abc123"), Some("/path/to/file1.txt"));
        assert_eq!(dedup.files, 1);
        assert_eq!(dedup.md5s, 1);
        assert_eq!(dedup.unique_count(), 1);

        // Process file without MD5
        dedup.process(None, Some("/path/to/file2.txt"));
        assert_eq!(dedup.files, 2);
        assert_eq!(dedup.md5s, 1);
        assert_eq!(dedup.unique_count(), 1);

        // Process duplicate (same MD5)
        dedup.process(Some("abc123"), Some("/path/to/file3.txt"));
        assert_eq!(dedup.files, 3);
        assert_eq!(dedup.md5s, 2);
        assert_eq!(dedup.unique_count(), 1); // Still only one unique MD5
    }

    #[test]
    fn test_dedup_report_distinct() {
        let mut dedup = Dedup::new();
        dedup.process(Some("unique1"), Some("/unique/file.txt"));
        dedup.process(Some("dup1"), Some("/dup/file1.txt"));
        dedup.process(Some("dup1"), Some("/dup/file2.txt"));

        let mut distinct_count = 0;
        dedup.report(
            |names| names.len() == 1,
            |_md5, _names| {
                distinct_count += 1;
            },
        );
        assert_eq!(distinct_count, 1);
    }

    #[test]
    fn test_dedup_report_dups() {
        let mut dedup = Dedup::new();
        dedup.process(Some("unique1"), Some("/unique/file.txt"));
        dedup.process(Some("dup1"), Some("/dup/file1.txt"));
        dedup.process(Some("dup1"), Some("/dup/file2.txt"));

        let mut dup_count = 0;
        dedup.report(
            |names| names.len() > 1,
            |_md5, names| {
                dup_count += names.len();
            },
        );
        assert_eq!(dup_count, 2);
    }
}
