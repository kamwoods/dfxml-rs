//! Error types for the DFXML library.

use thiserror::Error;

/// Errors that can occur when working with DFXML data.
#[derive(Error, Debug)]
pub enum Error {
    /// XML parsing error
    #[error("XML parsing error: {0}")]
    XmlParse(#[from] quick_xml::Error),

    /// XML attribute parsing error
    #[error("XML attribute error: {0}")]
    XmlAttribute(#[from] quick_xml::events::attributes::AttrError),

    /// Invalid timestamp format
    #[error("Invalid timestamp format: {0}")]
    InvalidTimestamp(String),

    /// Invalid hash value
    #[error("Invalid hash value for {hash_type}: {message}")]
    InvalidHash {
        /// The hash algorithm type that was invalid
        hash_type: String,
        /// Description of why the hash was invalid
        message: String,
    },

    /// Invalid byte run
    #[error("Invalid byte run: {0}")]
    InvalidByteRun(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid precision format
    #[error("Invalid precision format: {0}")]
    InvalidPrecision(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// UTF-8 conversion error
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    /// Integer parsing error
    #[error("Integer parsing error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    /// Unexpected XML element
    #[error("Unexpected XML element: {0}")]
    UnexpectedElement(String),

    /// Invalid facet value
    #[error("Invalid facet value: {0}")]
    InvalidFacet(String),
}

/// Result type alias for DFXML operations.
pub type Result<T> = std::result::Result<T, Error>;
