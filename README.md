# dfxml-rs

[![GitHub issues](https://img.shields.io/github/issues/kamwoods/dfxml-rs.svg)](https://github.com/kamwoods/dfxml-rs/issues)
[![Build](https://github.com/kamwoods/dfxml-rs/actions/workflows/build.yml/badge.svg)](https://github.com/kamwoods/dfxml-rs/actions/workflows/build.yml)
[![GitHub forks](https://img.shields.io/github/forks/kamwoods/dfxml-rs.svg)](https://github.com/kamwoods/dfxml-rs/network)

## Digital Forensics XML (DFXML) Library for Rust

A Rust library for reading, writing, and manipulating Digital Forensics XML (DFXML) files. DFXML is a standardized format for representing digital forensic metadata, commonly used in disk imaging, file system analysis, and digital evidence processing.

This library provides a complete object model mirroring the [Python DFXML library](https://github.com/dfxml-working-group/dfxml_python), with streaming XML parsing for memory-efficient processing of large forensic datasets.

This is an early WIP prototype. Exercise caution when using.

## Features

- **Complete Object Model**: Full representation of DFXML elements including files, volumes, disk images, partitions, and metadata
- **Streaming Reader**: Memory-efficient parsing using `quick-xml` — process millions of file entries without loading everything into memory
- **XML Writer**: Generate valid DFXML output with proper namespace handling
- **Round-trip Support**: Parse DFXML, modify objects, and write back to XML
- **Optional Serde Support**: Enable the `serde` feature for serialization/deserialization

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
dfxml-rs = { path = "path/to/dfxml-rs" }

# Optional: enable serde support
# dfxml-rs = { path = "path/to/dfxml-rs", features = ["serde"] }
```

## Quick Start

### Parsing a DFXML File

```rust
use dfxml_rs::{parse, DFXMLObject};
use std::fs::File;
use std::io::BufReader;

fn main() -> dfxml_rs::Result<()> {
    let file = File::open("forensic_output.xml")?;
    let dfxml = parse(BufReader::new(file))?;

    println!("Program: {:?}", dfxml.program);
    println!("Volumes: {}", dfxml.volume_count());

    for file in dfxml.iter_files() {
        println!("  {} ({:?} bytes)", 
            file.filename.as_deref().unwrap_or("<unnamed>"),
            file.filesize);
    }

    Ok(())
}
```

### Streaming Large Files

For very large DFXML files, use the streaming API to avoid loading everything into memory:

```rust
use dfxml_rs::{DFXMLReader, Event};
use std::fs::File;
use std::io::BufReader;

fn main() -> dfxml_rs::Result<()> {
    let file = File::open("large_forensic_output.xml")?;
    let reader = DFXMLReader::from_reader(BufReader::new(file));

    let mut file_count = 0;
    let mut total_size = 0u64;

    for result in reader {
        match result? {
            Event::FileObject(file) => {
                file_count += 1;
                total_size += file.filesize.unwrap_or(0);
            }
            Event::VolumeStart(vol) => {
                println!("Processing volume: {:?}", vol.ftype_str);
            }
            _ => {}
        }
    }

    println!("Processed {} files, {} bytes total", file_count, total_size);
    Ok(())
}
```

### Creating DFXML Output

```rust
use dfxml_rs::{DFXMLObject, VolumeObject, FileObject, HashType, to_string};

fn main() -> dfxml_rs::Result<()> {
    let mut doc = DFXMLObject::new();
    doc.program = Some("my-forensic-tool".to_string());
    doc.program_version = Some("1.0.0".to_string());

    let mut volume = VolumeObject::with_ftype("ntfs");
    volume.block_size = Some(4096);

    let mut file = FileObject::with_filename("/Users/evidence/document.pdf");
    file.filesize = Some(1048576);
    file.hashes.set(
        HashType::Sha256,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
    );

    volume.append_file(file);
    doc.append_volume(volume);

    let xml = to_string(&doc)?;
    println!("{}", xml);

    Ok(())
}
```

## Core Types

### Document Structure

| Type | Description |
|------|-------------|
| `DFXMLObject` | Root document container with metadata, sources, and child objects |
| `DiskImageObject` | Disk image with filename, size, hashes, and nested structures |
| `PartitionSystemObject` | Partition table (MBR/GPT) containing partitions |
| `PartitionObject` | Individual partition with type, offset, and contents |
| `VolumeObject` | File system volume with geometry and file entries |
| `FileObject` | File metadata including timestamps, hashes, and byte runs |

### Supporting Types

| Type | Description |
|------|-------------|
| `Timestamp` | ISO 8601 timestamp with optional precision |
| `Hashes` | Collection of hash values (MD5, SHA1, SHA256, etc.) |
| `HashType` | Enum of supported hash algorithms |
| `ByteRun` | Disk location descriptor (offset, length, fill) |
| `ByteRuns` | Collection of byte runs with optional facet (data/inode/name) |
| `LibraryObject` | Library name and version for creator/build info |
| `NameType` / `MetaType` | File system entry type enums |

## Reader Module

### Functions

| Function | Description |
|----------|-------------|
| `parse(reader)` | Parse complete DFXML into a `DFXMLObject` with all children attached |
| `parse_file_objects(reader)` | Extract just the `FileObject`s as a `Vec` |

### Streaming Events

The `DFXMLReader` iterator yields `Event` variants:

| Event | Description |
|-------|-------------|
| `DFXMLStart(Box<DFXMLObject>)` | Document opened with metadata |
| `DFXMLEnd(Box<DFXMLObject>)` | Document closed (contains completed metadata) |
| `DiskImageStart(Box<DiskImageObject>)` | Disk image opened |
| `DiskImageEnd(Box<DiskImageObject>)` | Disk image closed (contains completed object) |
| `PartitionSystemStart(Box<PartitionSystemObject>)` | Partition system opened |
| `PartitionSystemEnd(Box<PartitionSystemObject>)` | Partition system closed (contains completed object) |
| `PartitionStart(Box<PartitionObject>)` | Partition opened |
| `PartitionEnd(Box<PartitionObject>)` | Partition closed (contains completed object) |
| `VolumeStart(Box<VolumeObject>)` | Volume opened |
| `VolumeEnd(Box<VolumeObject>)` | Volume closed (contains completed object with files) |
| `FileObject(Box<FileObject>)` | Complete file object |

### Supported Elements

The reader parses all standard DFXML elements:

- **Document**: `<dfxml>` with version and namespaces
- **Metadata**: `<creator>`, `<program>`, `<version>`, `<command_line>`, `<library>`, `<build_environment>`
- **Containers**: `<diskimageobject>`, `<partitionsystemobject>`, `<partitionobject>`, `<volume>`
- **Files**: `<fileobject>` with all standard child elements
- **Properties**: `<filename>`, `<filesize>`, `<inode>`, `<mode>`, `<uid>`, `<gid>`, `<nlink>`, `<link_target>`, `<libmagic>`, `<error>`
- **Allocation**: `<alloc>`, `<alloc_inode>`, `<alloc_name>`, `<used>`, `<orphan>`, `<compressed>`
- **Timestamps**: `<mtime>`, `<atime>`, `<ctime>`, `<crtime>`, `<dtime>`, `<bkup_time>` with precision
- **Hashes**: `<hashdigest>` with type attribute (md5, sha1, sha256, etc.)
- **Byte Runs**: `<byte_runs>` with facet, `<byte_run>` with offset/length attributes

## Writer Module

### Functions

| Function | Description |
|----------|-------------|
| `to_string(doc)` | Write to string with default formatting (indented) |
| `to_string_compact(doc)` | Write to string without indentation |
| `write(doc, writer)` | Write to any `std::io::Write` implementation |

### Configuration

```rust
use dfxml_rs::writer::{DFXMLWriter, WriterConfig};

// Default: indented with 2 spaces
let writer = DFXMLWriter::new();

// Compact: no indentation
let writer = DFXMLWriter::with_config(WriterConfig::compact());

// Custom: 4-space indentation
let writer = DFXMLWriter::with_config(
    WriterConfig::new()
        .with_indent(true)
        .with_indent_string("    ")
);

let xml = writer.write_to_string(&doc)?;
```

### Output Features

- XML declaration with UTF-8 encoding
- Proper DFXML and Dublin Core namespace declarations
- Elements written in DFXML schema order
- Boolean values as "0"/"1"
- Timestamps in RFC 3339 format
- Byte run facets included when multiple facet types present
- Self-closing tags for empty elements

## Project Structure

```
dfxml-rs/
├── src/
│   ├── lib.rs            # Crate entry point and re-exports
│   ├── error.rs          # Error types
│   ├── objects/          # Core data structures
│   │   ├── mod.rs
│   │   ├── common.rs     # Hashes, Timestamps, ByteRuns, etc.
│   │   ├── fileobject.rs # FileObject
│   │   ├── volume.rs     # VolumeObject, PartitionObject, DiskImageObject
│   │   └── dfxml.rs      # DFXMLObject (root document)
│   ├── reader.rs         # Streaming XML parser
│   └── writer.rs         # XML serializer
├── schema/
│   └── dfxml.xsd         # DFXML schema for reference
└── Cargo.toml
```

## Dependencies

- [`quick-xml`](https://crates.io/crates/quick-xml) - Fast XML parsing and writing
- [`chrono`](https://crates.io/crates/chrono) - Date/time handling
- [`thiserror`](https://crates.io/crates/thiserror) - Error type derivation
- [`serde`](https://crates.io/crates/serde) (optional) - Serialization support

## Related Projects

- [dfxml_python](https://github.com/dfxml-working-group/dfxml_python) - Python DFXML library (reference implementation)
- [dfxml_schema](https://github.com/dfxml-working-group/dfxml_schema) - DFXML XML Schema definitions
- [The Sleuth Kit](https://sleuthkit.org/) - Digital forensics toolkit that outputs DFXML

## License

Original contributions to the library and tools in this repository are licenced under the GNU Lesser General Public License, v3.0.

Some commits to this repository may contain a version of the DFXML Schema (in the form of the file ```dfxml.xsd```), which is in the public domain. The following text duplicates LICENSE.md from https://github.com/dfxml-working-group/dfxml_schema:

DFXML and its schema were developed by employees and contractors of the United States Government. Within the United States, copyright protection, under Section 105 of the United States Code, Title 17, is not available for any work of the United States Government and/or for any works created by United States Government employees. By that Section, and by agreement with the developing contractors, this work is in the public domain.

## Contributing

Issues, PRs, and bug reports can be submitted directly to this repository.
