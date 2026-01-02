//! cat_fileobjects - Extract fileobjects from a DFXML file.
//!
//! This tool reads a DFXML file and outputs a new DFXML file containing
//! only the fileobjects from the input. It is a Rust port of the Python
//! cat_fileobjects.py tool from the dfxml_python project.
//!
//! # Usage
//!
//! ```bash
//! cat_fileobjects [OPTIONS] <FILENAME>
//! ```
//!
//! # Examples
//!
//! ```bash
//! # Extract all fileobjects from a DFXML file
//! cat_fileobjects input.dfxml > output.dfxml
//!
//! # Cache all fileobjects before printing (for debugging)
//! cat_fileobjects --cache input.dfxml > output.dfxml
//!
//! # Enable debug output
//! cat_fileobjects --debug input.dfxml > output.dfxml
//! ```
//!
//! # Output
//!
//! The output is a valid DFXML document with:
//! - Standard DFXML and delta namespaces
//! - Creator metadata (program name, version, command line)
//! - Source image filename (the input file)
//! - All fileobject elements from the input file

use std::fs::File;
use std::io::{self, BufReader};

use clap::Parser;

use dfxml_rs::objects::{DFXMLObject, FileObject, DFXML_VERSION, XMLNS_DELTA, XMLNS_DFXML};
use dfxml_rs::reader::{DFXMLReader, Event};
use dfxml_rs::writer::{DFXMLWriter, WriterConfig};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Extract fileobjects from a DFXML file.
///
/// Reads a DFXML file and outputs a new DFXML file containing only the
/// fileobjects from the input. This is useful for extracting file metadata
/// from forensic analysis results.
#[derive(Parser, Debug)]
#[command(name = "cat_fileobjects")]
#[command(version = VERSION)]
#[command(about = "Extract fileobjects from a DFXML file")]
#[command(
    long_about = "Reads a DFXML file and outputs a new DFXML document containing \
    only the fileobjects. This is a Rust port of the Python cat_fileobjects.py tool."
)]
struct Args {
    /// Input DFXML file to process
    filename: String,

    /// Cache all fileobjects before printing
    ///
    /// When enabled, all fileobjects are read into memory before being
    /// written to output. This can be useful for debugging or when the
    /// input file might be modified during processing.
    #[arg(long)]
    cache: bool,

    /// Enable debug output
    ///
    /// Prints additional information about processing to stderr.
    #[arg(long)]
    debug: bool,

    /// Output compact XML (no indentation)
    #[arg(long)]
    compact: bool,
}

/// Writes a single FileObject to stdout as XML.
fn write_fileobject(file: &FileObject, config: &WriterConfig) -> io::Result<()> {
    // Create a temporary DFXML document to use the writer infrastructure
    // We'll extract just the fileobject portion
    let mut temp_doc = DFXMLObject::new();
    temp_doc.append_file(file.clone());

    // Use the writer to generate XML, then extract the fileobject portion
    let writer = DFXMLWriter::with_config(config.clone());
    let xml = writer
        .write_to_string(&temp_doc)
        .map_err(|e| io::Error::other(e.to_string()))?;

    // Find and extract the fileobject element
    // Look for <fileobject> ... </fileobject>
    if let Some(start) = xml.find("<fileobject") {
        if let Some(end) = xml.rfind("</fileobject>") {
            let fileobject_xml = &xml[start..end + "</fileobject>".len()];
            println!("{}", fileobject_xml);
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.debug {
        eprintln!("Debug mode enabled");
        eprintln!("Processing: {}", args.filename);
        eprintln!("Cache mode: {}", args.cache);
    }

    // Build the output DFXML header
    let command_line = std::env::args().collect::<Vec<_>>().join(" ");

    // Print XML declaration and opening elements
    println!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<dfxml
  xmlns="{}"
  xmlns:delta="{}"
  version="{}">
  <metadata/>
  <creator>
    <program>cat_fileobjects</program>
    <version>{}</version>
    <execution_environment>
      <command_line>{}</command_line>
    </execution_environment>
  </creator>
  <source>
    <image_filename>{}</image_filename>
  </source>"#,
        XMLNS_DFXML, XMLNS_DELTA, DFXML_VERSION, VERSION, command_line, args.filename
    );

    // Open and parse the input file
    let file = File::open(&args.filename)?;
    let reader = BufReader::new(file);
    let dfxml_reader = DFXMLReader::from_reader(reader);

    // Configure writer for fileobject output
    let config = if args.compact {
        WriterConfig::compact()
    } else {
        WriterConfig::default()
    };

    if args.cache {
        // Cache mode: collect all fileobjects first, then print
        let mut file_objects: Vec<FileObject> = Vec::new();

        for result in dfxml_reader {
            match result {
                Ok(Event::FileObject(file)) => {
                    if args.debug {
                        eprintln!("Processing: {:?}", file.filename);
                    }
                    file_objects.push(*file);
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

        // Print all cached fileobjects
        for file in &file_objects {
            if args.debug {
                eprintln!("Printing with cache: {:?}", file.filename);
            }
            write_fileobject(file, &config)?;
        }
    } else {
        // Streaming mode: print each fileobject as it's parsed
        for result in dfxml_reader {
            match result {
                Ok(Event::FileObject(file)) => {
                    if args.debug {
                        eprintln!("Processing: {:?}", file.filename);
                        eprintln!("Printing without cache: {:?}", file.filename);
                    }
                    write_fileobject(&file, &config)?;
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
    }

    // Close the DFXML document
    println!("</dfxml>");

    Ok(())
}

#[cfg(test)]
mod tests {
    use dfxml_rs::objects::FileObject;
    use dfxml_rs::reader::{DFXMLReader, Event};
    use std::io::Cursor;

    const TEST_DFXML: &str = r#"<?xml version="1.0"?>
<dfxml version="1.0">
  <creator>
    <program>test</program>
  </creator>
  <fileobject>
    <filename>file1.txt</filename>
    <filesize>1024</filesize>
  </fileobject>
  <volume>
    <ftype_str>ntfs</ftype_str>
    <fileobject>
      <filename>file2.txt</filename>
      <filesize>2048</filesize>
    </fileobject>
  </volume>
  <fileobject>
    <filename>file3.txt</filename>
    <filesize>4096</filesize>
  </fileobject>
</dfxml>"#;

    #[test]
    fn test_parse_fileobjects() {
        let cursor = Cursor::new(TEST_DFXML);
        let reader = DFXMLReader::from_reader(cursor);

        let files: Vec<FileObject> = reader
            .filter_map(|r| match r {
                Ok(Event::FileObject(f)) => Some(*f),
                _ => None,
            })
            .collect();

        assert_eq!(files.len(), 3);
        assert_eq!(files[0].filename, Some("file1.txt".to_string()));
        assert_eq!(files[1].filename, Some("file2.txt".to_string()));
        assert_eq!(files[2].filename, Some("file3.txt".to_string()));
    }
}
