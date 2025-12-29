//! DFXML streaming reader.
//!
//! This module provides a memory-efficient streaming parser for DFXML files.
//! It uses `quick-xml` for XML parsing and yields objects as they are parsed.
//!
//! # Example
//!
//! ```rust,no_run
//! use dfxml::reader::{DFXMLReader, Event};
//! use std::fs::File;
//! use std::io::BufReader;
//!
//! let file = File::open("forensic_output.xml").unwrap();
//! let reader = DFXMLReader::from_reader(BufReader::new(file));
//!
//! for result in reader {
//!     match result {
//!         Ok(Event::FileObject(file)) => {
//!             println!("File: {:?}", file.filename);
//!         }
//!         Ok(Event::VolumeStart(vol)) => {
//!             println!("Volume: {:?}", vol.ftype_str);
//!         }
//!         Ok(_) => {}
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! ```

use crate::error::{Error, Result};
use crate::objects::{
    ByteRun, ByteRunFacet, ByteRuns, DFXMLObject, DiskImageObject, FileObject, HashType,
    LibraryObject, PartitionObject, PartitionSystemObject, Timestamp, TimestampName,
    VolumeObject,
};
use quick_xml::events::BytesStart;
use quick_xml::events::Event as XmlEvent;
use quick_xml::Reader;
use std::io::BufRead;
use std::str;

/// Events emitted by the DFXML reader.
///
/// The reader emits start events when container objects (DFXMLObject, VolumeObject, etc.)
/// are opened, and end events when they are closed. FileObjects are emitted as complete
/// objects when their closing tag is encountered.
#[derive(Debug)]
pub enum Event {
    /// Start of the DFXML document (metadata may not be fully populated yet)
    DFXMLStart(DFXMLObject),
    /// End of the DFXML document (contains the completed document with all metadata)
    DFXMLEnd(DFXMLObject),
    /// Start of a disk image
    DiskImageStart(DiskImageObject),
    /// End of a disk image
    DiskImageEnd,
    /// Start of a partition system
    PartitionSystemStart(PartitionSystemObject),
    /// End of a partition system
    PartitionSystemEnd,
    /// Start of a partition
    PartitionStart(PartitionObject),
    /// End of a partition
    PartitionEnd,
    /// Start of a volume
    VolumeStart(VolumeObject),
    /// End of a volume
    VolumeEnd,
    /// A complete file object
    FileObject(FileObject),
}

/// Parser state tracking what container we're currently inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserState {
    /// Initial state, before seeing <dfxml>
    Initial,
    /// Inside <dfxml>, processing metadata
    InDfxml,
    /// Inside <diskimageobject>
    InDiskImage,
    /// Inside <partitionsystemobject>
    InPartitionSystem,
    /// Inside <partitionobject>
    InPartition,
    /// Inside <volume>
    InVolume,
    /// Inside <fileobject>
    InFileObject,
    /// Inside <creator>
    InCreator,
    /// Inside <build_environment>
    InBuildEnvironment,
    /// Finished parsing
    Done,
}

/// Context for tracking nested element parsing.
#[derive(Debug, Default)]
struct ElementContext {
    /// Current element path (e.g., ["dfxml", "creator", "library"])
    path: Vec<String>,
    /// Accumulated text content
    text: String,
    /// Current attributes (for elements that need them)
    attrs: Vec<(String, String)>,
}

impl ElementContext {
    fn push(&mut self, name: String) {
        self.path.push(name);
        self.text.clear();
        self.attrs.clear();
    }

    fn pop(&mut self) -> Option<String> {
        self.text.clear();
        self.attrs.clear();
        self.path.pop()
    }

    /// Returns the current element name (for error reporting).
    #[allow(dead_code)]
    fn current(&self) -> Option<&str> {
        self.path.last().map(|s| s.as_str())
    }

    /// Returns the current nesting depth (for debugging).
    #[allow(dead_code)]
    fn depth(&self) -> usize {
        self.path.len()
    }
}

/// Intermediate parsed event data (owned, to avoid borrow conflicts).
enum ParsedEvent {
    Start { name: String, attrs: Vec<(String, String)> },
    End { name: String },
    Empty { name: String, attrs: Vec<(String, String)> },
    Text { text: String },
    Eof,
}

/// A streaming DFXML parser.
///
/// Reads DFXML from any `BufRead` source and yields [`Event`]s as objects
/// are parsed. This is memory-efficient for large DFXML files since it
/// doesn't load the entire document into memory.
pub struct DFXMLReader<R: BufRead> {
    reader: Reader<R>,
    state: ParserState,
    state_stack: Vec<ParserState>,
    buf: Vec<u8>,
    context: ElementContext,

    // Objects being built
    dfxml: Option<DFXMLObject>,
    disk_image: Option<DiskImageObject>,
    partition_system: Option<PartitionSystemObject>,
    partition: Option<PartitionObject>,
    volume: Option<VolumeObject>,
    file: Option<FileObject>,

    // Nested object building
    byte_runs: Option<ByteRuns>,
    current_byte_run: Option<ByteRun>,
    current_timestamp: Option<(TimestampName, Timestamp)>,
    current_library: Option<LibraryObject>,

    // Track if we're in specific sub-elements
    in_byte_runs: bool,
    byte_runs_facet: Option<ByteRunFacet>,

    // Pending events to yield
    pending_events: Vec<Event>,
}

impl<R: BufRead> DFXMLReader<R> {
    /// Creates a new DFXML reader from a buffered reader.
    pub fn from_reader(reader: R) -> Self {
        let mut xml_reader = Reader::from_reader(reader);
        xml_reader.config_mut().trim_text(true);

        Self {
            reader: xml_reader,
            state: ParserState::Initial,
            state_stack: Vec::new(),
            buf: Vec::with_capacity(4096),
            context: ElementContext::default(),
            dfxml: None,
            disk_image: None,
            partition_system: None,
            partition: None,
            volume: None,
            file: None,
            byte_runs: None,
            current_byte_run: None,
            current_timestamp: None,
            current_library: None,
            in_byte_runs: false,
            byte_runs_facet: None,
            pending_events: Vec::new(),
        }
    }

    /// Parses the next event from the DFXML stream.
    fn parse_next(&mut self) -> Result<Option<Event>> {
        // Return any pending events first
        if let Some(event) = self.pending_events.pop() {
            return Ok(Some(event));
        }

        loop {
            self.buf.clear();
            
            // Read the event and immediately extract what we need as owned data
            let event_data = {
                let event = self.reader.read_event_into(&mut self.buf)?;
                match event {
                    XmlEvent::Start(ref e) => {
                        let local_name = e.local_name();
                        let name = str::from_utf8(local_name.as_ref())?.to_string();
                        let attrs = Self::extract_attrs(e)?;
                        Some(ParsedEvent::Start { name, attrs })
                    }
                    XmlEvent::End(ref e) => {
                        let local_name = e.local_name();
                        let name = str::from_utf8(local_name.as_ref())?.to_string();
                        Some(ParsedEvent::End { name })
                    }
                    XmlEvent::Empty(ref e) => {
                        let local_name = e.local_name();
                        let name = str::from_utf8(local_name.as_ref())?.to_string();
                        let attrs = Self::extract_attrs(e)?;
                        Some(ParsedEvent::Empty { name, attrs })
                    }
                    XmlEvent::Text(ref e) => {
                        let text = e.unescape()?.to_string();
                        Some(ParsedEvent::Text { text })
                    }
                    XmlEvent::CData(ref e) => {
                        let text = str::from_utf8(e.as_ref())?.to_string();
                        Some(ParsedEvent::Text { text })
                    }
                    XmlEvent::Eof => {
                        Some(ParsedEvent::Eof)
                    }
                    _ => None,
                }
            };

            // Now process the extracted data without borrowing self.buf
            match event_data {
                Some(ParsedEvent::Start { name, attrs }) => {
                    if let Some(ev) = self.handle_start_owned(&name, attrs)? {
                        return Ok(Some(ev));
                    }
                }
                Some(ParsedEvent::End { name }) => {
                    if let Some(ev) = self.handle_end_owned(&name)? {
                        return Ok(Some(ev));
                    }
                }
                Some(ParsedEvent::Empty { name, attrs }) => {
                    // Handle self-closing tags like <byte_run ... />
                    if let Some(ev) = self.handle_start_owned(&name, attrs)? {
                        self.pending_events.push(ev);
                    }
                    if let Some(ev) = self.handle_end_owned(&name)? {
                        return Ok(Some(ev));
                    }
                    if let Some(ev) = self.pending_events.pop() {
                        return Ok(Some(ev));
                    }
                }
                Some(ParsedEvent::Text { text }) => {
                    self.context.text.push_str(&text);
                }
                Some(ParsedEvent::Eof) => {
                    self.state = ParserState::Done;
                    return Ok(None);
                }
                None => {}
            }
        }
    }

    /// Extracts attributes from a BytesStart element as owned data.
    fn extract_attrs(e: &BytesStart<'_>) -> Result<Vec<(String, String)>> {
        let mut attrs = Vec::new();
        for attr in e.attributes().flatten() {
            let key = str::from_utf8(attr.key.as_ref())?.to_string();
            let value = attr.unescape_value()?.to_string();
            attrs.push((key, value));
        }
        Ok(attrs)
    }

    /// Handles a start element event with owned data.
    fn handle_start_owned(&mut self, local_name: &str, attrs: Vec<(String, String)>) -> Result<Option<Event>> {
        self.context.push(local_name.to_string());
        self.context.attrs = attrs;

        match local_name {
            "dfxml" => {
                let mut dfxml = DFXMLObject::new();
                // Parse version attribute
                for (key, value) in &self.context.attrs {
                    if key == "version" {
                        dfxml.version = value.clone();
                    }
                }
                self.dfxml = Some(dfxml.clone());
                self.state = ParserState::InDfxml;
                return Ok(Some(Event::DFXMLStart(dfxml)));
            }
            "diskimageobject" => {
                self.state_stack.push(self.state);
                self.state = ParserState::InDiskImage;
                self.disk_image = Some(DiskImageObject::new());
            }
            "partitionsystemobject" => {
                self.state_stack.push(self.state);
                self.state = ParserState::InPartitionSystem;
                self.partition_system = Some(PartitionSystemObject::new());
            }
            "partitionobject" => {
                self.state_stack.push(self.state);
                self.state = ParserState::InPartition;
                self.partition = Some(PartitionObject::new());
            }
            "volume" => {
                self.state_stack.push(self.state);
                self.state = ParserState::InVolume;
                self.volume = Some(VolumeObject::new());
            }
            "fileobject" => {
                self.state_stack.push(self.state);
                self.state = ParserState::InFileObject;
                self.file = Some(FileObject::new());
            }
            "creator" => {
                self.state_stack.push(self.state);
                self.state = ParserState::InCreator;
            }
            "build_environment" => {
                self.state_stack.push(self.state);
                self.state = ParserState::InBuildEnvironment;
            }
            "byte_runs" => {
                self.in_byte_runs = true;
                self.byte_runs = Some(ByteRuns::new());
                // Check for facet attribute
                for (key, value) in &self.context.attrs {
                    if key == "facet" {
                        self.byte_runs_facet = value.parse().ok();
                        if let Some(ref mut brs) = self.byte_runs {
                            brs.facet = self.byte_runs_facet;
                        }
                    }
                }
            }
            "byte_run" => {
                let mut br = ByteRun::new();
                self.parse_byte_run_attrs(&self.context.attrs.clone(), &mut br)?;
                self.current_byte_run = Some(br);
            }
            "library" => {
                let mut lib = LibraryObject::empty();
                for (key, value) in &self.context.attrs {
                    match key.as_str() {
                        "name" => lib.name = Some(value.clone()),
                        "version" => lib.version = Some(value.clone()),
                        _ => {}
                    }
                }
                self.current_library = Some(lib);
            }
            "mtime" | "atime" | "ctime" | "crtime" | "dtime" | "bkup_time" => {
                let name: TimestampName = local_name.parse()?;
                let mut ts = Timestamp::new();
                ts.name = Some(name);
                // Parse prec attribute if present
                for (key, value) in &self.context.attrs {
                    if key == "prec" {
                        ts.prec = value.parse().ok();
                    }
                }
                self.current_timestamp = Some((name, ts));
            }
            _ => {}
        }

        Ok(None)
    }

    /// Handles an end element event with owned data.
    fn handle_end_owned(&mut self, local_name: &str) -> Result<Option<Event>> {
        let text = self.context.text.trim().to_string();
        let attrs = self.context.attrs.clone();
        self.context.pop();

        match local_name {
            "dfxml" => {
                self.state = ParserState::Done;
                // Return the completed DFXMLObject with all parsed metadata
                let completed = self.dfxml.take().unwrap_or_else(DFXMLObject::new);
                return Ok(Some(Event::DFXMLEnd(completed)));
            }
            "diskimageobject" => {
                self.state = self.state_stack.pop().unwrap_or(ParserState::InDfxml);
                if let Some(di) = self.disk_image.take() {
                    return Ok(Some(Event::DiskImageStart(di)));
                }
            }
            "partitionsystemobject" => {
                self.state = self.state_stack.pop().unwrap_or(ParserState::InDfxml);
                if let Some(ps) = self.partition_system.take() {
                    return Ok(Some(Event::PartitionSystemStart(ps)));
                }
            }
            "partitionobject" => {
                self.state = self.state_stack.pop().unwrap_or(ParserState::InDfxml);
                if let Some(p) = self.partition.take() {
                    return Ok(Some(Event::PartitionStart(p)));
                }
            }
            "volume" => {
                self.state = self.state_stack.pop().unwrap_or(ParserState::InDfxml);
                if let Some(vol) = self.volume.take() {
                    return Ok(Some(Event::VolumeStart(vol)));
                }
            }
            "fileobject" => {
                self.state = self.state_stack.pop().unwrap_or(ParserState::InDfxml);
                if let Some(file) = self.file.take() {
                    return Ok(Some(Event::FileObject(file)));
                }
            }
            "creator" => {
                self.state = self.state_stack.pop().unwrap_or(ParserState::InDfxml);
            }
            "build_environment" => {
                self.state = self.state_stack.pop().unwrap_or(ParserState::InDfxml);
            }
            "byte_runs" => {
                self.in_byte_runs = false;
                if let Some(brs) = self.byte_runs.take() {
                    self.apply_byte_runs(brs);
                }
                self.byte_runs_facet = None;
            }
            "byte_run" => {
                if let Some(br) = self.current_byte_run.take() {
                    if let Some(ref mut brs) = self.byte_runs {
                        brs.push(br);
                    }
                }
            }
            "library" => {
                if let Some(lib) = self.current_library.take() {
                    match self.state {
                        ParserState::InCreator => {
                            if let Some(ref mut dfxml) = self.dfxml {
                                dfxml.add_creator_library(lib);
                            }
                        }
                        ParserState::InBuildEnvironment => {
                            if let Some(ref mut dfxml) = self.dfxml {
                                dfxml.add_build_library(lib);
                            }
                        }
                        _ => {}
                    }
                }
            }
            "mtime" | "atime" | "ctime" | "crtime" | "dtime" | "bkup_time" => {
                if let Some((name, mut ts)) = self.current_timestamp.take() {
                    if !text.is_empty() {
                        ts.time = Timestamp::parse_iso8601(&text).ok();
                    }
                    if let Some(ref mut file) = self.file {
                        file.set_timestamp(name, ts);
                    }
                }
            }
            "hashdigest" => {
                // Get hash type from the stored attributes
                let hash_type_str = attrs
                    .iter()
                    .find(|(k, _)| k == "type")
                    .map(|(_, v)| v.as_str());

                if let Some(type_str) = hash_type_str {
                    if let Ok(hash_type) = type_str.parse::<HashType>() {
                        // Apply to current byte_run or file
                        if let Some(ref mut br) = self.current_byte_run {
                            br.hashes.set(hash_type, text);
                        } else if let Some(ref mut file) = self.file {
                            file.hashes.set(hash_type, text);
                        }
                    }
                }
            }
            // File object simple properties
            "filename" => {
                if let Some(ref mut file) = self.file {
                    file.filename = Some(text);
                }
            }
            "filesize" => {
                if let Some(ref mut file) = self.file {
                    file.filesize = text.parse().ok();
                }
            }
            "inode" => {
                if let Some(ref mut file) = self.file {
                    file.inode = text.parse().ok();
                }
            }
            "partition" => {
                if let Some(ref mut file) = self.file {
                    file.partition = text.parse().ok();
                }
            }
            "id" => {
                if let Some(ref mut file) = self.file {
                    file.id = text.parse().ok();
                }
            }
            "name_type" => {
                if let Some(ref mut file) = self.file {
                    file.name_type = text.parse().ok();
                }
            }
            "meta_type" => {
                if let Some(ref mut file) = self.file {
                    if let Ok(code) = text.parse::<i32>() {
                        file.meta_type = Some(crate::objects::MetaType::from_code(code));
                    }
                }
            }
            "mode" => {
                if let Some(ref mut file) = self.file {
                    // Parse octal or decimal mode
                    file.mode = if text.starts_with('0') {
                        u32::from_str_radix(text.trim_start_matches('0'), 8).ok()
                    } else {
                        text.parse().ok()
                    };
                }
            }
            "nlink" => {
                if let Some(ref mut file) = self.file {
                    file.nlink = text.parse().ok();
                }
            }
            "uid" => {
                if let Some(ref mut file) = self.file {
                    file.uid = text.parse().ok();
                }
            }
            "gid" => {
                if let Some(ref mut file) = self.file {
                    file.gid = text.parse().ok();
                }
            }
            "link_target" => {
                if let Some(ref mut file) = self.file {
                    file.link_target = Some(text);
                }
            }
            "libmagic" => {
                if let Some(ref mut file) = self.file {
                    file.libmagic = Some(text);
                }
            }
            "seq" => {
                if let Some(ref mut file) = self.file {
                    file.seq = text.parse().ok();
                }
            }
            "alloc" => {
                if let Some(ref mut file) = self.file {
                    file.alloc = parse_bool(&text);
                }
            }
            "alloc_inode" => {
                if let Some(ref mut file) = self.file {
                    file.alloc_inode = parse_bool(&text);
                }
            }
            "alloc_name" => {
                if let Some(ref mut file) = self.file {
                    file.alloc_name = parse_bool(&text);
                }
            }
            "orphan" => {
                if let Some(ref mut file) = self.file {
                    file.orphan = parse_bool(&text);
                }
            }
            "compressed" => {
                if let Some(ref mut file) = self.file {
                    file.compressed = parse_bool(&text);
                }
            }
            "error" => {
                match self.state {
                    ParserState::InFileObject => {
                        if let Some(ref mut file) = self.file {
                            file.error = Some(text);
                        }
                    }
                    ParserState::InVolume => {
                        if let Some(ref mut vol) = self.volume {
                            vol.error = Some(text);
                        }
                    }
                    ParserState::InPartitionSystem => {
                        if let Some(ref mut ps) = self.partition_system {
                            ps.error = Some(text);
                        }
                    }
                    ParserState::InDiskImage => {
                        if let Some(ref mut di) = self.disk_image {
                            di.error = Some(text);
                        }
                    }
                    _ => {}
                }
            }
            // Volume properties
            "ftype_str" => {
                if let Some(ref mut vol) = self.volume {
                    vol.ftype_str = Some(text);
                }
            }
            "ftype" => {
                if let Some(ref mut vol) = self.volume {
                    vol.ftype = text.parse().ok();
                }
            }
            "block_size" => {
                if let Some(ref mut vol) = self.volume {
                    vol.block_size = text.parse().ok();
                }
            }
            "block_count" => {
                if let Some(ref mut vol) = self.volume {
                    vol.block_count = text.parse().ok();
                }
            }
            "first_block" => {
                if let Some(ref mut vol) = self.volume {
                    vol.first_block = text.parse().ok();
                }
            }
            "last_block" => {
                if let Some(ref mut vol) = self.volume {
                    vol.last_block = text.parse().ok();
                }
            }
            "partition_offset" => {
                if let Some(ref mut vol) = self.volume {
                    vol.partition_offset = text.parse().ok();
                }
            }
            "sector_size" => {
                if let Some(ref mut vol) = self.volume {
                    vol.sector_size = text.parse().ok();
                }
            }
            "allocated_only" => {
                if let Some(ref mut vol) = self.volume {
                    vol.allocated_only = parse_bool(&text);
                }
            }
            // DFXML metadata
            "program" => {
                if let Some(ref mut dfxml) = self.dfxml {
                    dfxml.program = Some(text);
                }
            }
            "version" if self.state == ParserState::InCreator => {
                if let Some(ref mut dfxml) = self.dfxml {
                    dfxml.program_version = Some(text);
                }
            }
            "command_line" => {
                if let Some(ref mut dfxml) = self.dfxml {
                    dfxml.command_line = Some(text);
                }
            }
            "image_filename" => {
                if let Some(ref mut dfxml) = self.dfxml {
                    dfxml.sources.push(text);
                }
            }
            // Partition system properties
            "pstype_str" => {
                if let Some(ref mut ps) = self.partition_system {
                    ps.pstype_str = Some(text);
                }
            }
            // Partition properties
            "ptype" => {
                if let Some(ref mut p) = self.partition {
                    p.ptype = text.parse().ok();
                }
            }
            "ptype_str" => {
                if let Some(ref mut p) = self.partition {
                    p.ptype_str = Some(text);
                }
            }
            "partition_index" => {
                if let Some(ref mut p) = self.partition {
                    p.partition_index = text.parse().ok();
                }
            }
            // Disk image properties
            "imagesize" => {
                if let Some(ref mut di) = self.disk_image {
                    di.image_size = text.parse().ok();
                }
            }
            _ => {}
        }

        Ok(None)
    }

    /// Parses byte_run element attributes into a ByteRun struct.
    fn parse_byte_run_attrs(&self, attrs: &[(String, String)], br: &mut ByteRun) -> Result<()> {
        for (key, value) in attrs {
            match key.as_str() {
                "img_offset" => br.img_offset = value.parse().ok(),
                "fs_offset" => br.fs_offset = value.parse().ok(),
                "file_offset" => br.file_offset = value.parse().ok(),
                "len" => br.len = value.parse().ok(),
                "fill" => {
                    if let Ok(v) = value.parse::<u8>() {
                        br.fill = Some(v);
                    }
                }
                "type" => br.run_type = value.parse().ok(),
                "uncompressed_len" => br.uncompressed_len = value.parse().ok(),
                _ => {}
            }
        }
        Ok(())
    }

    /// Applies parsed byte runs to the current object.
    fn apply_byte_runs(&mut self, brs: ByteRuns) {
        match self.state {
            ParserState::InFileObject => {
                if let Some(ref mut file) = self.file {
                    match self.byte_runs_facet {
                        Some(ByteRunFacet::Inode) => file.inode_brs = Some(brs),
                        Some(ByteRunFacet::Name) => file.name_brs = Some(brs),
                        _ => file.data_brs = Some(brs),
                    }
                }
            }
            ParserState::InVolume => {
                if let Some(ref mut vol) = self.volume {
                    vol.byte_runs = Some(brs);
                }
            }
            ParserState::InPartition => {
                if let Some(ref mut p) = self.partition {
                    p.byte_runs = Some(brs);
                }
            }
            ParserState::InPartitionSystem => {
                if let Some(ref mut ps) = self.partition_system {
                    ps.byte_runs = Some(brs);
                }
            }
            ParserState::InDiskImage => {
                if let Some(ref mut di) = self.disk_image {
                    di.byte_runs = Some(brs);
                }
            }
            _ => {}
        }
    }
}

impl<R: BufRead> Iterator for DFXMLReader<R> {
    type Item = Result<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.state == ParserState::Done {
            return None;
        }

        match self.parse_next() {
            Ok(Some(event)) => Some(Ok(event)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

/// Parses a boolean value from a string.
///
/// Accepts "1", "0", "true", "false" (case-insensitive).
fn parse_bool(s: &str) -> Option<bool> {
    match s.to_lowercase().as_str() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

/// Convenience function to parse a DFXML file and collect all file objects.
///
/// This loads all files into memory, so it's not suitable for very large
/// DFXML files. For large files, use [`DFXMLReader`] directly.
pub fn parse_file_objects<R: BufRead>(reader: R) -> Result<Vec<FileObject>> {
    let mut files = Vec::new();
    for event in DFXMLReader::from_reader(reader) {
        if let Event::FileObject(file) = event? {
            files.push(file);
        }
    }
    Ok(files)
}

/// Convenience function to parse a complete DFXML document.
///
/// Returns the DFXMLObject with all child objects attached.
/// This loads the entire document into memory.
pub fn parse<R: BufRead>(reader: R) -> Result<DFXMLObject> {
    let mut dfxml: Option<DFXMLObject> = None;
    let mut volume_stack: Vec<VolumeObject> = Vec::new();
    let mut partition_stack: Vec<PartitionObject> = Vec::new();
    let mut partition_system_stack: Vec<PartitionSystemObject> = Vec::new();
    let mut disk_image_stack: Vec<DiskImageObject> = Vec::new();

    for event in DFXMLReader::from_reader(reader) {
        match event? {
            Event::DFXMLStart(d) => {
                // Use DFXMLStart to initialize the object so children can be attached
                dfxml = Some(d);
            }
            Event::DFXMLEnd(d) => {
                // Merge metadata from DFXMLEnd (which has all parsed creator info)
                // into our existing dfxml that has the children attached
                if let Some(ref mut existing) = dfxml {
                    existing.program = d.program;
                    existing.program_version = d.program_version;
                    existing.command_line = d.command_line;
                    existing.sources = d.sources;
                    // Copy creator and build libraries
                    for lib in d.creator_libraries() {
                        existing.add_creator_library(lib.clone());
                    }
                    for lib in d.build_libraries() {
                        existing.add_build_library(lib.clone());
                    }
                }
            }
            Event::DiskImageStart(di) => {
                disk_image_stack.push(di);
            }
            Event::DiskImageEnd => {
                if let Some(di) = disk_image_stack.pop() {
                    if let Some(ref mut d) = dfxml {
                        d.append_disk_image(di);
                    }
                }
            }
            Event::PartitionSystemStart(ps) => {
                partition_system_stack.push(ps);
            }
            Event::PartitionSystemEnd => {
                if let Some(ps) = partition_system_stack.pop() {
                    if let Some(di) = disk_image_stack.last_mut() {
                        di.append_partition_system(ps);
                    } else if let Some(ref mut d) = dfxml {
                        d.append_partition_system(ps);
                    }
                }
            }
            Event::PartitionStart(p) => {
                partition_stack.push(p);
            }
            Event::PartitionEnd => {
                if let Some(p) = partition_stack.pop() {
                    if let Some(ps) = partition_system_stack.last_mut() {
                        ps.append_partition(p);
                    } else if let Some(ref mut d) = dfxml {
                        d.append_partition(p);
                    }
                }
            }
            Event::VolumeStart(v) => {
                volume_stack.push(v);
            }
            Event::VolumeEnd => {
                if let Some(v) = volume_stack.pop() {
                    if let Some(p) = partition_stack.last_mut() {
                        p.append_volume(v);
                    } else if let Some(di) = disk_image_stack.last_mut() {
                        di.append_volume(v);
                    } else if let Some(ref mut d) = dfxml {
                        d.append_volume(v);
                    }
                }
            }
            Event::FileObject(f) => {
                if let Some(v) = volume_stack.last_mut() {
                    v.append_file(f);
                } else if let Some(p) = partition_stack.last_mut() {
                    p.append_file(f);
                } else if let Some(ps) = partition_system_stack.last_mut() {
                    ps.append_file(f);
                } else if let Some(di) = disk_image_stack.last_mut() {
                    di.append_file(f);
                } else if let Some(ref mut d) = dfxml {
                    d.append_file(f);
                }
            }
        }
    }

    dfxml.ok_or_else(|| Error::MissingField("dfxml root element".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const SIMPLE_DFXML: &str = r#"<?xml version="1.0"?>
<dfxml version="1.0">
  <creator>
    <program>test</program>
    <version>1.0</version>
  </creator>
  <volume>
    <ftype_str>ntfs</ftype_str>
    <block_size>4096</block_size>
    <fileobject>
      <filename>test.txt</filename>
      <filesize>1024</filesize>
      <mtime>2024-01-15T10:30:00Z</mtime>
      <hashdigest type="md5">d41d8cd98f00b204e9800998ecf8427e</hashdigest>
      <byte_runs>
        <byte_run img_offset="1024" len="512"/>
        <byte_run img_offset="2048" len="512"/>
      </byte_runs>
    </fileobject>
  </volume>
</dfxml>"#;

    #[test]
    fn test_parse_simple_dfxml() {
        let cursor = Cursor::new(SIMPLE_DFXML);
        let dfxml = parse(cursor).unwrap();

        assert_eq!(dfxml.version, "1.0");
        assert_eq!(dfxml.program, Some("test".to_string()));
        assert_eq!(dfxml.program_version, Some("1.0".to_string()));
        assert_eq!(dfxml.volume_count(), 1);
    }

    #[test]
    fn test_parse_file_objects() {
        let cursor = Cursor::new(SIMPLE_DFXML);
        let files = parse_file_objects(cursor).unwrap();

        assert_eq!(files.len(), 1);
        let file = &files[0];
        assert_eq!(file.filename, Some("test.txt".to_string()));
        assert_eq!(file.filesize, Some(1024));
        assert!(file.mtime.is_some());
        assert!(file.hashes.md5.is_some());

        let brs = file.byte_runs().unwrap();
        assert_eq!(brs.len(), 2);
        assert_eq!(brs[0].img_offset, Some(1024));
        assert_eq!(brs[0].len, Some(512));
    }

    #[test]
    fn test_streaming_reader() {
        let cursor = Cursor::new(SIMPLE_DFXML);
        let reader = DFXMLReader::from_reader(cursor);

        let events: Vec<_> = reader.collect::<Result<Vec<_>>>().unwrap();

        // Should have: DFXMLStart, VolumeStart, FileObject, VolumeEnd(?), DFXMLEnd
        assert!(events.iter().any(|e| matches!(e, Event::DFXMLStart(_))));
        assert!(events.iter().any(|e| matches!(e, Event::VolumeStart(_))));
        assert!(events.iter().any(|e| matches!(e, Event::FileObject(_))));
        assert!(events.iter().any(|e| matches!(e, Event::DFXMLEnd(_))));
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse_bool("1"), Some(true));
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("TRUE"), Some(true));
        assert_eq!(parse_bool("invalid"), None);
    }
}
