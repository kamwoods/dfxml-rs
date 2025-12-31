//! DFXMLObject - the root document container for DFXML.
//!
//! This is the top-level object that contains all other DFXML elements,
//! including metadata about the creator, source images, and child objects.

use crate::objects::common::{DFXML_VERSION, XMLNS_DC, XMLNS_DELTA, XMLNS_DFXML, XMLNS_DFXML_EXT};
use crate::objects::fileobject::FileObject;
use crate::objects::volume::{
    DiskImageObject, PartitionObject, PartitionSystemObject, VolumeObject,
};
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
/// - External elements from non-DFXML namespaces
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

    // === External Elements ===
    /// Elements from non-DFXML namespaces (preserved for round-tripping)
    pub externals: crate::objects::common::Externals,

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
        self.namespaces.entry(prefix).or_insert_with(|| uri.into());
    }

    /// Returns an iterator over namespaces (prefix, uri).
    pub fn namespaces(&self) -> impl Iterator<Item = (&str, &str)> {
        self.namespaces
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
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

    /// Appends any child object to the document.
    ///
    /// This is a unified method that accepts any valid child type via the `ChildObject` enum.
    /// For convenience, you can use `.into()` to convert from specific types.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dfxml_rs::objects::{DFXMLObject, VolumeObject, FileObject, ChildObject};
    ///
    /// let mut doc = DFXMLObject::new();
    ///
    /// // Direct ChildObject usage
    /// doc.append(ChildObject::Volume(VolumeObject::new()));
    ///
    /// // Using From trait for ergonomic conversion
    /// doc.append(FileObject::with_filename("test.txt").into());
    /// ```
    pub fn append(&mut self, child: ChildObject) {
        match child {
            ChildObject::DiskImage(di) => self.disk_images.push(di),
            ChildObject::PartitionSystem(ps) => self.partition_systems.push(ps),
            ChildObject::Partition(p) => self.partitions.push(p),
            ChildObject::Volume(v) => self.volumes.push(v),
            ChildObject::File(f) => self.files.push(*f),
        }
    }

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

    /// Returns an iterator over direct child objects only.
    ///
    /// This yields only the immediate children of this document, not their descendants.
    /// For recursive traversal, use `iter_descendants()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dfxml_rs::objects::{DFXMLObject, VolumeObject, FileObject, DFXMLChild};
    ///
    /// let mut doc = DFXMLObject::new();
    /// doc.append_volume(VolumeObject::new());
    /// doc.append_file(FileObject::with_filename("test.txt"));
    ///
    /// for child in doc.child_objects() {
    ///     match child {
    ///         DFXMLChild::Volume(v) => println!("Volume: {:?}", v.ftype_str),
    ///         DFXMLChild::File(f) => println!("File: {:?}", f.filename),
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub fn child_objects(&self) -> DFXMLChildIterator<'_> {
        DFXMLChildIterator::new(self)
    }

    /// Returns an iterator that yields all descendant objects in depth-first order.
    ///
    /// This recursively yields all objects: disk images and their contents,
    /// partition systems and their contents, partitions and their contents,
    /// volumes and their contents, then files.
    ///
    /// This is equivalent to Python's `__iter__` method on DFXMLObject.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dfxml_rs::objects::{DFXMLObject, VolumeObject, FileObject, DFXMLChild};
    ///
    /// let mut doc = DFXMLObject::new();
    /// let mut vol = VolumeObject::new();
    /// vol.append_file(FileObject::with_filename("inner.txt"));
    /// doc.append_volume(vol);
    /// doc.append_file(FileObject::with_filename("outer.txt"));
    ///
    /// // iter_descendants yields: Volume, then inner.txt (depth-first), then outer.txt
    /// for child in doc.iter_descendants() {
    ///     match child {
    ///         DFXMLChild::Volume(_) => println!("Found a volume"),
    ///         DFXMLChild::File(f) => println!("Found file: {:?}", f.filename),
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub fn iter_descendants(&self) -> DFXMLIterator<'_> {
        DFXMLIterator::new(self)
    }

    /// Returns an iterator that yields all child objects in depth-first order.
    ///
    /// This is an alias for `iter_descendants()` for compatibility.
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

        direct_files.chain(volume_files).chain(disk_image_files)
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

/// An owned enum representing any child object that can be appended to a DFXML container.
///
/// This enum is used by the unified `append()` method on container objects like
/// `DFXMLObject`, `VolumeObject`, `PartitionObject`, etc. It allows appending
/// any valid child type without needing to call type-specific methods.
///
/// # Example
///
/// ```rust
/// use dfxml_rs::objects::{DFXMLObject, VolumeObject, FileObject, ChildObject};
///
/// let mut doc = DFXMLObject::new();
///
/// // Using the unified append method with ChildObject
/// doc.append(ChildObject::Volume(VolumeObject::new()));
/// doc.append(ChildObject::File(Box::new(FileObject::with_filename("test.txt"))));
///
/// // Or using From implementations for ergonomic conversion
/// doc.append(VolumeObject::with_ftype("ntfs").into());
/// doc.append(FileObject::with_filename("another.txt").into());
/// ```
#[derive(Debug, Clone)]
pub enum ChildObject {
    /// A disk image object
    DiskImage(DiskImageObject),
    /// A partition system object
    PartitionSystem(PartitionSystemObject),
    /// A partition object
    Partition(PartitionObject),
    /// A volume object
    Volume(VolumeObject),
    /// A file object (boxed to reduce enum size)
    File(Box<FileObject>),
}

impl From<DiskImageObject> for ChildObject {
    fn from(obj: DiskImageObject) -> Self {
        ChildObject::DiskImage(obj)
    }
}

impl From<PartitionSystemObject> for ChildObject {
    fn from(obj: PartitionSystemObject) -> Self {
        ChildObject::PartitionSystem(obj)
    }
}

impl From<PartitionObject> for ChildObject {
    fn from(obj: PartitionObject) -> Self {
        ChildObject::Partition(obj)
    }
}

impl From<VolumeObject> for ChildObject {
    fn from(obj: VolumeObject) -> Self {
        ChildObject::Volume(obj)
    }
}

impl From<FileObject> for ChildObject {
    fn from(obj: FileObject) -> Self {
        ChildObject::File(Box::new(obj))
    }
}

/// Iterator over direct child objects in a DFXMLObject.
///
/// This iterator yields only the immediate children, not their descendants.
/// For recursive traversal, use `iter_descendants()`.
pub struct DFXMLChildIterator<'a> {
    disk_images: std::slice::Iter<'a, DiskImageObject>,
    partition_systems: std::slice::Iter<'a, PartitionSystemObject>,
    partitions: std::slice::Iter<'a, PartitionObject>,
    volumes: std::slice::Iter<'a, VolumeObject>,
    files: std::slice::Iter<'a, FileObject>,
}

impl<'a> DFXMLChildIterator<'a> {
    fn new(doc: &'a DFXMLObject) -> Self {
        Self {
            disk_images: doc.disk_images.iter(),
            partition_systems: doc.partition_systems.iter(),
            partitions: doc.partitions.iter(),
            volumes: doc.volumes.iter(),
            files: doc.files.iter(),
        }
    }
}

impl<'a> Iterator for DFXMLChildIterator<'a> {
    type Item = DFXMLChild<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(di) = self.disk_images.next() {
            return Some(DFXMLChild::DiskImage(di));
        }
        if let Some(ps) = self.partition_systems.next() {
            return Some(DFXMLChild::PartitionSystem(ps));
        }
        if let Some(p) = self.partitions.next() {
            return Some(DFXMLChild::Partition(p));
        }
        if let Some(v) = self.volumes.next() {
            return Some(DFXMLChild::Volume(v));
        }
        if let Some(f) = self.files.next() {
            return Some(DFXMLChild::File(f));
        }
        None
    }
}

/// Iterator over all descendant objects in a DFXML document (depth-first).
///
/// This iterator recursively yields all objects in depth-first order:
/// disk images and their contents, partition systems and their contents,
/// partitions and their contents, volumes and their contents, then files.
pub struct DFXMLIterator<'a> {
    /// Stack for depth-first traversal
    stack: Vec<DFXMLChild<'a>>,
}

impl<'a> DFXMLIterator<'a> {
    fn new(doc: &'a DFXMLObject) -> Self {
        // Build initial stack in reverse order (so first items are popped first)
        let mut stack = Vec::new();

        // Add in reverse order: files, volumes, partitions, partition_systems, disk_images
        for f in doc.files.iter().rev() {
            stack.push(DFXMLChild::File(f));
        }
        for v in doc.volumes.iter().rev() {
            stack.push(DFXMLChild::Volume(v));
        }
        for p in doc.partitions.iter().rev() {
            stack.push(DFXMLChild::Partition(p));
        }
        for ps in doc.partition_systems.iter().rev() {
            stack.push(DFXMLChild::PartitionSystem(ps));
        }
        for di in doc.disk_images.iter().rev() {
            stack.push(DFXMLChild::DiskImage(di));
        }

        Self { stack }
    }

    /// Push children of a container onto the stack (in reverse order for correct traversal)
    fn push_children(&mut self, child: &DFXMLChild<'a>) {
        match child {
            DFXMLChild::DiskImage(di) => {
                // Push in reverse order: files, volumes, partitions, partition_systems
                // Collect into Vec to allow reverse iteration
                let files: Vec<_> = di.files().collect();
                let volumes: Vec<_> = di.volumes().collect();
                let partitions: Vec<_> = di.partitions().collect();
                let partition_systems: Vec<_> = di.partition_systems().collect();

                for f in files.into_iter().rev() {
                    self.stack.push(DFXMLChild::File(f));
                }
                for v in volumes.into_iter().rev() {
                    self.stack.push(DFXMLChild::Volume(v));
                }
                for p in partitions.into_iter().rev() {
                    self.stack.push(DFXMLChild::Partition(p));
                }
                for ps in partition_systems.into_iter().rev() {
                    self.stack.push(DFXMLChild::PartitionSystem(ps));
                }
            }
            DFXMLChild::PartitionSystem(ps) => {
                let files: Vec<_> = ps.files().collect();
                let partitions: Vec<_> = ps.partitions().collect();

                for f in files.into_iter().rev() {
                    self.stack.push(DFXMLChild::File(f));
                }
                for p in partitions.into_iter().rev() {
                    self.stack.push(DFXMLChild::Partition(p));
                }
            }
            DFXMLChild::Partition(p) => {
                let files: Vec<_> = p.files().collect();
                let volumes: Vec<_> = p.volumes().collect();
                let partitions: Vec<_> = p.partitions().collect();
                let partition_systems: Vec<_> = p.partition_systems().collect();

                for f in files.into_iter().rev() {
                    self.stack.push(DFXMLChild::File(f));
                }
                for v in volumes.into_iter().rev() {
                    self.stack.push(DFXMLChild::Volume(v));
                }
                for part in partitions.into_iter().rev() {
                    self.stack.push(DFXMLChild::Partition(part));
                }
                for ps in partition_systems.into_iter().rev() {
                    self.stack.push(DFXMLChild::PartitionSystem(ps));
                }
            }
            DFXMLChild::Volume(v) => {
                let files: Vec<_> = v.files().collect();
                let volumes: Vec<_> = v.volumes().collect();
                let disk_images: Vec<_> = v.disk_images().collect();

                for f in files.into_iter().rev() {
                    self.stack.push(DFXMLChild::File(f));
                }
                for vol in volumes.into_iter().rev() {
                    self.stack.push(DFXMLChild::Volume(vol));
                }
                for di in disk_images.into_iter().rev() {
                    self.stack.push(DFXMLChild::DiskImage(di));
                }
            }
            DFXMLChild::File(_) => {
                // Files have no children
            }
        }
    }
}

impl<'a> Iterator for DFXMLIterator<'a> {
    type Item = DFXMLChild<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(child) = self.stack.pop() {
            // Push this child's children onto the stack for depth-first traversal
            self.push_children(&child);
            Some(child)
        } else {
            None
        }
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

        let ns: HashMap<_, _> = doc.namespaces().collect();
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
