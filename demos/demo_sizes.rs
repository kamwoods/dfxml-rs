//! demo_sizes - Calculate file size statistics grouped by file extension.
//!
//! This demo reads a DFXML file and calculates statistics about file sizes
//! grouped by their file extension. It reports count, total size, average,
//! and standard deviation for each extension.
//!
//! This is a Rust port of the Python `demo_sizes.py` from the dfxml_python project.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example demo_sizes <filename.xml>
//! ```
//!
//! # Output
//!
//! The output is a formatted table with columns:
//! - Ext: File extension (or empty for files without extension)
//! - Count: Number of files with that extension
//! - Total: Total size in bytes
//! - Average: Average file size
//! - StdDev: Standard deviation of file sizes

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::BufReader;

use dfxml_rs::reader::{DFXMLReader, Event};

/// Statistics accumulator for a single file extension.
#[derive(Default)]
struct ExtStats {
    count: u64,
    sum: u64,
    sum_of_squares: f64,
}

impl ExtStats {
    fn add(&mut self, size: u64) {
        self.count += 1;
        self.sum += size;
        self.sum_of_squares += (size as f64).powi(2);
    }

    fn average(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum as f64 / self.count as f64
        }
    }

    fn stddev(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            let mean = self.average();
            let variance = self.sum_of_squares / self.count as f64 - mean.powi(2);
            // Handle floating point errors that might make variance slightly negative
            if variance < 0.0 {
                0.0
            } else {
                variance.sqrt()
            }
        }
    }
}

/// Extract the file extension from a filename.
fn get_extension(filename: &str) -> String {
    if let Some(pos) = filename.rfind('.') {
        // Make sure there's something after the dot and it's not a hidden file
        let ext = &filename[pos + 1..];
        if !ext.is_empty() && pos > 0 {
            return ext.to_lowercase();
        }
    }
    String::new()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <filename.xml>", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];
    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let dfxml_reader = DFXMLReader::from_reader(reader);

    let mut stats: HashMap<String, ExtStats> = HashMap::new();

    for result in dfxml_reader {
        match result {
            Ok(Event::FileObject(fi)) => {
                if let (Some(filename), Some(filesize)) = (&fi.filename, fi.filesize) {
                    let ext = get_extension(filename);
                    stats.entry(ext).or_default().add(filesize);
                }
            }
            Ok(_) => {
                // Ignore other events
            }
            Err(e) => {
                eprintln!("Error parsing DFXML: {}", e);
                return Err(e.into());
            }
        }
    }

    // Print header
    println!(
        "{:>8}    {:>8} {:>12} {:>12} {:>12}",
        "Ext", "Count", "Total", "Average", "StdDev"
    );

    // Sort extensions for consistent output
    let mut extensions: Vec<_> = stats.keys().collect();
    extensions.sort();

    // Print statistics for each extension
    for ext in extensions {
        let s = &stats[ext];
        let display_ext = if ext.is_empty() { "(none)" } else { ext };
        println!(
            "{:>8}    {:>8} {:>12} {:>12.1} {:>12.1}",
            display_ext,
            s.count,
            s.sum,
            s.average(),
            s.stddev()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_extension() {
        assert_eq!(get_extension("file.txt"), "txt");
        assert_eq!(get_extension("file.TXT"), "txt");
        assert_eq!(get_extension("archive.tar.gz"), "gz");
        assert_eq!(get_extension("noextension"), "");
        assert_eq!(get_extension(".hidden"), "");
        assert_eq!(get_extension("file."), "");
        assert_eq!(get_extension("/path/to/file.pdf"), "pdf");
    }

    #[test]
    fn test_ext_stats_empty() {
        let stats = ExtStats::default();
        assert_eq!(stats.count, 0);
        assert_eq!(stats.sum, 0);
        assert_eq!(stats.average(), 0.0);
        assert_eq!(stats.stddev(), 0.0);
    }

    #[test]
    fn test_ext_stats_single() {
        let mut stats = ExtStats::default();
        stats.add(100);
        assert_eq!(stats.count, 1);
        assert_eq!(stats.sum, 100);
        assert_eq!(stats.average(), 100.0);
        assert_eq!(stats.stddev(), 0.0);
    }

    #[test]
    fn test_ext_stats_multiple() {
        let mut stats = ExtStats::default();
        stats.add(10);
        stats.add(20);
        stats.add(30);
        assert_eq!(stats.count, 3);
        assert_eq!(stats.sum, 60);
        assert_eq!(stats.average(), 20.0);
        // StdDev of [10, 20, 30] = sqrt(((10-20)^2 + (20-20)^2 + (30-20)^2) / 3)
        //                       = sqrt((100 + 0 + 100) / 3) = sqrt(200/3) ≈ 8.165
        let expected_stddev = (200.0_f64 / 3.0).sqrt();
        assert!((stats.stddev() - expected_stddev).abs() < 0.001);
    }
}
