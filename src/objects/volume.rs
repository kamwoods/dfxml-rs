//! VolumeObject - represents a file system volume in DFXML.
//!
//! Volumes are containers for files and represent a single file system
//! (e.g., an NTFS partition, an ext4 file system).

use crate::objects::common::ByteRuns;
use crate::objects::fileobject::FileObject;
use std::collections::HashSet;

/// Represents a file system volume in DFXML.
///
/// VolumeObject is a container that holds:
/// - File system type and metadata
/// - Block/sector geometry
/// - Child FileObjects
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VolumeObject {
    // === Location ===
    /// Offset from the start of the disk image (bytes)
    pub partition_offset: Option<u64>,

    // === Geometry ===
    /// Sector size (bytes)
    pub sector_size: Option<u32>,
    /// Block/cluster size (bytes)
    pub block_size: Option<u32>,
    /// Total number of blocks
    pub block_count: Option<u64>,
    /// First block number
    pub first_block: Option<u64>,
    /// Last block number
    pub last_block: Option<u64>,

    // === File System Type ===
    /// File system type code (numeric)
    pub ftype: Option<i32>,
    /// File system type string (e.g., "ntfs", "ext4")
    pub ftype_str: Option<String>,

    // === Flags ===
    /// Only allocated files were processed
    pub allocated_only: Option<bool>,

    // === Error ===
    /// Error message if volume processing failed
    pub error: Option<String>,

    // === Byte Runs ===
    /// Byte runs for the volume
    pub byte_runs: Option<ByteRuns>,

    // === Child Objects ===
    /// Files contained in this volume
    #[cfg_attr(feature = "serde", serde(skip))]
    files: Vec<FileObject>,

    /// Nested volumes (e.g., for disk images within volumes)
    #[cfg_attr(feature = "serde", serde(skip))]
    volumes: Vec<VolumeObject>,

    // === Differential Analysis ===
    /// Differential annotations
    pub annos: HashSet<String>,
    /// Properties that differ from original
    pub diffs: HashSet<String>,
    /// Reference to the original volume (for differencing)
    #[cfg_attr(feature = "serde", serde(skip))]
    pub original_volume: Option<Box<VolumeObject>>,
}

impl VolumeObject {
    /// Creates a new empty VolumeObject.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a VolumeObject with a file system type string.
    pub fn with_ftype(ftype_str: impl Into<String>) -> Self {
        Self {
            ftype_str: Some(ftype_str.into()),
            ..Default::default()
        }
    }

    /// Appends a FileObject to this volume.
    pub fn append_file(&mut self, file: FileObject) {
        self.files.push(file);
    }

    /// Appends a nested VolumeObject to this volume.
    pub fn append_volume(&mut self, volume: VolumeObject) {
        self.volumes.push(volume);
    }

    /// Returns an iterator over the files in this volume.
    pub fn files(&self) -> impl Iterator<Item = &FileObject> {
        self.files.iter()
    }

    /// Returns a mutable iterator over the files in this volume.
    pub fn files_mut(&mut self) -> impl Iterator<Item = &mut FileObject> {
        self.files.iter_mut()
    }

    /// Returns the number of files in this volume.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Returns an iterator over nested volumes.
    pub fn volumes(&self) -> impl Iterator<Item = &VolumeObject> {
        self.volumes.iter()
    }

    /// Returns a mutable iterator over nested volumes.
    pub fn volumes_mut(&mut self) -> impl Iterator<Item = &mut VolumeObject> {
        self.volumes.iter_mut()
    }

    /// Returns an iterator that recursively yields all files.
    ///
    /// This includes files in nested volumes.
    pub fn iter_all_files(&self) -> Box<dyn Iterator<Item = &FileObject> + '_> {
        Box::new(
            self.files
                .iter()
                .chain(self.volumes.iter().flat_map(|v| v.iter_all_files())),
        )
    }

    /// Compares this volume to another, returning the set of differing properties.
    pub fn compare_to(&self, other: &VolumeObject) -> HashSet<String> {
        let mut diffs = HashSet::new();

        macro_rules! compare_field {
            ($field:ident) => {
                if self.$field != other.$field {
                    diffs.insert(stringify!($field).to_string());
                }
            };
        }

        compare_field!(partition_offset);
        compare_field!(sector_size);
        compare_field!(block_size);
        compare_field!(block_count);
        compare_field!(first_block);
        compare_field!(last_block);
        compare_field!(ftype);
        compare_field!(ftype_str);
        compare_field!(allocated_only);

        diffs
    }
}

/// Represents a disk partition in DFXML.
///
/// Partitions are intermediate containers between disk images and volumes/file systems.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PartitionObject {
    /// Partition index/number
    pub partition_index: Option<u32>,
    /// Partition type code
    pub ptype: Option<u32>,
    /// Partition type string
    pub ptype_str: Option<String>,
    /// File system type string within the partition
    pub ftype_str: Option<String>,
    /// Partition label/name
    pub partition_label: Option<String>,
    /// GUID (for GPT partitions)
    pub guid: Option<String>,
    /// Block count
    pub block_count: Option<u64>,
    /// Block size
    pub block_size: Option<u32>,
    /// Offset within partition system (bytes)
    pub partition_system_offset: Option<u64>,

    /// Byte runs for the partition
    pub byte_runs: Option<ByteRuns>,

    // Child objects
    #[cfg_attr(feature = "serde", serde(skip))]
    volumes: Vec<VolumeObject>,
    #[cfg_attr(feature = "serde", serde(skip))]
    files: Vec<FileObject>,
    #[cfg_attr(feature = "serde", serde(skip))]
    partitions: Vec<PartitionObject>,
    #[cfg_attr(feature = "serde", serde(skip))]
    partition_systems: Vec<PartitionSystemObject>,
}

impl PartitionObject {
    /// Creates a new empty PartitionObject.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a VolumeObject to this partition.
    pub fn append_volume(&mut self, volume: VolumeObject) {
        self.volumes.push(volume);
    }

    /// Appends a FileObject to this partition (for slack space discoveries).
    pub fn append_file(&mut self, file: FileObject) {
        self.files.push(file);
    }

    /// Appends a nested PartitionObject.
    pub fn append_partition(&mut self, partition: PartitionObject) {
        self.partitions.push(partition);
    }

    /// Appends a PartitionSystemObject.
    pub fn append_partition_system(&mut self, ps: PartitionSystemObject) {
        self.partition_systems.push(ps);
    }

    /// Returns an iterator over volumes in this partition.
    pub fn volumes(&self) -> impl Iterator<Item = &VolumeObject> {
        self.volumes.iter()
    }

    /// Returns an iterator over files directly attached to this partition.
    pub fn files(&self) -> impl Iterator<Item = &FileObject> {
        self.files.iter()
    }

    /// Returns an iterator over nested partitions.
    pub fn partitions(&self) -> impl Iterator<Item = &PartitionObject> {
        self.partitions.iter()
    }
}

/// Represents a partition system (e.g., MBR, GPT) in DFXML.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PartitionSystemObject {
    /// Partition system type string (e.g., "dos", "gpt")
    pub pstype_str: Option<String>,
    /// Block size
    pub block_size: Option<u32>,
    /// Volume name
    pub volume_name: Option<String>,
    /// GUID (for GPT)
    pub guid: Option<String>,
    /// Error message
    pub error: Option<String>,
    /// Byte runs
    pub byte_runs: Option<ByteRuns>,

    // Child objects
    #[cfg_attr(feature = "serde", serde(skip))]
    partitions: Vec<PartitionObject>,
    #[cfg_attr(feature = "serde", serde(skip))]
    files: Vec<FileObject>,
}

impl PartitionSystemObject {
    /// Creates a new empty PartitionSystemObject.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a PartitionSystemObject with a type string.
    pub fn with_pstype(pstype_str: impl Into<String>) -> Self {
        Self {
            pstype_str: Some(pstype_str.into()),
            ..Default::default()
        }
    }

    /// Appends a PartitionObject.
    pub fn append_partition(&mut self, partition: PartitionObject) {
        self.partitions.push(partition);
    }

    /// Appends a FileObject (for slack space discoveries).
    pub fn append_file(&mut self, file: FileObject) {
        self.files.push(file);
    }

    /// Returns an iterator over partitions.
    pub fn partitions(&self) -> impl Iterator<Item = &PartitionObject> {
        self.partitions.iter()
    }

    /// Returns an iterator over files directly attached.
    pub fn files(&self) -> impl Iterator<Item = &FileObject> {
        self.files.iter()
    }
}

/// Represents a disk image in DFXML.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DiskImageObject {
    /// Image filename
    pub image_filename: Option<String>,
    /// Image size in bytes
    pub image_size: Option<u64>,
    /// Sector size
    pub sector_size: Option<u32>,
    /// Byte runs
    pub byte_runs: Option<ByteRuns>,
    /// Hashes of the disk image
    pub hashes: crate::objects::common::Hashes,
    /// Error message
    pub error: Option<String>,

    // Child objects
    #[cfg_attr(feature = "serde", serde(skip))]
    partition_systems: Vec<PartitionSystemObject>,
    #[cfg_attr(feature = "serde", serde(skip))]
    partitions: Vec<PartitionObject>,
    #[cfg_attr(feature = "serde", serde(skip))]
    volumes: Vec<VolumeObject>,
    #[cfg_attr(feature = "serde", serde(skip))]
    files: Vec<FileObject>,
}

impl DiskImageObject {
    /// Creates a new empty DiskImageObject.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a DiskImageObject with an image filename.
    pub fn with_filename(filename: impl Into<String>) -> Self {
        Self {
            image_filename: Some(filename.into()),
            ..Default::default()
        }
    }

    /// Appends a PartitionSystemObject.
    pub fn append_partition_system(&mut self, ps: PartitionSystemObject) {
        self.partition_systems.push(ps);
    }

    /// Appends a PartitionObject.
    pub fn append_partition(&mut self, partition: PartitionObject) {
        self.partitions.push(partition);
    }

    /// Appends a VolumeObject.
    pub fn append_volume(&mut self, volume: VolumeObject) {
        self.volumes.push(volume);
    }

    /// Appends a FileObject.
    pub fn append_file(&mut self, file: FileObject) {
        self.files.push(file);
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

    /// Returns an iterator over files directly attached.
    pub fn files(&self) -> impl Iterator<Item = &FileObject> {
        self.files.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_object_new() {
        let vol = VolumeObject::new();
        assert!(vol.ftype_str.is_none());
        assert_eq!(vol.file_count(), 0);
    }

    #[test]
    fn test_volume_with_files() {
        let mut vol = VolumeObject::with_ftype("ntfs");
        vol.append_file(FileObject::with_filename("file1.txt"));
        vol.append_file(FileObject::with_filename("file2.txt"));

        assert_eq!(vol.ftype_str, Some("ntfs".to_string()));
        assert_eq!(vol.file_count(), 2);
    }

    #[test]
    fn test_volume_nested_iteration() {
        let mut vol = VolumeObject::new();
        vol.append_file(FileObject::with_filename("outer.txt"));

        let mut inner_vol = VolumeObject::new();
        inner_vol.append_file(FileObject::with_filename("inner.txt"));
        vol.append_volume(inner_vol);

        let all_files: Vec<_> = vol.iter_all_files().collect();
        assert_eq!(all_files.len(), 2);
    }

    #[test]
    fn test_partition_system() {
        let mut ps = PartitionSystemObject::with_pstype("gpt");
        let mut part = PartitionObject::new();
        part.partition_index = Some(1);
        ps.append_partition(part);

        assert_eq!(ps.pstype_str, Some("gpt".to_string()));
        assert_eq!(ps.partitions().count(), 1);
    }

    #[test]
    fn test_disk_image() {
        let mut di = DiskImageObject::with_filename("test.E01");
        di.image_size = Some(1024 * 1024 * 1024); // 1 GB
        di.append_volume(VolumeObject::with_ftype("ntfs"));

        assert_eq!(di.image_filename, Some("test.E01".to_string()));
        assert_eq!(di.volumes().count(), 1);
    }
}
