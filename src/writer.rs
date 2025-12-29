//! DFXML writer for serializing objects to XML.
//!
//! This module provides functionality to serialize DFXML objects to XML format
//! with proper namespace handling.
//!
//! # Example
//!
//! ```rust
//! use dfxml_rs::objects::{DFXMLObject, VolumeObject, FileObject, HashType};
//! use dfxml_rs::writer::DFXMLWriter;
//!
//! let mut doc = DFXMLObject::new();
//! doc.program = Some("my-tool".to_string());
//! doc.program_version = Some("1.0.0".to_string());
//!
//! let mut volume = VolumeObject::with_ftype("ntfs");
//! let mut file = FileObject::with_filename("test.txt");
//! file.filesize = Some(1024);
//! file.hashes.set(HashType::Md5, "d41d8cd98f00b204e9800998ecf8427e".to_string());
//! volume.append_file(file);
//! doc.append_volume(volume);
//!
//! let mut writer = DFXMLWriter::new();
//! let xml = writer.write_to_string(&doc).unwrap();
//! println!("{}", xml);
//! ```

use crate::error::Result;
use crate::objects::{
    ByteRun, ByteRunFacet, ByteRuns, DFXMLObject, DiskImageObject, FileObject, HashType,
    LibraryObject, PartitionObject, PartitionSystemObject, Timestamp, VolumeObject, XMLNS_DC,
    XMLNS_DFXML,
};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::Write;

/// Configuration options for the DFXML writer.
#[derive(Debug, Clone)]
pub struct WriterConfig {
    /// Whether to indent the output for readability
    pub indent: bool,
    /// Indentation string (default: two spaces)
    pub indent_string: String,
    /// Whether to include the XML declaration
    pub xml_declaration: bool,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            indent: true,
            indent_string: "  ".to_string(),
            xml_declaration: true,
        }
    }
}

impl WriterConfig {
    /// Creates a new configuration with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a compact configuration (no indentation).
    pub fn compact() -> Self {
        Self {
            indent: false,
            indent_string: String::new(),
            xml_declaration: true,
        }
    }

    /// Sets whether to indent the output.
    pub fn with_indent(mut self, indent: bool) -> Self {
        self.indent = indent;
        self
    }

    /// Sets the indentation string.
    pub fn with_indent_string(mut self, s: impl Into<String>) -> Self {
        self.indent_string = s.into();
        self
    }
}

/// DFXML XML writer.
///
/// Serializes DFXML objects to XML format with proper namespace handling.
pub struct DFXMLWriter {
    config: WriterConfig,
}

impl DFXMLWriter {
    /// Creates a new writer with default configuration.
    pub fn new() -> Self {
        Self {
            config: WriterConfig::default(),
        }
    }

    /// Creates a new writer with the specified configuration.
    pub fn with_config(config: WriterConfig) -> Self {
        Self { config }
    }

    /// Writes a DFXMLObject to a string.
    pub fn write_to_string(&self, doc: &DFXMLObject) -> Result<String> {
        let mut buffer = Vec::new();
        self.write(doc, &mut buffer)?;
        Ok(String::from_utf8(buffer).expect("Generated XML should be valid UTF-8"))
    }

    /// Writes a DFXMLObject to any Write implementation.
    pub fn write<W: Write>(&self, doc: &DFXMLObject, writer: W) -> Result<()> {
        let mut xml_writer = if self.config.indent {
            Writer::new_with_indent(writer, b' ', self.config.indent_string.len())
        } else {
            Writer::new(writer)
        };

        // XML declaration
        if self.config.xml_declaration {
            xml_writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
            if self.config.indent {
                xml_writer.get_mut().write_all(b"\n")?;
            }
        }

        // Root dfxml element with namespaces
        let mut dfxml_start = BytesStart::new("dfxml");
        dfxml_start.push_attribute(("version", doc.version.as_str()));
        dfxml_start.push_attribute(("xmlns", XMLNS_DFXML));
        dfxml_start.push_attribute(("xmlns:dc", XMLNS_DC));
        xml_writer.write_event(Event::Start(dfxml_start))?;

        // Write metadata/creator section
        self.write_creator(&mut xml_writer, doc)?;

        // Write source images
        for source in &doc.sources {
            self.write_simple_element(&mut xml_writer, "image_filename", source)?;
        }

        // Write disk images
        for di in doc.disk_images() {
            self.write_disk_image(&mut xml_writer, di)?;
        }

        // Write partition systems
        for ps in doc.partition_systems() {
            self.write_partition_system(&mut xml_writer, ps)?;
        }

        // Write partitions
        for p in doc.partitions() {
            self.write_partition(&mut xml_writer, p)?;
        }

        // Write volumes
        for vol in doc.volumes() {
            self.write_volume(&mut xml_writer, vol)?;
        }

        // Write files directly attached to document
        for file in doc.files() {
            self.write_file(&mut xml_writer, file)?;
        }

        // Close dfxml
        xml_writer.write_event(Event::End(BytesEnd::new("dfxml")))?;

        Ok(())
    }

    /// Writes the creator section.
    fn write_creator<W: Write>(&self, writer: &mut Writer<W>, doc: &DFXMLObject) -> Result<()> {
        // Only write creator if there's something to write
        if doc.program.is_none()
            && doc.program_version.is_none()
            && doc.command_line.is_none()
            && doc.creator_libraries().count() == 0
        {
            return Ok(());
        }

        writer.write_event(Event::Start(BytesStart::new("creator")))?;

        if let Some(ref program) = doc.program {
            self.write_simple_element(writer, "program", program)?;
        }
        if let Some(ref version) = doc.program_version {
            self.write_simple_element(writer, "version", version)?;
        }
        if let Some(ref cmd) = doc.command_line {
            self.write_simple_element(writer, "command_line", cmd)?;
        }

        // Write creator libraries
        for lib in doc.creator_libraries() {
            self.write_library(writer, lib)?;
        }

        writer.write_event(Event::End(BytesEnd::new("creator")))?;

        // Write build environment if there are build libraries
        if doc.build_libraries().count() > 0 {
            writer.write_event(Event::Start(BytesStart::new("build_environment")))?;
            for lib in doc.build_libraries() {
                self.write_library(writer, lib)?;
            }
            writer.write_event(Event::End(BytesEnd::new("build_environment")))?;
        }

        Ok(())
    }

    /// Writes a library element.
    fn write_library<W: Write>(&self, writer: &mut Writer<W>, lib: &LibraryObject) -> Result<()> {
        let mut elem = BytesStart::new("library");
        if let Some(ref name) = lib.name {
            elem.push_attribute(("name", name.as_str()));
        }
        if let Some(ref version) = lib.version {
            elem.push_attribute(("version", version.as_str()));
        }
        writer.write_event(Event::Empty(elem))?;
        Ok(())
    }

    /// Writes a disk image object.
    fn write_disk_image<W: Write>(
        &self,
        writer: &mut Writer<W>,
        di: &DiskImageObject,
    ) -> Result<()> {
        writer.write_event(Event::Start(BytesStart::new("diskimageobject")))?;

        if let Some(ref filename) = di.image_filename {
            self.write_simple_element(writer, "image_filename", filename)?;
        }
        if let Some(size) = di.image_size {
            self.write_simple_element(writer, "imagesize", &size.to_string())?;
        }
        if let Some(sector_size) = di.sector_size {
            self.write_simple_element(writer, "sector_size", &sector_size.to_string())?;
        }

        // Write hashes
        self.write_hashes(writer, &di.hashes)?;

        // Write byte runs
        if let Some(ref brs) = di.byte_runs {
            self.write_byte_runs(writer, brs)?;
        }

        // Write child partition systems
        for ps in di.partition_systems() {
            self.write_partition_system(writer, ps)?;
        }

        // Write child partitions
        for p in di.partitions() {
            self.write_partition(writer, p)?;
        }

        // Write child volumes
        for vol in di.volumes() {
            self.write_volume(writer, vol)?;
        }

        // Write child files
        for file in di.files() {
            self.write_file(writer, file)?;
        }

        if let Some(ref error) = di.error {
            self.write_simple_element(writer, "error", error)?;
        }

        writer.write_event(Event::End(BytesEnd::new("diskimageobject")))?;
        Ok(())
    }

    /// Writes a partition system object.
    fn write_partition_system<W: Write>(
        &self,
        writer: &mut Writer<W>,
        ps: &PartitionSystemObject,
    ) -> Result<()> {
        writer.write_event(Event::Start(BytesStart::new("partitionsystemobject")))?;

        if let Some(ref pstype) = ps.pstype_str {
            self.write_simple_element(writer, "pstype_str", pstype)?;
        }
        if let Some(block_size) = ps.block_size {
            self.write_simple_element(writer, "block_size", &block_size.to_string())?;
        }
        if let Some(ref volume_name) = ps.volume_name {
            self.write_simple_element(writer, "volume_name", volume_name)?;
        }
        if let Some(ref guid) = ps.guid {
            self.write_simple_element(writer, "guid", guid)?;
        }

        if let Some(ref brs) = ps.byte_runs {
            self.write_byte_runs(writer, brs)?;
        }

        // Write child partitions
        for p in ps.partitions() {
            self.write_partition(writer, p)?;
        }

        // Write child files
        for file in ps.files() {
            self.write_file(writer, file)?;
        }

        if let Some(ref error) = ps.error {
            self.write_simple_element(writer, "error", error)?;
        }

        writer.write_event(Event::End(BytesEnd::new("partitionsystemobject")))?;
        Ok(())
    }

    /// Writes a partition object.
    fn write_partition<W: Write>(&self, writer: &mut Writer<W>, p: &PartitionObject) -> Result<()> {
        writer.write_event(Event::Start(BytesStart::new("partitionobject")))?;

        if let Some(idx) = p.partition_index {
            self.write_simple_element(writer, "partition_index", &idx.to_string())?;
        }
        if let Some(ptype) = p.ptype {
            self.write_simple_element(writer, "ptype", &ptype.to_string())?;
        }
        if let Some(ref ptype_str) = p.ptype_str {
            self.write_simple_element(writer, "ptype_str", ptype_str)?;
        }
        if let Some(ref ftype_str) = p.ftype_str {
            self.write_simple_element(writer, "ftype_str", ftype_str)?;
        }
        if let Some(ref label) = p.partition_label {
            self.write_simple_element(writer, "partition_label", label)?;
        }
        if let Some(ref guid) = p.guid {
            self.write_simple_element(writer, "guid", guid)?;
        }
        if let Some(block_count) = p.block_count {
            self.write_simple_element(writer, "block_count", &block_count.to_string())?;
        }
        if let Some(block_size) = p.block_size {
            self.write_simple_element(writer, "block_size", &block_size.to_string())?;
        }

        if let Some(ref brs) = p.byte_runs {
            self.write_byte_runs(writer, brs)?;
        }

        // Write child volumes
        for vol in p.volumes() {
            self.write_volume(writer, vol)?;
        }

        // Write child partitions
        for sub_p in p.partitions() {
            self.write_partition(writer, sub_p)?;
        }

        // Write child files
        for file in p.files() {
            self.write_file(writer, file)?;
        }

        writer.write_event(Event::End(BytesEnd::new("partitionobject")))?;
        Ok(())
    }

    /// Writes a volume object.
    fn write_volume<W: Write>(&self, writer: &mut Writer<W>, vol: &VolumeObject) -> Result<()> {
        writer.write_event(Event::Start(BytesStart::new("volume")))?;

        if let Some(offset) = vol.partition_offset {
            self.write_simple_element(writer, "partition_offset", &offset.to_string())?;
        }
        if let Some(sector_size) = vol.sector_size {
            self.write_simple_element(writer, "sector_size", &sector_size.to_string())?;
        }
        if let Some(block_size) = vol.block_size {
            self.write_simple_element(writer, "block_size", &block_size.to_string())?;
        }
        if let Some(ftype) = vol.ftype {
            self.write_simple_element(writer, "ftype", &ftype.to_string())?;
        }
        if let Some(ref ftype_str) = vol.ftype_str {
            self.write_simple_element(writer, "ftype_str", ftype_str)?;
        }
        if let Some(block_count) = vol.block_count {
            self.write_simple_element(writer, "block_count", &block_count.to_string())?;
        }
        if let Some(first_block) = vol.first_block {
            self.write_simple_element(writer, "first_block", &first_block.to_string())?;
        }
        if let Some(last_block) = vol.last_block {
            self.write_simple_element(writer, "last_block", &last_block.to_string())?;
        }
        if let Some(allocated_only) = vol.allocated_only {
            self.write_simple_element(
                writer,
                "allocated_only",
                if allocated_only { "1" } else { "0" },
            )?;
        }

        if let Some(ref brs) = vol.byte_runs {
            self.write_byte_runs(writer, brs)?;
        }

        // Write nested volumes
        for nested in vol.volumes() {
            self.write_volume(writer, nested)?;
        }

        // Write files
        for file in vol.files() {
            self.write_file(writer, file)?;
        }

        if let Some(ref error) = vol.error {
            self.write_simple_element(writer, "error", error)?;
        }

        writer.write_event(Event::End(BytesEnd::new("volume")))?;
        Ok(())
    }

    /// Writes a file object.
    fn write_file<W: Write>(&self, writer: &mut Writer<W>, file: &FileObject) -> Result<()> {
        writer.write_event(Event::Start(BytesStart::new("fileobject")))?;

        // Write properties in DFXML schema order
        if let Some(ref filename) = file.filename {
            self.write_simple_element(writer, "filename", filename)?;
        }
        if let Some(ref error) = file.error {
            self.write_simple_element(writer, "error", error)?;
        }
        if let Some(partition) = file.partition {
            self.write_simple_element(writer, "partition", &partition.to_string())?;
        }
        if let Some(id) = file.id {
            self.write_simple_element(writer, "id", &id.to_string())?;
        }
        if let Some(ref name_type) = file.name_type {
            self.write_simple_element(writer, "name_type", name_type.as_str())?;
        }
        if let Some(filesize) = file.filesize {
            self.write_simple_element(writer, "filesize", &filesize.to_string())?;
        }

        // Allocation status
        if file.alloc_inode.is_none() && file.alloc_name.is_none() {
            if let Some(alloc) = file.alloc {
                self.write_simple_element(writer, "alloc", if alloc { "1" } else { "0" })?;
            }
        } else {
            if let Some(alloc_inode) = file.alloc_inode {
                self.write_simple_element(
                    writer,
                    "alloc_inode",
                    if alloc_inode { "1" } else { "0" },
                )?;
            }
            if let Some(alloc_name) = file.alloc_name {
                self.write_simple_element(
                    writer,
                    "alloc_name",
                    if alloc_name { "1" } else { "0" },
                )?;
            }
        }

        if let Some(used) = file.used {
            self.write_simple_element(writer, "used", if used { "1" } else { "0" })?;
        }
        if let Some(orphan) = file.orphan {
            self.write_simple_element(writer, "orphan", if orphan { "1" } else { "0" })?;
        }
        if let Some(compressed) = file.compressed {
            self.write_simple_element(writer, "compressed", if compressed { "1" } else { "0" })?;
        }
        if let Some(inode) = file.inode {
            self.write_simple_element(writer, "inode", &inode.to_string())?;
        }
        if let Some(ref meta_type) = file.meta_type {
            self.write_simple_element(
                writer,
                "meta_type",
                &(crate::objects::MetaType::from_code(match meta_type {
                    crate::objects::MetaType::Regular => 1,
                    crate::objects::MetaType::Directory => 2,
                    crate::objects::MetaType::SymbolicLink => 3,
                    crate::objects::MetaType::BlockDevice => 4,
                    crate::objects::MetaType::CharacterDevice => 5,
                    crate::objects::MetaType::Fifo => 6,
                    crate::objects::MetaType::Socket => 7,
                    crate::objects::MetaType::Shadow => 8,
                    crate::objects::MetaType::Virtual => 9,
                    crate::objects::MetaType::Unknown => 0,
                }) as u8)
                    .to_string(),
            )?;
        }
        if let Some(mode) = file.mode {
            self.write_simple_element(writer, "mode", &format!("{:o}", mode))?;
        }
        if let Some(nlink) = file.nlink {
            self.write_simple_element(writer, "nlink", &nlink.to_string())?;
        }
        if let Some(uid) = file.uid {
            self.write_simple_element(writer, "uid", &uid.to_string())?;
        }
        if let Some(gid) = file.gid {
            self.write_simple_element(writer, "gid", &gid.to_string())?;
        }

        // Timestamps
        if let Some(ref ts) = file.mtime {
            self.write_timestamp(writer, "mtime", ts)?;
        }
        if let Some(ref ts) = file.ctime {
            self.write_timestamp(writer, "ctime", ts)?;
        }
        if let Some(ref ts) = file.atime {
            self.write_timestamp(writer, "atime", ts)?;
        }
        if let Some(ref ts) = file.crtime {
            self.write_timestamp(writer, "crtime", ts)?;
        }
        if let Some(seq) = file.seq {
            self.write_simple_element(writer, "seq", &seq.to_string())?;
        }
        if let Some(ref ts) = file.dtime {
            self.write_timestamp(writer, "dtime", ts)?;
        }
        if let Some(ref ts) = file.bkup_time {
            self.write_timestamp(writer, "bkup_time", ts)?;
        }

        if let Some(ref link_target) = file.link_target {
            self.write_simple_element(writer, "link_target", link_target)?;
        }
        if let Some(ref libmagic) = file.libmagic {
            self.write_simple_element(writer, "libmagic", libmagic)?;
        }

        // Byte runs (with facets if multiple types present)
        let has_multiple_facets = [&file.inode_brs, &file.name_brs, &file.data_brs]
            .iter()
            .filter(|x| x.is_some())
            .count()
            > 1;

        if let Some(ref brs) = file.inode_brs {
            self.write_byte_runs_with_facet(writer, brs, Some(ByteRunFacet::Inode))?;
        }
        if let Some(ref brs) = file.name_brs {
            self.write_byte_runs_with_facet(writer, brs, Some(ByteRunFacet::Name))?;
        }
        if let Some(ref brs) = file.data_brs {
            let facet = if has_multiple_facets {
                Some(ByteRunFacet::Data)
            } else {
                brs.facet
            };
            self.write_byte_runs_with_facet(writer, brs, facet)?;
        }

        // Hashes
        self.write_hashes(writer, &file.hashes)?;

        writer.write_event(Event::End(BytesEnd::new("fileobject")))?;
        Ok(())
    }

    /// Writes a timestamp element.
    fn write_timestamp<W: Write>(
        &self,
        writer: &mut Writer<W>,
        name: &str,
        ts: &Timestamp,
    ) -> Result<()> {
        if let Some(ref time) = ts.time {
            let mut elem = BytesStart::new(name);
            if let Some(ref prec) = ts.prec {
                elem.push_attribute(("prec", prec.to_string().as_str()));
            }
            writer.write_event(Event::Start(elem))?;
            writer.write_event(Event::Text(BytesText::new(&time.to_rfc3339())))?;
            writer.write_event(Event::End(BytesEnd::new(name)))?;
        }
        Ok(())
    }

    /// Writes byte runs.
    fn write_byte_runs<W: Write>(&self, writer: &mut Writer<W>, brs: &ByteRuns) -> Result<()> {
        self.write_byte_runs_with_facet(writer, brs, brs.facet)
    }

    /// Writes byte runs with an optional facet.
    fn write_byte_runs_with_facet<W: Write>(
        &self,
        writer: &mut Writer<W>,
        brs: &ByteRuns,
        facet: Option<ByteRunFacet>,
    ) -> Result<()> {
        if brs.is_empty() {
            return Ok(());
        }

        let mut elem = BytesStart::new("byte_runs");
        if let Some(f) = facet {
            elem.push_attribute(("facet", f.as_str()));
        }
        writer.write_event(Event::Start(elem))?;

        for br in brs.iter() {
            self.write_byte_run(writer, br)?;
        }

        writer.write_event(Event::End(BytesEnd::new("byte_runs")))?;
        Ok(())
    }

    /// Writes a single byte run.
    fn write_byte_run<W: Write>(&self, writer: &mut Writer<W>, br: &ByteRun) -> Result<()> {
        let mut elem = BytesStart::new("byte_run");

        if let Some(offset) = br.img_offset {
            elem.push_attribute(("img_offset", offset.to_string().as_str()));
        }
        if let Some(offset) = br.fs_offset {
            elem.push_attribute(("fs_offset", offset.to_string().as_str()));
        }
        if let Some(offset) = br.file_offset {
            elem.push_attribute(("file_offset", offset.to_string().as_str()));
        }
        if let Some(len) = br.len {
            elem.push_attribute(("len", len.to_string().as_str()));
        }
        if let Some(fill) = br.fill {
            elem.push_attribute(("fill", fill.to_string().as_str()));
        }
        if let Some(ref run_type) = br.run_type {
            elem.push_attribute(("type", run_type.to_string().as_str()));
        }
        if let Some(len) = br.uncompressed_len {
            elem.push_attribute(("uncompressed_len", len.to_string().as_str()));
        }

        // If byte run has hashes, we need child elements
        if br.has_hashes() {
            writer.write_event(Event::Start(elem))?;
            self.write_hashes(writer, &br.hashes)?;
            writer.write_event(Event::End(BytesEnd::new("byte_run")))?;
        } else {
            writer.write_event(Event::Empty(elem))?;
        }

        Ok(())
    }

    /// Writes hash elements.
    fn write_hashes<W: Write>(
        &self,
        writer: &mut Writer<W>,
        hashes: &crate::objects::Hashes,
    ) -> Result<()> {
        // Write hashes in a consistent order
        let hash_order = [
            HashType::Md5,
            HashType::Md6,
            HashType::Sha1,
            HashType::Sha224,
            HashType::Sha256,
            HashType::Sha384,
            HashType::Sha512,
        ];

        for hash_type in hash_order {
            if let Some(value) = hashes.get(hash_type) {
                let mut elem = BytesStart::new("hashdigest");
                elem.push_attribute(("type", hash_type.as_str()));
                writer.write_event(Event::Start(elem))?;
                writer.write_event(Event::Text(BytesText::new(value)))?;
                writer.write_event(Event::End(BytesEnd::new("hashdigest")))?;
            }
        }

        Ok(())
    }

    /// Writes a simple text element.
    fn write_simple_element<W: Write>(
        &self,
        writer: &mut Writer<W>,
        name: &str,
        value: &str,
    ) -> Result<()> {
        writer.write_event(Event::Start(BytesStart::new(name)))?;
        writer.write_event(Event::Text(BytesText::new(value)))?;
        writer.write_event(Event::End(BytesEnd::new(name)))?;
        Ok(())
    }
}

impl Default for DFXMLWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to write a DFXMLObject to a string.
pub fn to_string(doc: &DFXMLObject) -> Result<String> {
    DFXMLWriter::new().write_to_string(doc)
}

/// Convenience function to write a DFXMLObject to a string without indentation.
pub fn to_string_compact(doc: &DFXMLObject) -> Result<String> {
    DFXMLWriter::with_config(WriterConfig::compact()).write_to_string(doc)
}

/// Convenience function to write a DFXMLObject to a writer.
pub fn write<W: Write>(doc: &DFXMLObject, writer: W) -> Result<()> {
    DFXMLWriter::new().write(doc, writer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{ByteRun, ByteRuns, HashType};

    #[test]
    fn test_write_simple_dfxml() {
        let mut doc = DFXMLObject::new();
        doc.program = Some("test-program".to_string());
        doc.program_version = Some("1.0.0".to_string());

        let xml = to_string(&doc).unwrap();

        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<dfxml"));
        assert!(xml.contains("xmlns="));
        assert!(xml.contains("<program>test-program</program>"));
        assert!(xml.contains("<version>1.0.0</version>"));
        assert!(xml.contains("</dfxml>"));
    }

    #[test]
    fn test_write_with_volume_and_file() {
        let mut doc = DFXMLObject::new();
        doc.program = Some("test".to_string());

        let mut vol = VolumeObject::with_ftype("ntfs");
        vol.block_size = Some(4096);

        let mut file = FileObject::with_filename("test.txt");
        file.filesize = Some(1024);
        file.hashes.set(
            HashType::Md5,
            "d41d8cd98f00b204e9800998ecf8427e".to_string(),
        );

        let mut brs = ByteRuns::new();
        brs.push(ByteRun::with_img_offset(0, 512));
        brs.push(ByteRun::with_img_offset(512, 512));
        file.data_brs = Some(brs);

        vol.append_file(file);
        doc.append_volume(vol);

        let xml = to_string(&doc).unwrap();

        assert!(xml.contains("<volume>"));
        assert!(xml.contains("<ftype_str>ntfs</ftype_str>"));
        assert!(xml.contains("<block_size>4096</block_size>"));
        assert!(xml.contains("<fileobject>"));
        assert!(xml.contains("<filename>test.txt</filename>"));
        assert!(xml.contains("<filesize>1024</filesize>"));
        assert!(xml.contains("<hashdigest type=\"md5\">"));
        assert!(xml.contains("<byte_runs>"));
        assert!(xml.contains("<byte_run"));
        assert!(xml.contains("img_offset=\"0\""));
        assert!(xml.contains("len=\"512\""));
    }

    #[test]
    fn test_write_compact() {
        let mut doc = DFXMLObject::new();
        doc.program = Some("test".to_string());

        let xml = to_string_compact(&doc).unwrap();

        // Compact should have no newlines in the content (except possibly in the declaration)
        let content_start = xml.find("<dfxml").unwrap();
        let content = &xml[content_start..];
        assert!(!content.contains('\n') || content.matches('\n').count() <= 1);
    }

    #[test]
    fn test_write_timestamps() {
        use chrono::{TimeZone, Utc};

        let mut doc = DFXMLObject::new();

        let mut file = FileObject::with_filename("test.txt");
        let time = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
        file.mtime = Some(Timestamp {
            name: Some(crate::objects::TimestampName::Mtime),
            time: Some(time.fixed_offset()),
            prec: None,
        });

        doc.append_file(file);

        let xml = to_string(&doc).unwrap();

        assert!(xml.contains("<mtime>"));
        assert!(xml.contains("2024-01-15"));
        assert!(xml.contains("</mtime>"));
    }

    #[test]
    fn test_roundtrip() {
        // Create a document
        let mut doc = DFXMLObject::new();
        doc.program = Some("roundtrip-test".to_string());
        doc.program_version = Some("1.0".to_string());

        let mut vol = VolumeObject::with_ftype("ext4");
        vol.block_size = Some(4096);

        let mut file = FileObject::with_filename("/home/user/test.txt");
        file.filesize = Some(2048);
        file.inode = Some(12345);
        file.hashes.set(HashType::Sha256, "abcd1234".to_string());

        vol.append_file(file);
        doc.append_volume(vol);

        // Write to string
        let xml = to_string(&doc).unwrap();

        // Parse back
        use std::io::Cursor;
        let parsed = crate::reader::parse(Cursor::new(xml.as_bytes())).unwrap();

        // Verify
        assert_eq!(parsed.program, Some("roundtrip-test".to_string()));
        assert_eq!(parsed.program_version, Some("1.0".to_string()));
        assert_eq!(parsed.volume_count(), 1);

        let files: Vec<_> = parsed.iter_files().collect();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, Some("/home/user/test.txt".to_string()));
        assert_eq!(files[0].filesize, Some(2048));
        assert_eq!(files[0].inode, Some(12345));
    }
}
