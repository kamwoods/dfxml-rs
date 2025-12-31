//! DFXML object types.
//!
//! This module contains all the core data structures for representing
//! Digital Forensics XML data:
//!
//! - [`DFXMLObject`] - The root document container
//! - [`FileObject`] - A file with metadata and hashes
//! - [`VolumeObject`] - A file system volume
//! - [`DiskImageObject`] - A disk image
//! - [`PartitionObject`] - A disk partition
//! - [`PartitionSystemObject`] - A partition table (MBR, GPT)
//!
//! Also provides common types:
//! - [`ByteRun`] and [`ByteRuns`] - Disk/file location information
//! - [`Timestamp`] - Forensic timestamps with precision
//! - [`Hashes`] - Cryptographic hash values

mod common;
mod dfxml;
mod fileobject;
mod volume;

// Re-export common types
pub use common::{
    ByteRun, ByteRunFacet, ByteRunType, ByteRuns, ExternalElement, Externals, HashType, Hashes,
    Precision, TimeUnit, Timestamp, TimestampName, DFXML_VERSION, XMLNS_DC, XMLNS_DELTA,
    XMLNS_DFXML, XMLNS_DFXML_EXT,
};

// Re-export main object types
pub use dfxml::{
    ChildObject, DFXMLChild, DFXMLChildIterator, DFXMLIterator, DFXMLObject, LibraryObject,
};
pub use fileobject::{AllocStatus, FileObject, MetaType, NameType};
pub use volume::{
    DiskImageChild, DiskImageChildRef, DiskImageObject, PartitionChild, PartitionChildRef,
    PartitionObject, PartitionSystemChild, PartitionSystemChildRef, PartitionSystemObject,
    VolumeChild, VolumeChildRef, VolumeObject,
};
