//! FileObject - represents a file in DFXML.
//!
//! This is the most commonly used DFXML object, representing a single file
//! with its metadata, timestamps, hashes, and byte run locations.

use crate::objects::common::{ByteRuns, Hashes, Timestamp, TimestampName};
use std::collections::HashSet;

/// Allocation status of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AllocStatus {
    /// Allocation status is unknown
    #[default]
    Unknown,
    /// File is allocated
    Allocated,
    /// File is unallocated/deleted
    Unallocated,
}

/// File system name type (regular file, directory, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NameType {
    /// Regular file
    Regular,
    /// Directory
    Directory,
    /// Symbolic link
    SymbolicLink,
    /// Block device
    BlockDevice,
    /// Character device
    CharacterDevice,
    /// Named pipe (FIFO)
    Fifo,
    /// Socket
    Socket,
    /// Shadow/whiteout entry
    Shadow,
    /// Virtual file
    Virtual,
    /// Unknown type
    Unknown,
}

impl NameType {
    /// Creates a NameType from a TSK name type code.
    pub fn from_code(code: i32) -> Self {
        // TSK name type codes
        match code {
            1 => NameType::Fifo,
            2 => NameType::CharacterDevice,
            3 => NameType::Directory,
            4 => NameType::BlockDevice,
            5 => NameType::Regular,
            6 => NameType::SymbolicLink,
            7 => NameType::Socket,
            8 => NameType::Shadow,
            9 => NameType::Virtual,
            _ => NameType::Unknown,
        }
    }

    /// Returns the single-character string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            NameType::Regular => "r",
            NameType::Directory => "d",
            NameType::SymbolicLink => "l",
            NameType::BlockDevice => "b",
            NameType::CharacterDevice => "c",
            NameType::Fifo => "p",
            NameType::Socket => "s",
            NameType::Shadow => "w",
            NameType::Virtual => "v",
            NameType::Unknown => "-",
        }
    }
}

impl std::str::FromStr for NameType {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "r" | "regular" => Ok(NameType::Regular),
            "d" | "directory" => Ok(NameType::Directory),
            "l" | "symlink" | "symbolic_link" => Ok(NameType::SymbolicLink),
            "b" | "block" => Ok(NameType::BlockDevice),
            "c" | "char" | "character" => Ok(NameType::CharacterDevice),
            "p" | "fifo" => Ok(NameType::Fifo),
            "s" | "socket" => Ok(NameType::Socket),
            "w" | "shadow" | "whiteout" => Ok(NameType::Shadow),
            "v" | "virtual" => Ok(NameType::Virtual),
            "-" | "unknown" | "" => Ok(NameType::Unknown),
            _ => Ok(NameType::Unknown),
        }
    }
}

/// Meta type (inode type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MetaType {
    /// Regular file
    Regular,
    /// Directory
    Directory,
    /// Symbolic link
    SymbolicLink,
    /// Block device
    BlockDevice,
    /// Character device
    CharacterDevice,
    /// Named pipe (FIFO)
    Fifo,
    /// Socket
    Socket,
    /// Shadow/whiteout entry
    Shadow,
    /// Virtual file
    Virtual,
    /// Unknown type
    Unknown,
}

impl MetaType {
    /// Creates a MetaType from a TSK meta type code.
    pub fn from_code(code: i32) -> Self {
        match code {
            1 => MetaType::Regular,
            2 => MetaType::Directory,
            3 => MetaType::SymbolicLink,
            4 => MetaType::BlockDevice,
            5 => MetaType::CharacterDevice,
            6 => MetaType::Fifo,
            7 => MetaType::Socket,
            8 => MetaType::Shadow,
            9 => MetaType::Virtual,
            _ => MetaType::Unknown,
        }
    }
}

/// Represents a file object in DFXML.
///
/// FileObject is the core type for representing files discovered during
/// forensic analysis. It contains:
/// - File identification (filename, inode, partition)
/// - Timestamps (mtime, atime, ctime, crtime)
/// - Size and hash information
/// - Byte run locations
/// - Ownership and permissions
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FileObject {
    // === Identification ===
    /// Unique identifier within the DFXML document
    pub id: Option<u64>,
    /// File path/name
    pub filename: Option<String>,
    /// Inode number
    pub inode: Option<u64>,
    /// Partition number
    pub partition: Option<u32>,
    /// Sequence number (for NTFS)
    pub seq: Option<u64>,

    // === Allocation ===
    /// Overall allocation status
    pub alloc: Option<bool>,
    /// Inode allocation status
    pub alloc_inode: Option<bool>,
    /// Name allocation status
    pub alloc_name: Option<bool>,
    /// Used flag
    pub used: Option<bool>,
    /// Unused flag (opposite of used)
    pub unused: Option<bool>,
    /// Orphan flag
    pub orphan: Option<bool>,
    /// Unallocated flag
    pub unalloc: Option<bool>,
    /// Compressed flag
    pub compressed: Option<bool>,

    // === Types ===
    /// Name type (file, directory, etc.)
    pub name_type: Option<NameType>,
    /// Meta/inode type
    pub meta_type: Option<MetaType>,

    // === Size ===
    /// Logical file size in bytes
    pub filesize: Option<u64>,

    // === Timestamps ===
    /// Modification time
    pub mtime: Option<Timestamp>,
    /// Access time
    pub atime: Option<Timestamp>,
    /// Change time (inode change on Unix)
    pub ctime: Option<Timestamp>,
    /// Creation time
    pub crtime: Option<Timestamp>,
    /// Deletion time
    pub dtime: Option<Timestamp>,
    /// Backup time
    pub bkup_time: Option<Timestamp>,

    // === Ownership and Permissions ===
    /// User ID
    pub uid: Option<u32>,
    /// Group ID
    pub gid: Option<u32>,
    /// File mode/permissions
    pub mode: Option<u32>,
    /// Number of hard links
    pub nlink: Option<u32>,

    // === Link target ===
    /// Target path for symbolic links
    pub link_target: Option<String>,

    // === Hashes ===
    /// Cryptographic hashes of file content
    pub hashes: Hashes,

    // === Byte Runs ===
    /// Data content byte runs (default)
    pub data_brs: Option<ByteRuns>,
    /// Inode/metadata byte runs
    pub inode_brs: Option<ByteRuns>,
    /// Name entry byte runs
    pub name_brs: Option<ByteRuns>,

    // === Libmagic ===
    /// File type from libmagic
    pub libmagic: Option<String>,

    // === Error ===
    /// Error message if processing failed
    pub error: Option<String>,

    // === Differential Analysis ===
    /// Differential annotations (new, deleted, modified, etc.)
    pub annos: HashSet<String>,
    /// Properties that differ from original
    pub diffs: HashSet<String>,
    /// Reference to the original file object (for differencing)
    #[cfg_attr(feature = "serde", serde(skip))]
    pub original_fileobject: Option<Box<FileObject>>,

    // === Parent References ===
    /// Parent object identifier
    pub parent_object: Option<u64>,
}

impl FileObject {
    /// Creates a new empty FileObject.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a FileObject with a filename.
    pub fn with_filename(filename: impl Into<String>) -> Self {
        Self {
            filename: Some(filename.into()),
            ..Default::default()
        }
    }

    /// Returns the primary byte runs (data content).
    ///
    /// This is an alias for `data_brs` and is provided for compatibility
    /// with the Python DFXML library.
    pub fn byte_runs(&self) -> Option<&ByteRuns> {
        self.data_brs.as_ref()
    }

    /// Sets the data byte runs.
    pub fn set_byte_runs(&mut self, runs: ByteRuns) {
        self.data_brs = Some(runs);
    }

    /// Returns true if the file is allocated.
    ///
    /// Collapses potentially partial allocation information into a single answer.
    pub fn is_allocated(&self) -> Option<bool> {
        // If both inode and name allocation are explicitly true, file is allocated
        if self.alloc_inode == Some(true) && self.alloc_name == Some(true) {
            return Some(true);
        }

        // If neither is set, fall back to the general alloc flag
        if self.alloc_inode.is_none() && self.alloc_name.is_none() {
            return self.alloc;
        }

        // Partial allocation information - assume unallocated
        Some(false)
    }

    /// Sets a timestamp by name.
    pub fn set_timestamp(&mut self, name: TimestampName, ts: Timestamp) {
        match name {
            TimestampName::Mtime => self.mtime = Some(ts),
            TimestampName::Atime => self.atime = Some(ts),
            TimestampName::Ctime => self.ctime = Some(ts),
            TimestampName::Crtime => self.crtime = Some(ts),
            TimestampName::Dtime => self.dtime = Some(ts),
            TimestampName::BkupTime => self.bkup_time = Some(ts),
        }
    }

    /// Gets a timestamp by name.
    pub fn get_timestamp(&self, name: TimestampName) -> Option<&Timestamp> {
        match name {
            TimestampName::Mtime => self.mtime.as_ref(),
            TimestampName::Atime => self.atime.as_ref(),
            TimestampName::Ctime => self.ctime.as_ref(),
            TimestampName::Crtime => self.crtime.as_ref(),
            TimestampName::Dtime => self.dtime.as_ref(),
            TimestampName::BkupTime => self.bkup_time.as_ref(),
        }
    }

    /// Compares this file object to another, returning the set of differing properties.
    pub fn compare_to(&self, other: &FileObject) -> HashSet<String> {
        let mut diffs = HashSet::new();

        macro_rules! compare_field {
            ($field:ident) => {
                if self.$field != other.$field {
                    diffs.insert(stringify!($field).to_string());
                }
            };
        }

        compare_field!(filename);
        compare_field!(inode);
        compare_field!(partition);
        compare_field!(seq);
        compare_field!(alloc);
        compare_field!(alloc_inode);
        compare_field!(alloc_name);
        compare_field!(name_type);
        compare_field!(meta_type);
        compare_field!(filesize);
        compare_field!(mtime);
        compare_field!(atime);
        compare_field!(ctime);
        compare_field!(crtime);
        compare_field!(dtime);
        compare_field!(bkup_time);
        compare_field!(uid);
        compare_field!(gid);
        compare_field!(mode);
        compare_field!(nlink);
        compare_field!(link_target);

        // Compare hashes
        if self.hashes != other.hashes {
            diffs.insert("hashes".to_string());
        }

        diffs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::common::{ByteRun, HashType};
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_file_object_new() {
        let fo = FileObject::new();
        assert!(fo.filename.is_none());
        assert!(fo.is_allocated().is_none());
    }

    #[test]
    fn test_file_object_with_filename() {
        let fo = FileObject::with_filename("/home/user/test.txt");
        assert_eq!(fo.filename, Some("/home/user/test.txt".to_string()));
    }

    #[test]
    fn test_is_allocated() {
        let mut fo = FileObject::new();
        assert!(fo.is_allocated().is_none());

        fo.alloc_inode = Some(true);
        fo.alloc_name = Some(true);
        assert_eq!(fo.is_allocated(), Some(true));

        fo.alloc_inode = Some(false);
        assert_eq!(fo.is_allocated(), Some(false));
    }

    #[test]
    fn test_file_object_hashes() {
        let mut fo = FileObject::new();
        fo.hashes
            .set(HashType::Md5, "d41d8cd98f00b204e9800998ecf8427e".to_string());
        fo.hashes.set(
            HashType::Sha256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        );

        assert!(fo.hashes.has_any());
        assert_eq!(
            fo.hashes.get(HashType::Md5),
            Some("d41d8cd98f00b204e9800998ecf8427e")
        );
    }

    #[test]
    fn test_file_object_byte_runs() {
        let mut fo = FileObject::new();
        let mut runs = ByteRuns::new();
        runs.push(ByteRun::with_img_offset(1024, 512));
        runs.push(ByteRun::with_img_offset(2048, 1024));
        fo.set_byte_runs(runs);

        let br = fo.byte_runs().unwrap();
        assert_eq!(br.len(), 2);
        assert_eq!(br.total_len(), Some(1536));
    }

    #[test]
    fn test_file_object_compare() {
        let mut fo1 = FileObject::with_filename("test.txt");
        fo1.filesize = Some(1024);

        let mut fo2 = FileObject::with_filename("test.txt");
        fo2.filesize = Some(2048);

        let diffs = fo1.compare_to(&fo2);
        assert!(diffs.contains("filesize"));
        assert!(!diffs.contains("filename"));
    }
}
