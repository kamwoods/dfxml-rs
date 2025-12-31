//! Common types shared across DFXML objects.
//!
//! This module contains foundational types used throughout DFXML:
//! - [`Hashes`] - Cryptographic hash values
//! - [`Timestamp`] - Forensic timestamps with precision
//! - [`ByteRun`] - A contiguous run of bytes on disk/in file
//! - [`ByteRuns`] - A collection of byte runs with an optional facet

use crate::error::{Error, Result};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use std::fmt;
use std::str::FromStr;

// ============================================================================
// DFXML Namespaces and Constants
// ============================================================================

/// DFXML schema version
pub const DFXML_VERSION: &str = "2.0.0-beta.0";

/// Dublin Core namespace
pub const XMLNS_DC: &str = "http://purl.org/dc/elements/1.1/";

/// DFXML namespace
pub const XMLNS_DFXML: &str = "http://www.forensicswiki.org/wiki/Category:Digital_Forensics_XML";

/// Delta (differencing) namespace
pub const XMLNS_DELTA: &str = "http://www.forensicswiki.org/wiki/Forensic_Disk_Differencing";

/// DFXML extensions namespace
pub const XMLNS_DFXML_EXT: &str =
    "http://www.forensicswiki.org/wiki/Category:Digital_Forensics_XML#extensions";

// ============================================================================
// Hash Types
// ============================================================================

/// Supported hash algorithms in DFXML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HashType {
    /// MD5 (128-bit)
    Md5,
    /// SHA-1 (160-bit)
    Sha1,
    /// SHA-224 (224-bit)
    Sha224,
    /// SHA-256 (256-bit)
    Sha256,
    /// SHA-384 (384-bit)
    Sha384,
    /// SHA-512 (512-bit)
    Sha512,
    /// MD6 (variable, typically 512-bit)
    Md6,
}

impl HashType {
    /// Returns the expected length of the hash in hexadecimal characters.
    pub fn expected_hex_len(&self) -> usize {
        match self {
            HashType::Md5 => 32,
            HashType::Sha1 => 40,
            HashType::Sha224 => 56,
            HashType::Sha256 => 64,
            HashType::Sha384 => 96,
            HashType::Sha512 => 128,
            HashType::Md6 => 128, // MD6 can vary, using 512-bit default
        }
    }

    /// Returns the XML element/attribute name for this hash type.
    pub fn as_str(&self) -> &'static str {
        match self {
            HashType::Md5 => "md5",
            HashType::Sha1 => "sha1",
            HashType::Sha224 => "sha224",
            HashType::Sha256 => "sha256",
            HashType::Sha384 => "sha384",
            HashType::Sha512 => "sha512",
            HashType::Md6 => "md6",
        }
    }
}

impl FromStr for HashType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "md5" => Ok(HashType::Md5),
            "sha1" => Ok(HashType::Sha1),
            "sha224" => Ok(HashType::Sha224),
            "sha256" => Ok(HashType::Sha256),
            "sha384" => Ok(HashType::Sha384),
            "sha512" => Ok(HashType::Sha512),
            "md6" => Ok(HashType::Md6),
            _ => Err(Error::InvalidHash {
                hash_type: s.to_string(),
                message: "Unknown hash type".to_string(),
            }),
        }
    }
}

impl fmt::Display for HashType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A collection of cryptographic hash values.
///
/// Stores hash digests for various algorithms. All hashes are stored as
/// lowercase hexadecimal strings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Hashes {
    /// MD5 hash (32 hex characters)
    pub md5: Option<String>,
    /// SHA-1 hash (40 hex characters)
    pub sha1: Option<String>,
    /// SHA-224 hash (56 hex characters)
    pub sha224: Option<String>,
    /// SHA-256 hash (64 hex characters)
    pub sha256: Option<String>,
    /// SHA-384 hash (96 hex characters)
    pub sha384: Option<String>,
    /// SHA-512 hash (128 hex characters)
    pub sha512: Option<String>,
    /// MD6 hash (variable length)
    pub md6: Option<String>,
}

impl Hashes {
    /// Creates a new empty Hashes collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if any hash is set.
    pub fn has_any(&self) -> bool {
        self.md5.is_some()
            || self.sha1.is_some()
            || self.sha224.is_some()
            || self.sha256.is_some()
            || self.sha384.is_some()
            || self.sha512.is_some()
            || self.md6.is_some()
    }

    /// Sets a hash value by type.
    pub fn set(&mut self, hash_type: HashType, value: String) {
        let normalized = value.to_lowercase();
        match hash_type {
            HashType::Md5 => self.md5 = Some(normalized),
            HashType::Sha1 => self.sha1 = Some(normalized),
            HashType::Sha224 => self.sha224 = Some(normalized),
            HashType::Sha256 => self.sha256 = Some(normalized),
            HashType::Sha384 => self.sha384 = Some(normalized),
            HashType::Sha512 => self.sha512 = Some(normalized),
            HashType::Md6 => self.md6 = Some(normalized),
        }
    }

    /// Gets a hash value by type.
    pub fn get(&self, hash_type: HashType) -> Option<&str> {
        match hash_type {
            HashType::Md5 => self.md5.as_deref(),
            HashType::Sha1 => self.sha1.as_deref(),
            HashType::Sha224 => self.sha224.as_deref(),
            HashType::Sha256 => self.sha256.as_deref(),
            HashType::Sha384 => self.sha384.as_deref(),
            HashType::Sha512 => self.sha512.as_deref(),
            HashType::Md6 => self.md6.as_deref(),
        }
    }

    /// Iterates over all set hashes.
    pub fn iter(&self) -> impl Iterator<Item = (HashType, &str)> {
        [
            (HashType::Md5, self.md5.as_deref()),
            (HashType::Sha1, self.sha1.as_deref()),
            (HashType::Sha224, self.sha224.as_deref()),
            (HashType::Sha256, self.sha256.as_deref()),
            (HashType::Sha384, self.sha384.as_deref()),
            (HashType::Sha512, self.sha512.as_deref()),
            (HashType::Md6, self.md6.as_deref()),
        ]
        .into_iter()
        .filter_map(|(t, v)| v.map(|val| (t, val)))
    }
}

// ============================================================================
// Timestamp Types
// ============================================================================

/// Time unit for precision specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TimeUnit {
    /// Days
    Day,
    /// Seconds
    Second,
    /// Milliseconds
    Millisecond,
    /// Nanoseconds
    Nanosecond,
}

impl TimeUnit {
    /// Returns the string representation of this time unit.
    pub fn as_str(&self) -> &'static str {
        match self {
            TimeUnit::Day => "d",
            TimeUnit::Second => "s",
            TimeUnit::Millisecond => "ms",
            TimeUnit::Nanosecond => "ns",
        }
    }
}

impl FromStr for TimeUnit {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "d" => Ok(TimeUnit::Day),
            "s" | "" => Ok(TimeUnit::Second),
            "ms" => Ok(TimeUnit::Millisecond),
            "ns" => Ok(TimeUnit::Nanosecond),
            _ => Err(Error::InvalidPrecision(format!("Unknown time unit: {}", s))),
        }
    }
}

impl fmt::Display for TimeUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Timestamp precision as (resolution, unit).
///
/// For example, `Precision { resolution: 100, unit: TimeUnit::Nanosecond }`
/// represents 100ns precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Precision {
    /// The numeric resolution value
    pub resolution: i32,
    /// The time unit (seconds, milliseconds, etc.)
    pub unit: TimeUnit,
}

impl Precision {
    /// Creates a new Precision with the given resolution and unit.
    pub fn new(resolution: i32, unit: TimeUnit) -> Self {
        Self { resolution, unit }
    }
}

impl FromStr for Precision {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        // Parse strings like "100ns", "1s", "1d"
        let s = s.trim();
        if s.is_empty() {
            return Err(Error::InvalidPrecision(
                "Empty precision string".to_string(),
            ));
        }

        // Find where digits end
        let digit_end = s
            .char_indices()
            .find(|(_, c)| !c.is_ascii_digit() && *c != '-')
            .map(|(i, _)| i)
            .unwrap_or(s.len());

        if digit_end == 0 {
            return Err(Error::InvalidPrecision(format!(
                "No numeric value in precision: {}",
                s
            )));
        }

        let resolution: i32 = s[..digit_end].parse()?;
        let unit_str = &s[digit_end..];
        let unit = if unit_str.is_empty() {
            TimeUnit::Second
        } else {
            unit_str.parse()?
        };

        Ok(Precision { resolution, unit })
    }
}

impl fmt::Display for Precision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.resolution, self.unit)
    }
}

/// The type/name of a timestamp (mtime, atime, ctime, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TimestampName {
    /// Modification time
    Mtime,
    /// Access time
    Atime,
    /// Change time (inode change on Unix)
    Ctime,
    /// Creation time
    Crtime,
    /// Deletion time
    Dtime,
    /// Backup time
    BkupTime,
}

impl TimestampName {
    /// Returns the XML element name for this timestamp type.
    pub fn as_str(&self) -> &'static str {
        match self {
            TimestampName::Mtime => "mtime",
            TimestampName::Atime => "atime",
            TimestampName::Ctime => "ctime",
            TimestampName::Crtime => "crtime",
            TimestampName::Dtime => "dtime",
            TimestampName::BkupTime => "bkup_time",
        }
    }
}

impl FromStr for TimestampName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "mtime" => Ok(TimestampName::Mtime),
            "atime" => Ok(TimestampName::Atime),
            "ctime" => Ok(TimestampName::Ctime),
            "crtime" => Ok(TimestampName::Crtime),
            "dtime" => Ok(TimestampName::Dtime),
            "bkup_time" => Ok(TimestampName::BkupTime),
            _ => Err(Error::InvalidTimestamp(format!(
                "Unknown timestamp name: {}",
                s
            ))),
        }
    }
}

impl fmt::Display for TimestampName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A forensic timestamp with optional precision and name.
///
/// Wraps a `DateTime<FixedOffset>` with additional metadata for forensic use.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Timestamp {
    /// The timestamp type (mtime, atime, etc.)
    pub name: Option<TimestampName>,
    /// The actual time value
    pub time: Option<DateTime<FixedOffset>>,
    /// Precision information
    pub prec: Option<Precision>,
}

impl Timestamp {
    /// Creates a new empty timestamp.
    pub fn new() -> Self {
        Self {
            name: None,
            time: None,
            prec: None,
        }
    }

    /// Creates a timestamp with a specific name and time.
    pub fn with_name_and_time(name: TimestampName, time: DateTime<FixedOffset>) -> Self {
        Self {
            name: Some(name),
            time: Some(time),
            prec: None,
        }
    }

    /// Parses an ISO 8601 timestamp string.
    pub fn parse_iso8601(s: &str) -> Result<DateTime<FixedOffset>> {
        // Try parsing with timezone
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Ok(dt);
        }

        // Try common ISO 8601 formats
        let formats = [
            "%Y-%m-%dT%H:%M:%S%.fZ",
            "%Y-%m-%dT%H:%M:%SZ",
            "%Y-%m-%dT%H:%M:%S%.f%:z",
            "%Y-%m-%dT%H:%M:%S%:z",
            "%Y-%m-%dT%H:%M:%S",
            "%Y-%m-%d %H:%M:%S",
        ];

        for fmt in formats {
            if let Ok(dt) = DateTime::parse_from_str(s, fmt) {
                return Ok(dt);
            }
            // Try parsing as naive and assume UTC
            if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
                return Ok(Utc.from_utc_datetime(&naive).fixed_offset());
            }
        }

        // Handle timestamps without timezone - assume UTC
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
            return Ok(Utc.from_utc_datetime(&naive).fixed_offset());
        }
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
            return Ok(Utc.from_utc_datetime(&naive).fixed_offset());
        }

        Err(Error::InvalidTimestamp(format!(
            "Cannot parse timestamp: {}",
            s
        )))
    }

    /// Returns the Unix timestamp (seconds since epoch).
    pub fn timestamp(&self) -> Option<i64> {
        self.time.map(|t| t.timestamp())
    }

    /// Returns the Unix timestamp with fractional seconds.
    pub fn timestamp_subsec(&self) -> Option<f64> {
        self.time
            .map(|t| t.timestamp() as f64 + (t.timestamp_subsec_nanos() as f64 / 1_000_000_000.0))
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.time {
            Some(t) => write!(f, "{}", t.to_rfc3339()),
            None => write!(f, ""),
        }
    }
}

// ============================================================================
// ByteRun Types
// ============================================================================

/// The facet (aspect) of a file that byte runs describe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ByteRunFacet {
    /// Data content byte runs (default)
    #[default]
    Data,
    /// Inode/metadata byte runs
    Inode,
    /// Filename byte runs
    Name,
}

impl ByteRunFacet {
    /// Returns the XML attribute value for this facet.
    pub fn as_str(&self) -> &'static str {
        match self {
            ByteRunFacet::Data => "data",
            ByteRunFacet::Inode => "inode",
            ByteRunFacet::Name => "name",
        }
    }
}

impl FromStr for ByteRunFacet {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "data" | "" => Ok(ByteRunFacet::Data),
            "inode" => Ok(ByteRunFacet::Inode),
            "name" => Ok(ByteRunFacet::Name),
            _ => Err(Error::InvalidFacet(s.to_string())),
        }
    }
}

impl fmt::Display for ByteRunFacet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The type of a byte run (e.g., "resident" for NTFS resident data).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ByteRunType {
    /// Resident data (stored in MFT for NTFS)
    Resident,
    /// Other/custom type
    Other(String),
}

impl FromStr for ByteRunType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "resident" => Ok(ByteRunType::Resident),
            other => Ok(ByteRunType::Other(other.to_string())),
        }
    }
}

impl fmt::Display for ByteRunType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ByteRunType::Resident => write!(f, "resident"),
            ByteRunType::Other(s) => write!(f, "{}", s),
        }
    }
}

/// A contiguous run of bytes representing data location.
///
/// Byte runs can specify locations in multiple coordinate systems:
/// - `img_offset`: Offset from the start of the disk image
/// - `fs_offset`: Offset from the start of the file system
/// - `file_offset`: Offset from the start of the logical file
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ByteRun {
    /// Offset from image start (bytes)
    pub img_offset: Option<u64>,
    /// Offset from file system start (bytes)
    pub fs_offset: Option<u64>,
    /// Offset from file start (bytes)
    pub file_offset: Option<u64>,
    /// Length of the run (bytes)
    pub len: Option<u64>,
    /// Fill byte for sparse/unallocated regions
    pub fill: Option<u8>,
    /// Run type (e.g., "resident")
    pub run_type: Option<ByteRunType>,
    /// Uncompressed length (if compressed)
    pub uncompressed_len: Option<u64>,
    /// Hash values for this specific run
    pub hashes: Hashes,
}

impl ByteRun {
    /// Creates a new empty byte run.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a byte run with image offset and length.
    pub fn with_img_offset(img_offset: u64, len: u64) -> Self {
        Self {
            img_offset: Some(img_offset),
            len: Some(len),
            ..Default::default()
        }
    }

    /// Returns true if this run has any hash values.
    pub fn has_hashes(&self) -> bool {
        self.hashes.has_any()
    }

    /// Attempts to concatenate two contiguous byte runs.
    ///
    /// Returns `Some(combined)` if the runs are contiguous and compatible,
    /// `None` otherwise.
    pub fn try_concat(&self, other: &ByteRun) -> Option<ByteRun> {
        // Don't concatenate runs with different fills
        if self.fill != other.fill {
            return None;
        }

        // Don't concatenate typed runs
        if self.run_type != other.run_type {
            return None;
        }

        // Don't concatenate compressed runs
        if self.uncompressed_len.is_some() || other.uncompressed_len.is_some() {
            return None;
        }

        // Don't concatenate runs with hashes
        if self.has_hashes() || other.has_hashes() {
            return None;
        }

        // Need lengths to concatenate
        let self_len = self.len?;
        let other_len = other.len?;

        // Check contiguity for each offset type
        let mut is_contiguous = false;

        let new_img_offset = match (self.img_offset, other.img_offset) {
            (Some(s), Some(o)) if s + self_len == o => {
                is_contiguous = true;
                Some(s)
            }
            (Some(_), Some(_)) => return None, // Not contiguous
            (None, None) => None,
            _ => return None, // Inconsistent
        };

        let new_fs_offset = match (self.fs_offset, other.fs_offset) {
            (Some(s), Some(o)) if s + self_len == o => {
                is_contiguous = true;
                Some(s)
            }
            (Some(_), Some(_)) => return None,
            (None, None) => None,
            _ => return None,
        };

        let new_file_offset = match (self.file_offset, other.file_offset) {
            (Some(s), Some(o)) if s + self_len == o => {
                is_contiguous = true;
                Some(s)
            }
            (Some(_), Some(_)) => return None,
            (None, None) => None,
            _ => return None,
        };

        if !is_contiguous {
            return None;
        }

        Some(ByteRun {
            img_offset: new_img_offset,
            fs_offset: new_fs_offset,
            file_offset: new_file_offset,
            len: Some(self_len + other_len),
            fill: self.fill,
            run_type: self.run_type.clone(),
            uncompressed_len: None,
            hashes: Hashes::default(),
        })
    }
}

/// A collection of byte runs with an optional facet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ByteRuns {
    /// The facet these runs describe
    pub facet: Option<ByteRunFacet>,
    /// The byte runs in this collection
    runs: Vec<ByteRun>,
}

impl ByteRuns {
    /// Creates a new empty ByteRuns collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a ByteRuns collection with a specific facet.
    pub fn with_facet(facet: ByteRunFacet) -> Self {
        Self {
            facet: Some(facet),
            runs: Vec::new(),
        }
    }

    /// Returns the number of byte runs.
    pub fn len(&self) -> usize {
        self.runs.len()
    }

    /// Returns true if there are no byte runs.
    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }

    /// Appends a byte run to the collection.
    pub fn push(&mut self, run: ByteRun) {
        self.runs.push(run);
    }

    /// Appends a byte run, attempting to merge with the last run if contiguous.
    ///
    /// This is useful for compacting fragmented run lists.
    pub fn glom(&mut self, run: ByteRun) {
        if let Some(last) = self.runs.last() {
            if let Some(merged) = last.try_concat(&run) {
                self.runs.pop();
                self.runs.push(merged);
                return;
            }
        }
        self.runs.push(run);
    }

    /// Returns an iterator over the byte runs.
    pub fn iter(&self) -> impl Iterator<Item = &ByteRun> {
        self.runs.iter()
    }

    /// Returns a mutable iterator over the byte runs.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ByteRun> {
        self.runs.iter_mut()
    }

    /// Returns the total length of all byte runs.
    pub fn total_len(&self) -> Option<u64> {
        let mut total = 0u64;
        for run in &self.runs {
            total += run.len?;
        }
        Some(total)
    }

    /// Gets a byte run by index.
    pub fn get(&self, index: usize) -> Option<&ByteRun> {
        self.runs.get(index)
    }
}

impl IntoIterator for ByteRuns {
    type Item = ByteRun;
    type IntoIter = std::vec::IntoIter<ByteRun>;

    fn into_iter(self) -> Self::IntoIter {
        self.runs.into_iter()
    }
}

impl<'a> IntoIterator for &'a ByteRuns {
    type Item = &'a ByteRun;
    type IntoIter = std::slice::Iter<'a, ByteRun>;

    fn into_iter(self) -> Self::IntoIter {
        self.runs.iter()
    }
}

impl FromIterator<ByteRun> for ByteRuns {
    fn from_iter<I: IntoIterator<Item = ByteRun>>(iter: I) -> Self {
        Self {
            facet: None,
            runs: iter.into_iter().collect(),
        }
    }
}

impl std::ops::Index<usize> for ByteRuns {
    type Output = ByteRun;

    fn index(&self, index: usize) -> &Self::Output {
        &self.runs[index]
    }
}

// ============================================================================
// External Elements (for preserving non-DFXML namespace elements)
// ============================================================================

/// Represents an XML element from a non-DFXML namespace.
///
/// This is used to preserve unknown/extension elements when reading DFXML,
/// allowing them to be round-tripped back to XML output.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExternalElement {
    /// The XML namespace URI (e.g., `"http://example.org/custom"`)
    pub namespace: Option<String>,
    /// The local tag name (without namespace prefix)
    pub tag_name: String,
    /// Attributes as (name, value) pairs
    pub attributes: Vec<(String, String)>,
    /// Text content of the element
    pub text: Option<String>,
    /// Child elements
    pub children: Vec<ExternalElement>,
}

impl ExternalElement {
    /// Creates a new ExternalElement with the given tag name.
    pub fn new(tag_name: impl Into<String>) -> Self {
        Self {
            namespace: None,
            tag_name: tag_name.into(),
            attributes: Vec::new(),
            text: None,
            children: Vec::new(),
        }
    }

    /// Creates a new ExternalElement with namespace and tag name.
    pub fn with_namespace(namespace: impl Into<String>, tag_name: impl Into<String>) -> Self {
        Self {
            namespace: Some(namespace.into()),
            tag_name: tag_name.into(),
            attributes: Vec::new(),
            text: None,
            children: Vec::new(),
        }
    }

    /// Sets the text content.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = Some(text.into());
    }

    /// Adds an attribute.
    pub fn add_attribute(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.attributes.push((name.into(), value.into()));
    }

    /// Adds a child element.
    pub fn add_child(&mut self, child: ExternalElement) {
        self.children.push(child);
    }

    /// Returns the qualified tag name (with namespace prefix if known).
    pub fn qualified_name(&self) -> String {
        if let Some(ref ns) = self.namespace {
            format!("{{{}}}{}", ns, self.tag_name)
        } else {
            self.tag_name.clone()
        }
    }
}

/// A list of external (non-DFXML namespace) XML elements.
///
/// This type is used to store unknown elements encountered during parsing,
/// allowing them to be preserved and written back to XML.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Externals {
    elements: Vec<ExternalElement>,
}

impl Externals {
    /// Creates a new empty Externals list.
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    /// Returns true if there are no external elements.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns the number of external elements.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Adds an external element.
    ///
    /// # Panics
    ///
    /// Panics if the element's namespace is the DFXML namespace.
    pub fn push(&mut self, element: ExternalElement) {
        if let Some(ref ns) = element.namespace {
            if ns == XMLNS_DFXML {
                panic!("External elements must not be in the DFXML namespace");
            }
        }
        self.elements.push(element);
    }

    /// Adds an external element, returning an error if it's in the DFXML namespace.
    pub fn try_push(&mut self, element: ExternalElement) -> Result<()> {
        if let Some(ref ns) = element.namespace {
            if ns == XMLNS_DFXML {
                return Err(Error::InvalidFacet(
                    "External elements must not be in the DFXML namespace".to_string(),
                ));
            }
        }
        self.elements.push(element);
        Ok(())
    }

    /// Returns an iterator over the external elements.
    pub fn iter(&self) -> impl Iterator<Item = &ExternalElement> {
        self.elements.iter()
    }

    /// Clears all external elements.
    pub fn clear(&mut self) {
        self.elements.clear();
    }
}

impl IntoIterator for Externals {
    type Item = ExternalElement;
    type IntoIter = std::vec::IntoIter<ExternalElement>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.into_iter()
    }
}

impl<'a> IntoIterator for &'a Externals {
    type Item = &'a ExternalElement;
    type IntoIter = std::slice::Iter<'a, ExternalElement>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter()
    }
}

impl std::ops::Index<usize> for Externals {
    type Output = ExternalElement;

    fn index(&self, index: usize) -> &Self::Output {
        &self.elements[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_type_from_str() {
        assert_eq!("md5".parse::<HashType>().unwrap(), HashType::Md5);
        assert_eq!("SHA256".parse::<HashType>().unwrap(), HashType::Sha256);
    }

    #[test]
    fn test_hashes() {
        let mut hashes = Hashes::new();
        assert!(!hashes.has_any());

        hashes.set(
            HashType::Md5,
            "d41d8cd98f00b204e9800998ecf8427e".to_string(),
        );
        assert!(hashes.has_any());
        assert_eq!(
            hashes.get(HashType::Md5),
            Some("d41d8cd98f00b204e9800998ecf8427e")
        );
    }

    #[test]
    fn test_precision_parse() {
        let p: Precision = "100ns".parse().unwrap();
        assert_eq!(p.resolution, 100);
        assert_eq!(p.unit, TimeUnit::Nanosecond);

        let p: Precision = "1s".parse().unwrap();
        assert_eq!(p.resolution, 1);
        assert_eq!(p.unit, TimeUnit::Second);

        let p: Precision = "1".parse().unwrap();
        assert_eq!(p.resolution, 1);
        assert_eq!(p.unit, TimeUnit::Second);
    }

    #[test]
    fn test_timestamp_parse() {
        let ts = Timestamp::parse_iso8601("2024-01-15T10:30:00Z").unwrap();
        assert_eq!(ts.timestamp(), 1705314600);

        let ts = Timestamp::parse_iso8601("2024-01-15T10:30:00.123456Z").unwrap();
        assert_eq!(ts.timestamp(), 1705314600);
        // Verify subsecond precision was parsed (123456 microseconds = 123456000 nanoseconds)
        assert_eq!(ts.timestamp_subsec_nanos(), 123456000);
    }

    #[test]
    fn test_byte_run_concat() {
        let run1 = ByteRun {
            img_offset: Some(0),
            len: Some(100),
            ..Default::default()
        };
        let run2 = ByteRun {
            img_offset: Some(100),
            len: Some(50),
            ..Default::default()
        };

        let merged = run1.try_concat(&run2).unwrap();
        assert_eq!(merged.img_offset, Some(0));
        assert_eq!(merged.len, Some(150));
    }

    #[test]
    fn test_byte_runs_glom() {
        let mut runs = ByteRuns::new();
        runs.glom(ByteRun::with_img_offset(0, 100));
        runs.glom(ByteRun::with_img_offset(100, 50));
        runs.glom(ByteRun::with_img_offset(150, 25)); // Must be contiguous: 0+100=100, 100+50=150

        // Should have merged into one run
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].len, Some(175));
    }
}
