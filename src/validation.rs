//! XSD validation for DFXML documents.
//!
//! This module provides functionality to validate DFXML documents against
//! the official DFXML XML Schema Definition (XSD).
//!
//! # Requirements
//!
//! This module requires the `validation` feature to be enabled and depends on
//! libxml2 being installed on the system.
//!
//! ## Installing libxml2
//!
//! **Ubuntu/Debian:**
//! ```bash
//! sudo apt-get install libxml2-dev
//! ```
//!
//! **macOS:**
//! ```bash
//! brew install libxml2
//! ```
//!
//! **Windows:**
//! See the libxml2 documentation for Windows installation instructions.
//!
//! # Example
//!
//! ```rust,ignore
//! use dfxml_rs::validation::{validate_file, validate_str};
//!
//! // Validate a DFXML file using the default schema location
//! validate_file("forensic_output.xml", None)?;
//!
//! // Validate with a custom schema path
//! validate_file("forensic_output.xml", Some("/path/to/dfxml.xsd"))?;
//!
//! // Validate a DFXML string
//! let xml = r#"<?xml version="1.0"?>
//! <dfxml version="1.0">
//!   <fileobject>
//!     <filename>test.txt</filename>
//!   </fileobject>
//! </dfxml>"#;
//! validate_str(xml, None)?;
//! ```

use std::path::Path;

use libxml::parser::Parser;
use libxml::schemas::{SchemaParserContext, SchemaValidationContext};

use crate::error::{Error, Result};

/// Default path to the DFXML schema file (relative to the crate root).
pub const DEFAULT_SCHEMA_PATH: &str = "external/dfxml_schema/dfxml.xsd";

/// Validates a DFXML file against the DFXML XML Schema.
///
/// # Arguments
///
/// * `xml_path` - Path to the DFXML file to validate
/// * `schema_path` - Optional path to the XSD schema file. If `None`, uses the
///   default schema location at `external/dfxml_schema/dfxml.xsd`
///
/// # Returns
///
/// Returns `Ok(())` if the document is valid, or an `Error` describing the
/// validation failure.
///
/// # Example
///
/// ```rust,ignore
/// use dfxml_rs::validation::validate_file;
///
/// // Using default schema
/// validate_file("forensic_output.xml", None)?;
///
/// // Using custom schema
/// validate_file("forensic_output.xml", Some("/path/to/dfxml.xsd"))?;
/// ```
pub fn validate_file<P: AsRef<Path>>(xml_path: P, schema_path: Option<&str>) -> Result<()> {
    let xml_path = xml_path.as_ref();
    let schema_path = schema_path.unwrap_or(DEFAULT_SCHEMA_PATH);

    // Check that files exist
    if !xml_path.exists() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("XML file not found: {}", xml_path.display()),
        )));
    }

    if !Path::new(schema_path).exists() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Schema file not found: {}. Ensure the dfxml_schema submodule is initialized.",
                schema_path
            ),
        )));
    }

    // Parse the schema
    let mut schema_parser = SchemaParserContext::from_file(schema_path);

    // Create validation context directly from the parser
    let mut validation_context =
        SchemaValidationContext::from_parser(&mut schema_parser).map_err(|errors| {
            let msg = errors
                .iter()
                .map(|e| e.message.clone().unwrap_or_default())
                .collect::<Vec<_>>()
                .join("; ");
            Error::Validation(format!("Failed to parse schema: {}", msg))
        })?;

    // Parse the XML document
    let parser = Parser::default();
    let doc = parser
        .parse_file(xml_path.to_string_lossy().as_ref())
        .map_err(|e| Error::Validation(format!("Failed to parse XML document: {:?}", e)))?;

    // Validate
    validation_context
        .validate_document(&doc)
        .map_err(|e| Error::Validation(format!("Validation failed: {:?}", e)))?;

    Ok(())
}

/// Validates a DFXML string against the DFXML XML Schema.
///
/// # Arguments
///
/// * `xml` - The DFXML content as a string
/// * `schema_path` - Optional path to the XSD schema file. If `None`, uses the
///   default schema location at `external/dfxml_schema/dfxml.xsd`
///
/// # Returns
///
/// Returns `Ok(())` if the document is valid, or an `Error` describing the
/// validation failure.
///
/// # Example
///
/// ```rust,ignore
/// use dfxml_rs::validation::validate_str;
///
/// let xml = r#"<?xml version="1.0"?>
/// <dfxml version="1.0">
///   <fileobject>
///     <filename>test.txt</filename>
///   </fileobject>
/// </dfxml>"#;
///
/// validate_str(xml, None)?;
/// ```
pub fn validate_str(xml: &str, schema_path: Option<&str>) -> Result<()> {
    let schema_path = schema_path.unwrap_or(DEFAULT_SCHEMA_PATH);

    if !Path::new(schema_path).exists() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Schema file not found: {}. Ensure the dfxml_schema submodule is initialized.",
                schema_path
            ),
        )));
    }

    // Parse the schema
    let mut schema_parser = SchemaParserContext::from_file(schema_path);

    // Create validation context directly from the parser
    let mut validation_context =
        SchemaValidationContext::from_parser(&mut schema_parser).map_err(|errors| {
            let msg = errors
                .iter()
                .map(|e| e.message.clone().unwrap_or_default())
                .collect::<Vec<_>>()
                .join("; ");
            Error::Validation(format!("Failed to parse schema: {}", msg))
        })?;

    // Parse the XML string
    let parser = Parser::default();
    let doc = parser
        .parse_string(xml)
        .map_err(|e| Error::Validation(format!("Failed to parse XML string: {:?}", e)))?;

    // Validate
    validation_context
        .validate_document(&doc)
        .map_err(|e| Error::Validation(format!("Validation failed: {:?}", e)))?;

    Ok(())
}

/// Validates a DFXML document that was generated by this library.
///
/// This is a convenience function that takes a `DFXMLObject`, serializes it
/// to XML, and validates the result against the schema.
///
/// # Arguments
///
/// * `doc` - The DFXML document object to validate
/// * `schema_path` - Optional path to the XSD schema file
///
/// # Returns
///
/// Returns `Ok(())` if the document is valid, or an `Error` describing the
/// validation failure.
///
/// # Example
///
/// ```rust,ignore
/// use dfxml_rs::objects::DFXMLObject;
/// use dfxml_rs::validation::validate_document;
///
/// let mut doc = DFXMLObject::new();
/// doc.program = Some("my-tool".to_string());
///
/// validate_document(&doc, None)?;
/// ```
pub fn validate_document(
    doc: &crate::objects::DFXMLObject,
    schema_path: Option<&str>,
) -> Result<()> {
    let xml = crate::writer::to_string(doc)?;
    validate_str(&xml, schema_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{DFXMLObject, FileObject, VolumeObject};

    // Note: These tests require the dfxml_schema submodule to be initialized
    // and libxml2 to be installed. They are ignored by default.

    #[test]
    #[ignore = "requires dfxml_schema submodule and libxml2"]
    fn test_validate_simple_document() {
        let mut doc = DFXMLObject::new();
        doc.program = Some("test".to_string());
        doc.program_version = Some("1.0".to_string());

        let mut vol = VolumeObject::new();
        let file = FileObject::with_filename("test.txt");
        vol.append_file(file);
        doc.append_volume(vol);

        let result = validate_document(&doc, None);
        assert!(result.is_ok(), "Validation failed: {:?}", result.err());
    }

    #[test]
    #[ignore = "requires dfxml_schema submodule and libxml2"]
    fn test_validate_str_valid() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<dfxml version="1.0" xmlns="http://www.forensicswiki.org/wiki/Category:Digital_Forensics_XML">
  <creator>
    <program>test</program>
    <version>1.0</version>
  </creator>
</dfxml>"#;

        let result = validate_str(xml, None);
        assert!(result.is_ok(), "Validation failed: {:?}", result.err());
    }

    #[test]
    fn test_validate_missing_schema() {
        let xml = "<dfxml version=\"1.0\"></dfxml>";
        let result = validate_str(xml, Some("/nonexistent/path/schema.xsd"));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Schema file not found"));
    }

    #[test]
    fn test_validate_missing_xml_file() {
        let result = validate_file("/nonexistent/file.xml", None);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("XML file not found"));
    }
}
