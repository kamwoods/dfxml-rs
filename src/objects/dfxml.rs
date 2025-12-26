//! DFXMLObject - the root document container for DFXML.
//!
//! This is the top-level object that contains all other DFXML elements,
//! including metadata about the creator, source images, and child objects.

use crate::objects::common::{DFXML_VERSION, XMLNS_DC, XMLNS_DELTA, XMLNS_DFXML, XMLNS_DFXML_EXT};
use crate::objects::fileobject::FileObject;
use crate::objects::volume::{DiskImageObject, PartitionObject, PartitionSystemObject, VolumeObject};
use std::collections::{HashMap, HashSet};

/// Information about a library used to create or build the DFXML.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LibraryObject {
    /// Library name
    pub name: Option<String>,
    /// Library version
    pub version: Option<String>,
}

impl LibraryObject {
    /// Creates a new LibraryObject with name and version.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            version: Some(version.into()),
        }
    }

    /// Creates an empty LibraryObject.
    pub fn empty() -> Self {
        Self {
            name: None,
            version: None,
        }
    }

    /// Returns true if the libraries match, allowing for missing versions.
    pub fn relaxed_eq(&self, other: &LibraryObject) -> bool {
        if self.name != other.name {
            return false;
        }
        if self.version.is_none() || other.version.is_none() {
            return true;
        }
        self.version == other.version
    }
}

impl Default for LibraryObject {
    fn default() -> Self {
        Self::empty()
    }
}

/// The root DFXML document object.
///
/// DFXMLObject is the top-level container that holds:
/// - Document metadata (version, creator info, command line)
/// - Source image filenames
/// - Namespaces
/// - Child objects (disk images, volumes, files)
/// - Build environment information
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DFXMLObject {
    // === Document Metadata ===
    /// DFXML schema version
    pub version: String,
    /// Program that created this DFXML
    pub program: Option<String>,
    /// Version of the creating program
    pub program_version: Option<String>,
    /// Command line used to create this DFXML
    pub command_line: Option<String>,

    // === Sources ===
    /// Source image filenames
    pub sources: Vec<String>,

    // === Libraries ===
    /// Libraries used to create this DFXML
    creator_libraries: Vec<LibraryObject>,
    /// Libraries used in the build environment
    build_libraries: Vec<LibraryObject>,

    // === Dublin Core Metadata ===
    /// Dublin Core metadata elements
    #[cfg_attr(feature = "serde", serde(skip))]
    pub dc: HashMap<String, String>,

    // === Namespaces ===
    /// XML namespaces (prefix -> URI)
    #[cfg_attr(feature = "serde", serde(skip))]
    namespaces: HashMap<String, String>,

    // === Child Objects ===
    /// Disk images directly attached to this document
    #[cfg_attr(feature = "serde", serde(skip))]
    disk_images: Vec<DiskImageObject>,
    /// Partition systems directly attached to this document
    #[cfg_attr(feature = "serde", serde(skip))]
    partition_systems: Vec<PartitionSystemObject>,
    /// Partitions directly attached to this document
    #[cfg_attr(feature = "serde", serde(skip))]
    partitions: Vec<PartitionObject>,
    /// Volumes directly attached to this document
    #[cfg_attr(feature = "serde", serde(skip))]
    volumes: Vec<VolumeObject>,
    /// Files directly attached to this document (not in a volume)
    #[cfg_attr(feature = "serde", serde(skip))]
    files: Vec<FileObject>,

    // === Differential Analysis ===
    /// Properties to ignore when diffing files
    pub diff_file_ignores: HashSet<String>,
}

impl DFXMLObject {
    /// Creates a new DFXMLObject with default settings.
    pub fn new() -> Self {
        let mut obj = Self {
            version: DFXML_VERSION.to_string(),
            ..Default::default()
        };

        // Add default namespaces
        obj.add_namespace("", XMLNS_DFXML);
        obj.add_namespace("dc", XMLNS_DC);
        obj.add_namespace("delta", XMLNS_DELTA);
        obj.add_namespace("dfxmlext", XMLNS_DFXML_EXT);

        obj
    }

    /// Creates a DFXMLObject with a specific version.
    pub fn with_version(version: impl Into<String>) -> Self {
        let mut obj = Self::new();
        obj.version = version.into();
        obj
    }

    // === Namespace Management ===

    /// Adds a namespace to the document.
    ///
    /// If the prefix already exists, the existing mapping is preserved.
    pub fn add_namespace(&mut self, prefix: impl Into<String>, uri: impl Into<String>) {
        let prefix = prefix.into();
        if !self.namespaces.contains_key(&prefix) {
            self.namespaces.insert(prefix, uri.into());
        }
    }

    /// Returns an iterator over namespaces (prefix, uri).
    pub fn namespaces(&self) -> impl Iterator<Item = (&str, &str)> {
        self.namespaces.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    // === Library Management ===

    /// Adds a creator library.
    pub fn add_creator_library(&mut self, library: LibraryObject) {
        self.creator_libraries.push(library);
    }

    /// Adds a build library.
    pub fn add_build_library(&mut self, library: LibraryObject) {
        self.build_libraries.push(library);
    }

    /// Returns an iterator over creator libraries.
    pub fn creator_libraries(&self) -> impl Iterator<Item = &LibraryObject> {
        self.creator_libraries.iter()
    }

    /// Returns an iterator over build libraries.
    pub fn build_libraries(&self) -> impl Iterator<Item = &LibraryObject> {
        self.build_libraries.iter()
    }

    // === Child Object Management ===

    /// Appends a disk image to the document.
    pub fn append_disk_image(&mut self, disk_image: DiskImageObject) {
        self.disk_images.push(disk_image);
    }

    /// Appends a partition system to the document.
    pub fn append_partition_system(&mut self, ps: PartitionSystemObject) {
        self.partition_systems.push(ps);
    }

    /// Appends a partition to the document.
    pub fn append_partition(&mut self, partition: PartitionObject) {
        self.partitions.push(partition);
    }

    /// Appends a volume to the document.
    pub fn append_volume(&mut self, volume: VolumeObject) {
        self.volumes.push(volume);
    }

    /// Appends a file to the document (not attached to a volume).
    pub fn append_file(&mut self, file: FileObject) {
        self.files.push(file);
    }

    // === Accessors ===

    /// Returns an iterator over disk images.
    pub fn disk_images(&self) -> impl Iterator<Item = &DiskImageObject> {
        self.disk_images.iter()
    }

    /// Returns a mutable iterator over disk images.
    pub fn disk_images_mut(&mut self) -> impl Iterator<Item = &mut DiskImageObject> {
        self.disk_images.iter_mut()
    }

    /// Returns the number of disk images.
    pub fn disk_image_count(&self) -> usize {
        self.disk_images.len()
    }

    /// Returns an iterator over partition systems.
    pub fn partition_systems(&self) -> impl Iterator<Item = &PartitionSystemObject> {
        self.partition_systems.iter()
    }

    /// Returns an iterator over partitions.
    pub fn partitions(&self) -> impl Iterator<Item = &PartitionObject> {
        self.partitions.iter()
    }

    /// Returns an iterator over volumes.
    pub fn volumes(&self) -> impl Iterator<Item = &VolumeObject> {
        self.volumes.iter()
    }

    /// Returns a mutable iterator over volumes.
    pub fn volumes_mut(&mut self) -> impl Iterator<Item = &mut VolumeObject> {
        self.volumes.iter_mut()
    }

    /// Returns the number of volumes.
    pub fn volume_count(&self) -> usize {
        self.volumes.len()
    }

    /// Returns an iterator over files directly attached to the document.
    pub fn files(&self) -> impl Iterator<Item = &FileObject> {
        self.files.iter()
    }

    /// Returns a mutable iterator over files.
    pub fn files_mut(&mut self) -> impl Iterator<Item = &mut FileObject> {
        self.files.iter_mut()
    }

    /// Returns the number of files directly attached to the document.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    // === Iteration ===

    /// Returns an iterator that yields all child objects in depth-first order.
    ///
    /// This recursively yields disk images, volumes, and files.
    pub fn iter(&self) -> DFXMLIterator<'_> {
        DFXMLIterator::new(self)
    }

    /// Returns an iterator over all files in the document.
    ///
    /// This includes files in disk images, partition systems, partitions,
    /// volumes, and files directly attached to the document.
    pub fn iter_files(&self) -> impl Iterator<Item = &FileObject> {
        // Files directly attached
        let direct_files = self.files.iter();

        // Files in volumes directly attached
        let volume_files = self.volumes.iter().flat_map(|v| v.iter_all_files());

        // Files in disk images
        let disk_image_files = self.disk_images.iter().flat_map(|di| {
            di.files()
                .chain(di.volumes().flat_map(|v| v.iter_all_files()))
        });

        direct_files
            .chain(volume_files)
            .chain(disk_image_files)
    }
}

/// An enum representing any child object in a DFXML document.
#[derive(Debug)]
pub enum DFXMLChild<'a> {
    /// A disk image object
    DiskImage(&'a DiskImageObject),
    /// A partition system object
    PartitionSystem(&'a PartitionSystemObject),
    /// A partition object
    Partition(&'a PartitionObject),
    /// A volume object
    Volume(&'a VolumeObject),
    /// A file object
    File(&'a FileObject),
}

/// Iterator over all child objects in a DFXMLObject.
pub struct DFXMLIterator<'a> {
    /// Stack of iterators for depth-first traversal
    disk_images: std::slice::Iter<'a, DiskImageObject>,
    partition_systems: std::slice::Iter<'a, PartitionSystemObject>,
    partitions: std::slice::Iter<'a, PartitionObject>,
    volumes: std::slice::Iter<'a, VolumeObject>,
    files: std::slice::Iter<'a, FileObject>,
    /// Reserved for future depth-first traversal into volume contents
    #[allow(dead_code)]
    current_volume_files: Option<Box<dyn Iterator<Item = &'a FileObject> + 'a>>,
}

impl<'a> DFXMLIterator<'a> {
    fn new(doc: &'a DFXMLObject) -> Self {
        Self {
            disk_images: doc.disk_images.iter(),
            partition_systems: doc.partition_systems.iter(),
            partitions: doc.partitions.iter(),
            volumes: doc.volumes.iter(),
            files: doc.files.iter(),
            current_volume_files: None,
        }
    }
}

impl<'a> Iterator for DFXMLIterator<'a> {
    type Item = DFXMLChild<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // First yield disk images
        if let Some(di) = self.disk_images.next() {
            return Some(DFXMLChild::DiskImage(di));
        }

        // Then partition systems
        if let Some(ps) = self.partition_systems.next() {
            return Some(DFXMLChild::PartitionSystem(ps));
        }

        // Then partitions
        if let Some(p) = self.partitions.next() {
            return Some(DFXMLChild::Partition(p));
        }

        // Then volumes (and their files)
        if let Some(v) = self.volumes.next() {
            return Some(DFXMLChild::Volume(v));
        }

        // Then files
        if let Some(f) = self.files.next() {
            return Some(DFXMLChild::File(f));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dfxml_object_new() {
        let doc = DFXMLObject::new();
        assert_eq!(doc.version, DFXML_VERSION);
        assert!(doc.namespaces.contains_key(""));
        assert!(doc.namespaces.contains_key("dc"));
    }

    #[test]
    fn test_dfxml_add_namespace() {
        let mut doc = DFXMLObject::new();
        doc.add_namespace("custom", "http://example.com/custom");

        let ns: HashMap<_, _> = doc.namespaces().map(|(k, v)| (k, v)).collect();
        assert_eq!(ns.get("custom"), Some(&"http://example.com/custom"));
    }

    #[test]
    fn test_dfxml_creator_info() {
        let mut doc = DFXMLObject::new();
        doc.program = Some("fiwalk".to_string());
        doc.program_version = Some("0.7.4".to_string());
        doc.command_line = Some("fiwalk -X image.raw".to_string());
        doc.sources.push("image.raw".to_string());

        assert_eq!(doc.program, Some("fiwalk".to_string()));
        assert_eq!(doc.sources.len(), 1);
    }

    #[test]
    fn test_dfxml_libraries() {
        let mut doc = DFXMLObject::new();
        doc.add_creator_library(LibraryObject::new("libewf", "20140608"));
        doc.add_build_library(LibraryObject::new("libtsk", "4.6.0"));

        assert_eq!(doc.creator_libraries().count(), 1);
        assert_eq!(doc.build_libraries().count(), 1);
    }

    #[test]
    fn test_dfxml_with_children() {
        let mut doc = DFXMLObject::new();

        // Add a volume with files
        let mut vol = VolumeObject::with_ftype("ntfs");
        vol.append_file(FileObject::with_filename("file1.txt"));
        vol.append_file(FileObject::with_filename("file2.txt"));
        doc.append_volume(vol);

        // Add a file directly
        doc.append_file(FileObject::with_filename("direct.txt"));

        assert_eq!(doc.volume_count(), 1);
        assert_eq!(doc.file_count(), 1);

        // Test iter_files
        let all_files: Vec<_> = doc.iter_files().collect();
        assert_eq!(all_files.len(), 3);
    }

    #[test]
    fn test_library_relaxed_eq() {
        let lib1 = LibraryObject::new("test", "1.0");
        let lib2 = LibraryObject::new("test", "1.0");
        let lib3 = LibraryObject {
            name: Some("test".to_string()),
            version: None,
        };

        assert!(lib1.relaxed_eq(&lib2));
        assert!(lib1.relaxed_eq(&lib3)); // Version None matches anything
    }

    #[test]
    fn test_dfxml_iteration() {
        let mut doc = DFXMLObject::new();
        doc.append_volume(VolumeObject::new());
        doc.append_file(FileObject::new());

        let items: Vec<_> = doc.iter().collect();
        assert_eq!(items.len(), 2);

        assert!(matches!(items[0], DFXMLChild::Volume(_)));
        assert!(matches!(items[1], DFXMLChild::File(_)));
    }
}
