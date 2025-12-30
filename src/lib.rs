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
//! - **Writer**: Generate valid DFXML output with proper namespace handling.
//! - **Serde Support**: Optional serialization with the `serde` feature.
//! - **XSD Validation**: Optional schema validation with the `validation` feature.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use dfxml_rs::reader::{DFXMLReader, Event, parse};
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
//! use dfxml_rs::reader::{DFXMLReader, Event};
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
//! # Writing DFXML
//!
//! ```rust,ignore
//! use dfxml_rs::objects::{DFXMLObject, VolumeObject, FileObject, HashType};
//! use dfxml_rs::writer;
//!
//! let mut doc = DFXMLObject::new();
//! doc.program = Some("my-tool".to_string());
//!
//! let mut vol = VolumeObject::with_ftype("ntfs");
//! let mut file = FileObject::with_filename("evidence.doc");
//! file.filesize = Some(1024);
//! vol.append_file(file);
//! doc.append_volume(vol);
//!
//! let xml = writer::to_string(&doc).unwrap();
//! println!("{}", xml);
//! ```
//!
//! # XSD Validation (optional)
//!
//! With the `validation` feature enabled, you can validate DFXML documents
//! against the official DFXML XML Schema:
//!
//! ```rust,ignore
//! use dfxml_rs::validation::{validate_file, validate_document};
//!
//! // Validate a file
//! validate_file("forensic_output.xml", None)?;
//!
//! // Validate a document object
//! let doc = DFXMLObject::new();
//! validate_document(&doc, None)?;
//! ```
//!
//! # Module Structure
//!
//! - [`objects`] - Core DFXML data structures
//! - [`reader`] - Streaming XML parser
//! - [`writer`] - XML serialization
//! - [`error`] - Error types
//! - [`validation`] - XSD validation (requires `validation` feature)
//!
//! # Optional Features
//!
//! - `serde` - Enable serde serialization/deserialization support
//! - `validation` - Enable XSD schema validation (requires libxml2)
//! - `cli` - Build command-line tools

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod error;
pub mod objects;
pub mod reader;
pub mod writer;

#[cfg(feature = "validation")]
pub mod validation;

// Re-export commonly used types at the crate root
pub use error::{Error, Result};
pub use objects::{
    ByteRun, ByteRuns, DFXMLObject, FileObject, HashType, Hashes, Timestamp, VolumeObject,
};
pub use reader::{parse, parse_file_objects, DFXMLReader, Event};
pub use writer::{to_string, write, DFXMLWriter, WriterConfig};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
