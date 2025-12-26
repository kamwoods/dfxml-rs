//! Digital Forensics XML (DFXML) library for Rust.
//!
//! This crate provides types and utilities for working with Digital Forensics XML,
//! a standardized format for representing digital forensic metadata.
//!
//! # Features
//!
//! - **Core Types**: Complete representation of DFXML elements including files,
//!   volumes, disk images, partitions, and metadata.
//! - **Streaming Reader**: Memory-efficient parsing of large DFXML files (coming soon).
//! - **Writer**: Generate valid DFXML output (coming soon).
//! - **Serde Support**: Optional serialization with the `serde` feature.
//!
//! # Quick Start
//!
//! ```rust
//! use dfxml::objects::{DFXMLObject, VolumeObject, FileObject, HashType};
//!
//! // Create a new DFXML document
//! let mut doc = DFXMLObject::new();
//! doc.program = Some("my-forensic-tool".to_string());
//! doc.program_version = Some("1.0.0".to_string());
//!
//! // Create a volume
//! let mut volume = VolumeObject::with_ftype("ntfs");
//! volume.block_size = Some(4096);
//!
//! // Create a file with metadata
//! let mut file = FileObject::with_filename("/Users/test/document.pdf");
//! file.filesize = Some(1024000);
//! file.hashes.set(HashType::Sha256,
//!     "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string());
//!
//! // Add file to volume, volume to document
//! volume.append_file(file);
//! doc.append_volume(volume);
//!
//! // Iterate over all files
//! for file in doc.iter_files() {
//!     println!("File: {:?}", file.filename);
//! }
//! ```
//!
//! # Module Structure
//!
//! - [`objects`] - Core DFXML data structures
//! - [`error`] - Error types
//!
//! # Optional Features
//!
//! - `serde` - Enable serde serialization/deserialization support

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod error;
pub mod objects;

// Re-export commonly used types at the crate root
pub use error::{Error, Result};
pub use objects::{
    ByteRun, ByteRuns, DFXMLObject, FileObject, HashType, Hashes, Timestamp, VolumeObject,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
