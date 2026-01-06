//! cat_partitions - Concatenate DFXML documents with partition offset handling.
//!
//! This tool reads multiple DFXML files, each prefixed with a partition offset,
//! and outputs a single DFXML document with all volumes and their file objects
//! combined. Partition numbers, offsets, and byte run `img_offset` attributes
//! are updated based on the provided offsets.
//!
//! This is a Rust port of the Python `cat_partitions.py` tool from the
//! dfxml_python project.
//!
//! # Usage
//!
//! ```bash
//! cat_partitions [OPTIONS] <OFFSET:FILE>...
//! ```
//!
//! # Examples
//!
//! ```bash
//! # Concatenate two DFXML files with their partition offsets
//! cat_partitions 32256:part1.dfxml 1073741824:part2.dfxml > combined.dfxml
//!
//! # Include source image path in output
//! cat_partitions --image-path disk.raw 32256:part1.dfxml > combined.dfxml
//!
//! # Enable debug output
//! cat_partitions --debug 32256:part1.dfxml > combined.dfxml
//! ```
//!
//! # Input Format
//!
//! Each input argument must be in the format `OFFSET:PATH` where:
//! - `OFFSET` is the partition's byte offset from the start of the disk image
//! - `PATH` is the path to the DFXML file for that partition
//!
//! # Output
//!
//! The output is a valid DFXML document with:
//! - All volumes from the input files, each with `partition_offset` set
//! - All file objects with updated `partition` numbers and byte run offsets
//! - Accumulated namespaces from all input documents

use std::fs::File;
use std::io::BufReader;

use clap::Parser;

use dfxml_rs::objects::{DFXMLObject, FileObject, LibraryObject, VolumeObject};
use dfxml_rs::reader::parse;
use dfxml_rs::writer::{to_string, DFXMLWriter, WriterConfig};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Concatenate DFXML documents with partition offset handling.
///
/// Reads multiple DFXML files, each prefixed with a partition offset, and
/// outputs a single combined DFXML document. Partition numbers and byte run
/// offsets are updated based on the provided offsets.
#[derive(Parser, Debug)]
#[command(name = "cat_partitions")]
#[command(version = VERSION)]
#[command(about = "Concatenate DFXML documents with partition offset handling")]
#[command(
    long_about = "Reads multiple DFXML files, each prefixed with a partition offset \
    (e.g., '32256:file.dfxml'), and outputs a single combined DFXML document. \
    This is a Rust port of the Python cat_partitions.py tool."
)]
struct Args {
    /// List of DFXML files with partition offsets.
    ///
    /// Each argument should be in the format OFFSET:PATH, where OFFSET is
    /// the partition's byte offset and PATH is the DFXML file path.
    /// Example: 32256:partition1.dfxml
    #[arg(required = true, value_name = "OFFSET:FILE")]
    labeled_xml_files: Vec<String>,

    /// Enable debug output
    #[arg(short, long)]
    debug: bool,

    /// Path to the source image file to record in the resulting DFXML
    #[arg(long)]
    image_path: Option<String>,

    /// Output compact XML (no indentation)
    #[arg(long)]
    compact: bool,
}

/// Parsed input specification: offset and path.
#[derive(Debug)]
struct LabeledInput {
    offset: u64,
    path: String,
}

/// Parse a labeled input specification (OFFSET:PATH).
fn parse_labeled_input(spec: &str) -> Result<LabeledInput, String> {
    let parts: Vec<&str> = spec.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Malformed argument. Expected 'OFFSET:PATH', got: {}",
            spec
        ));
    }

    let offset = parts[0].parse::<u64>().map_err(|_| {
        format!(
            "Invalid offset '{}'. Expected a number in: {}",
            parts[0], spec
        )
    })?;

    let path = parts[1].to_string();
    if path.is_empty() {
        return Err(format!("Empty path in: {}", spec));
    }

    Ok(LabeledInput { offset, path })
}

/// Update byte run img_offsets based on fs_offset and partition offset.
fn update_file_byte_runs(file: &mut FileObject, partition_offset: u64) {
    // Update data byte runs
    if let Some(ref mut brs) = file.data_brs {
        for br in brs.iter_mut() {
            if let Some(fs_offset) = br.fs_offset {
                br.img_offset = Some(fs_offset + partition_offset);
            }
        }
    }

    // Update inode byte runs
    if let Some(ref mut brs) = file.inode_brs {
        for br in brs.iter_mut() {
            if let Some(fs_offset) = br.fs_offset {
                br.img_offset = Some(fs_offset + partition_offset);
            }
        }
    }

    // Update name byte runs
    if let Some(ref mut brs) = file.name_brs {
        for br in brs.iter_mut() {
            if let Some(fs_offset) = br.fs_offset {
                br.img_offset = Some(fs_offset + partition_offset);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.debug {
        eprintln!("Debug mode enabled");
        eprintln!("Processing {} input files", args.labeled_xml_files.len());
    }

    // Parse and validate all input specifications
    let mut inputs: Vec<LabeledInput> = Vec::new();
    for spec in &args.labeled_xml_files {
        match parse_labeled_input(spec) {
            Ok(input) => inputs.push(input),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Sort by offset (matching Python behavior)
    inputs.sort_by_key(|i| i.offset);

    // Create output DFXML document
    let mut output_doc = DFXMLObject::new();
    output_doc.program = Some("cat_partitions".to_string());
    output_doc.program_version = Some(VERSION.to_string());
    output_doc.command_line = Some(std::env::args().collect::<Vec<_>>().join(" "));

    // Add Dublin Core metadata
    output_doc.dc.insert(
        "type".to_string(),
        "File system walk concatenation".to_string(),
    );

    // Add creator libraries
    output_doc.add_creator_library(LibraryObject::new("Rust", env!("CARGO_PKG_RUST_VERSION")));
    output_doc.add_creator_library(LibraryObject::new("dfxml-rs", dfxml_rs::VERSION));

    // Add source image if provided
    if let Some(ref image_path) = args.image_path {
        output_doc.sources.push(image_path.clone());
    }

    // Process each input file
    for (partition_index, input) in inputs.iter().enumerate() {
        if args.debug {
            eprintln!(
                "Processing partition {}: offset={}, path={}",
                partition_index + 1,
                input.offset,
                input.path
            );
        }

        // Parse the input DFXML
        let file = File::open(&input.path)?;
        let reader = BufReader::new(file);
        let parsed_doc = parse(reader)?;

        // Check volume count (Python script assumes at most one volume per document)
        let volume_count = parsed_doc.volume_count();
        if volume_count > 1 {
            eprintln!(
                "Error: Input DFXML document has {} volumes; this script assumes each \
                input document only has one: {}",
                volume_count, input.path
            );
            std::process::exit(1);
        }

        // Accumulate namespaces from input document
        for (prefix, uri) in parsed_doc.namespaces() {
            output_doc.add_namespace(prefix, uri);
        }

        // Get or create the volume
        let mut volume = if volume_count == 0 {
            // No volume in input - create a new one and collect files
            if args.debug {
                eprintln!("  No volume found, creating new volume");
            }
            let mut v = VolumeObject::new();

            // Collect files directly attached to the document
            for file in parsed_doc.files() {
                let mut file_copy = file.clone();
                file_copy.partition = Some((partition_index + 1) as u32);
                update_file_byte_runs(&mut file_copy, input.offset);
                v.append_file(file_copy);
            }

            v
        } else {
            // Has a volume - clone it and update files
            if args.debug {
                eprintln!("  Found existing volume");
            }
            let source_volume = parsed_doc.volumes().next().unwrap();
            let mut v = source_volume.clone();

            // Update all files in the volume
            for file in v.files_mut() {
                file.partition = Some((partition_index + 1) as u32);
                update_file_byte_runs(file, input.offset);
            }

            v
        };

        // Set the partition offset
        volume.partition_offset = Some(input.offset);

        if args.debug {
            eprintln!("  Volume has {} files", volume.file_count());
        }

        // Append to output document
        output_doc.append_volume(volume);
    }

    // Write output
    let xml = if args.compact {
        DFXMLWriter::with_config(WriterConfig::compact()).write_to_string(&output_doc)?
    } else {
        to_string(&output_doc)?
    };

    println!("{}", xml);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_labeled_input, update_file_byte_runs};
    use dfxml_rs::objects::FileObject;

    #[test]
    fn test_parse_labeled_input_valid() {
        let input = parse_labeled_input("32256:test.dfxml").unwrap();
        assert_eq!(input.offset, 32256);
        assert_eq!(input.path, "test.dfxml");
    }

    #[test]
    fn test_parse_labeled_input_large_offset() {
        let input = parse_labeled_input("1073741824:partition2.dfxml").unwrap();
        assert_eq!(input.offset, 1073741824);
        assert_eq!(input.path, "partition2.dfxml");
    }

    #[test]
    fn test_parse_labeled_input_path_with_colon() {
        // Path might contain colons (e.g., Windows paths or URLs)
        let input = parse_labeled_input("512:C:\\path\\to\\file.dfxml").unwrap();
        assert_eq!(input.offset, 512);
        assert_eq!(input.path, "C:\\path\\to\\file.dfxml");
    }

    #[test]
    fn test_parse_labeled_input_invalid_no_colon() {
        let result = parse_labeled_input("test.dfxml");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_labeled_input_invalid_offset() {
        let result = parse_labeled_input("abc:test.dfxml");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_labeled_input_empty_path() {
        let result = parse_labeled_input("32256:");
        assert!(result.is_err());
    }

    #[test]
    fn test_update_file_byte_runs() {
        use dfxml_rs::objects::{ByteRun, ByteRuns};

        let mut file = FileObject::new();

        // Create byte runs with fs_offset
        let mut brs = ByteRuns::new();
        let mut br = ByteRun::new();
        br.fs_offset = Some(1000);
        br.len = Some(512);
        brs.push(br);
        file.data_brs = Some(brs);

        // Update with partition offset
        update_file_byte_runs(&mut file, 32256);

        // Check that img_offset was calculated
        let updated_br = &file.data_brs.as_ref().unwrap()[0];
        assert_eq!(updated_br.img_offset, Some(33256)); // 1000 + 32256
        assert_eq!(updated_br.fs_offset, Some(1000)); // Unchanged
    }
}
