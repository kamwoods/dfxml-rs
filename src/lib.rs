//! Digital Forensics XML (DFXML) library for Rust.
//!
//! This crate provides types and utilities for working with Digital Forensics XML,
//! a standardized format for representing digital forensic metadata.
//!
//! # Features
//!
//! - **Core Types**: Complete representation of DFXML elements including files,
//!   volumes, disk images, partitions, and metadata.
//! - **Streaming Reader**: Memory-efficient parsing of large DFXML files.
//! - **Writer**: Generate valid DFXML output (coming soon).
//! - **Serde Support**: Optional serialization with the `serde` feature.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use dfxml::reader::{DFXMLReader, Event, parse};
//! use std::fs::File;
//! use std::io::BufReader;
//!
//! // Parse a complete DFXML file
//! let file = File::open("forensic_output.xml").unwrap();
//! let dfxml = parse(BufReader::new(file)).unwrap();
//!
//! // Iterate over all files
//! for file in dfxml.iter_files() {
//!     println!("File: {:?}, Size: {:?}", file.filename, file.filesize);
//! }
//! ```
//!
//! # Streaming API
//!
//! For large DFXML files, use the streaming reader:
//!
//! ```rust,no_run
//! use dfxml::reader::{DFXMLReader, Event};
//! use std::fs::File;
//! use std::io::BufReader;
//!
//! let file = File::open("large_forensic_output.xml").unwrap();
//! let reader = DFXMLReader::from_reader(BufReader::new(file));
//!
//! for result in reader {
//!     match result {
//!         Ok(Event::FileObject(file)) => {
//!             println!("File: {:?}", file.filename);
//!         }
//!         Ok(_) => {}
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! ```
//!
//! # Module Structure
//!
//! - [`objects`] - Core DFXML data structures
//! - [`reader`] - Streaming XML parser
//! - [`error`] - Error types
//!
//! # Optional Features
//!
//! - `serde` - Enable serde serialization/deserialization support

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod error;
pub mod objects;
pub mod reader;

// Re-export commonly used types at the crate root
pub use error::{Error, Result};
pub use objects::{
    ByteRun, ByteRuns, DFXMLObject, FileObject, HashType, Hashes, Timestamp, VolumeObject,
};
pub use reader::{parse, parse_file_objects, DFXMLReader, Event};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
