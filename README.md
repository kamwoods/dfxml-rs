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
- **XSD Validation**: Validate DFXML documents against the official schema (optional `validation` feature)
- **CLI Tools**: Command-line utilities for working with DFXML (optional `cli` feature)
- **Optional Serde Support**: Enable the `serde` feature for serialization/deserialization

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
dfxml-rs = { path = "path/to/dfxml-rs" }

# Optional: enable serde support
# dfxml-rs = { path = "path/to/dfxml-rs", features = ["serde"] }
```

## Building

### Library Only

```bash
cargo build --release
```

### With CLI Tools

To build the command-line tools, enable the `cli` feature:

```bash
cargo build --release --features cli
```

This builds the following tools:
- `walk_to_dfxml` - Walk a directory tree and generate DFXML output

### With XSD Validation

To enable XSD schema validation, enable the `validation` feature:

```bash
cargo build --release --features validation
```

**Note:** The `validation` feature requires libxml2 to be installed on your system:

- **Ubuntu/Debian:** `sudo apt-get install libxml2-dev`
- **macOS:** `brew install libxml2`
- **Windows:** See libxml2 documentation for installation instructions

You must also initialize the dfxml_schema submodule:

```bash
git submodule update --init --recursive
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

## CLI Tools

The following command-line tools are available when building with `--features cli`.

### walk_to_dfxml

Walk a directory tree and generate DFXML output to stdout. This is a Rust implementation of the Python `walk_to_dfxml.py` tool from the [dfxml_python](https://github.com/dfxml-working-group/dfxml_python) project.

**Usage:**

```bash
walk_to_dfxml [OPTIONS] [PATH]
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `[PATH]` | Directory to walk (defaults to current directory) |

**Options:**

| Option | Description |
|--------|-------------|
| `-d, --debug` | Enable debug output |
| `-i, --ignore <PROPERTY>` | Ignore a property on file objects (can be specified multiple times) |
| `--ignore-hashes` | Do not calculate any hashes |
| `-j, --jobs <N>` | Number of file-processing threads (default: 1) |
| `--follow-links` | Follow symbolic links when walking directories |
| `--compact` | Output compact XML (no indentation) |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

**Ignorable Properties:**

Use `-i` to exclude specific properties from the output:

- File identification: `filename`, `name_type`, `filesize`, `alloc`
- Unix metadata: `inode`, `mode`, `nlink`, `uid`, `gid`
- Timestamps: `mtime`, `atime`, `ctime`, `crtime`
- Symlinks: `link_target`
- Hashes: `md5`, `sha1`, `sha256`, `sha384`, `sha512`
- Errors: `error`

Property ignores can be restricted to specific file types using `property@type` syntax:
- `d` = directory
- `r` = regular file
- `l` = symbolic link
- `b` = block device
- `c` = character device
- `p` = FIFO/pipe
- `s` = socket

**Examples:**

```bash
# Walk current directory, output to file
walk_to_dfxml > manifest.dfxml

# Walk specific directory with 4 threads for parallel hash computation
walk_to_dfxml -j 4 /path/to/directory > manifest.dfxml

# Skip all hash computation for faster scanning
walk_to_dfxml --ignore-hashes /path/to/directory > manifest.dfxml

# Ignore inode numbers and modification times
walk_to_dfxml -i inode -i mtime /path/to/directory > manifest.dfxml

# Ignore modification times only for directories
walk_to_dfxml -i mtime@d /path/to/directory > manifest.dfxml

# Output compact XML
walk_to_dfxml --compact /path/to/directory > manifest.dfxml
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

## Validation Module

The `validation` module provides XSD schema validation for DFXML documents. This feature requires the `validation` feature flag and libxml2 to be installed.

### Functions

| Function | Description |
|----------|-------------|
| `validate_file(path, schema)` | Validate a DFXML file against the schema |
| `validate_str(xml, schema)` | Validate a DFXML string against the schema |
| `validate_document(doc, schema)` | Validate a `DFXMLObject` against the schema |

### Usage

```rust
use dfxml_rs::validation::{validate_file, validate_str, validate_document};
use dfxml_rs::objects::DFXMLObject;

// Validate a file (uses default schema path)
validate_file("forensic_output.xml", None)?;

// Validate with a custom schema path
validate_file("forensic_output.xml", Some("/path/to/dfxml.xsd"))?;

// Validate an XML string
let xml = r#"<?xml version="1.0"?>
<dfxml version="1.0">
  <fileobject>
    <filename>test.txt</filename>
  </fileobject>
</dfxml>"#;
validate_str(xml, None)?;

// Validate a document object
let doc = DFXMLObject::new();
validate_document(&doc, None)?;
```

### Default Schema Location

By default, the validation functions look for the schema at `external/dfxml_schema/dfxml.xsd` (the submodule location). You can override this by passing a custom path.

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
│   ├── bin/              # CLI tools (requires 'cli' feature)
│   │   └── walk_to_dfxml.rs
│   ├── reader.rs         # Streaming XML parser
│   ├── writer.rs         # XML serializer
│   └── validation.rs     # XSD validation (requires 'validation' feature)
├── .github/
│   └── workflows/
│       └── build.yml     # CI workflow
├── external/
│   └── dfxml_schema/     # DFXML schema (git submodule)
└── Cargo.toml
```

## Dependencies

### Core Library

- [`quick-xml`](https://crates.io/crates/quick-xml) - Fast XML parsing and writing
- [`chrono`](https://crates.io/crates/chrono) - Date/time handling
- [`thiserror`](https://crates.io/crates/thiserror) - Error type derivation
- [`serde`](https://crates.io/crates/serde) (optional) - Serialization support

### CLI Tools (optional, `cli` feature)

- [`clap`](https://crates.io/crates/clap) - Command-line argument parsing
- [`walkdir`](https://crates.io/crates/walkdir) - Directory traversal
- [`rayon`](https://crates.io/crates/rayon) - Parallel processing
- [`md-5`](https://crates.io/crates/md-5), [`sha1`](https://crates.io/crates/sha1), [`sha2`](https://crates.io/crates/sha2) - Hash computation

### XSD Validation (optional, `validation` feature)

- [`libxml`](https://crates.io/crates/libxml) - Rust bindings to libxml2 (requires libxml2 system library)

## Related Projects

- [dfxml_python](https://github.com/dfxml-working-group/dfxml_python) - Python DFXML library (reference implementation)
- [dfxml_schema](https://github.com/dfxml-working-group/dfxml_schema) - DFXML XML Schema definitions
- [The Sleuth Kit](https://sleuthkit.org/) - Digital forensics toolkit that outputs DFXML

## License

Original contributions to the library and tools in this repository are licenced under the GNU Lesser General Public License, v3.0.

This repository points to https://github.com/dfxml-working-group/dfxml_schema as a submodule. The LICENSE.md text from that repository is noted in the following paragraph.

DFXML and its schema were developed by employees and contractors of the United States Government. Within the United States, copyright protection, under Section 105 of the United States Code, Title 17, is not available for any work of the United States Government and/or for any works created by United States Government employees. By that Section, and by agreement with the developing contractors, this work is in the public domain.

## Contributing

Issues, PRs, and bug reports can be submitted directly to this repository.
