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
- **Full Container Nesting**: Containers support arbitrary nesting matching the Python library (e.g., disk images in volumes, partition systems in partitions)
- **Unified Append Methods**: Generic `append()` methods with type-safe child enums for each container
- **Recursive Iteration**: Depth-first traversal of all descendants with `iter_descendants()`, plus `child_objects()` for direct children
- **External Element Preservation**: Non-DFXML namespace elements are preserved for round-trip XML processing
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
- `cat_fileobjects` - Extract fileobjects from a DFXML file
- `cat_partitions` - Concatenate DFXML documents with partition offset handling
- `dedup` - Detect and report duplicate files based on MD5 hashes

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

    // Type-specific append
    volume.append_file(file);
    doc.append_volume(volume);

    // Or use unified append with .into()
    // volume.append(file.into());
    // doc.append(volume.into());

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

### cat_fileobjects

Extract all fileobjects from a DFXML file and output a new DFXML document containing only those fileobjects. This is a Rust implementation of the Python `cat_fileobjects.py` tool from the [dfxml_python](https://github.com/dfxml-working-group/dfxml_python) project.

**Usage:**

```bash
cat_fileobjects [OPTIONS] <FILENAME>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<FILENAME>` | Input DFXML file to process |

**Options:**

| Option | Description |
|--------|-------------|
| `--cache` | Cache all fileobjects in memory before printing |
| `--debug` | Enable debug output to stderr |
| `--compact` | Output compact XML (no indentation) |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

**Examples:**

```bash
# Extract fileobjects from a DFXML file
cat_fileobjects input.dfxml > output.dfxml

# With debug output to see processing
cat_fileobjects --debug input.dfxml > output.dfxml

# Cache mode (read all into memory before writing)
cat_fileobjects --cache input.dfxml > output.dfxml

# Compact XML output
cat_fileobjects --compact input.dfxml > output.dfxml
```

**Output Format:**

The tool generates a complete DFXML document with:
- XML declaration and proper namespaces (DFXML and delta)
- Metadata section with creator information (program name, version, command line)
- Source section referencing the input file
- All fileobject elements extracted from the input, regardless of their original container (volume, partition, etc.)

This is useful for:
- Flattening nested DFXML structures
- Extracting file metadata from forensic analysis results
- Creating file-only manifests from complex disk image analysis output

### cat_partitions

Concatenate multiple DFXML documents with partition offset handling. This is a Rust implementation of the Python `cat_partitions.py` tool from the [dfxml_python](https://github.com/dfxml-working-group/dfxml_python) project.

Each input DFXML file is prefixed with its partition's byte offset from the start of the disk image. The tool combines all volumes and updates partition numbers and byte run `img_offset` attributes accordingly.

**Usage:**

```bash
cat_partitions [OPTIONS] <OFFSET:FILE>...
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<OFFSET:FILE>...` | List of DFXML files with partition offsets (e.g., `32256:part1.dfxml`) |

**Options:**

| Option | Description |
|--------|-------------|
| `-d, --debug` | Enable debug output to stderr |
| `--image-path <PATH>` | Path to the source image file to record in the output |
| `--compact` | Output compact XML (no indentation) |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

**Examples:**

```bash
# Concatenate two partition DFXML files
cat_partitions 32256:partition1.dfxml 1073741824:partition2.dfxml > combined.dfxml

# Include the source disk image path in output
cat_partitions --image-path disk.raw 32256:part1.dfxml 512:part2.dfxml > combined.dfxml

# With debug output
cat_partitions --debug 32256:part1.dfxml > combined.dfxml
```

**Input Format:**

Each input argument must be in the format `OFFSET:PATH` where:
- `OFFSET` is the partition's byte offset from the start of the disk image (in bytes)
- `PATH` is the path to the DFXML file for that partition

The tool assumes each input DFXML document has at most one volume.

**Output Processing:**

The tool performs the following transformations:
- Sets `partition_offset` on each volume based on the provided offset
- Updates `partition` attribute on all file objects with a sequential partition number
- Recalculates `img_offset` in byte runs as `fs_offset + partition_offset`
- Accumulates namespaces from all input documents
- Sorts partitions by offset before processing

### dedup

Detect and report duplicate files based on MD5 hashes in a DFXML file. This is a Rust implementation of the Python `dedup.py` tool from the [dfxml_python](https://github.com/dfxml-working-group/dfxml_python) project.

The tool reads a DFXML file, groups files by their MD5 hash, and reports statistics. It can optionally list distinct (unique) files or files that have duplicates.

**Usage:**

```bash
dedup [OPTIONS] <DFXML_FILE>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<DFXML_FILE>` | Input DFXML file to process |

**Options:**

| Option | Description |
|--------|-------------|
| `--verbose` | Enable verbose output |
| `--prefix <PREFIX>` | Only output files with the given path prefix |
| `--distinct` | Report the distinct (unique) files |
| `--dups` | Report files that are duplicates, with duplicate count |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

**Examples:**

```bash
# Show summary statistics only
dedup input.dfxml

# List all distinct (unique) files
dedup --distinct input.dfxml

# List all duplicate files with their duplicate count
dedup --dups input.dfxml

# Filter to files under a specific directory
dedup --dups --prefix /home/user input.dfxml

# Combine options
dedup --distinct --dups --prefix /data input.dfxml
```

**Output:**

The tool always prints a summary line:
```
Total files: 1,234  total MD5s processed: 1,200  Unique MD5s: 950
```

With `--distinct`, each unique file is printed:
```
distinct: /path/to/unique/file.txt
```

With `--dups`, each duplicate file is printed with the count of files sharing its hash:
```
dups: /path/to/duplicate1.txt 3
dups: /path/to/duplicate2.txt 3
dups: /path/to/duplicate3.txt 3
```

## Examples

The `demos/` directory contains example programs demonstrating library usage. These are ports of examples from the Python dfxml_python project.

### demo_mac_timeline

Produces a MAC-times (Modified, Accessed, Changed, Created) timeline from a DFXML file. This demonstrates using the streaming reader to extract timestamp information.

**Run the example:**

```bash
cargo run --example demo_mac_timeline <filename.xml>
```

**Output format:**

The output is tab-separated with three columns:
- Timestamp (RFC 3339 / ISO 8601 format)
- Filename
- Event type (`modified`, `accessed`, `changed`, or `created`)

**Example output:**

```
2024-01-15T10:30:00+00:00	/home/user/document.txt	modified
2024-01-15T10:30:00+00:00	/home/user/document.txt	changed
2024-01-15T14:22:00+00:00	/home/user/document.txt	accessed
2024-01-20T09:00:00+00:00	/home/user/notes.md	created
```

### demo_sizes

Calculates file size statistics grouped by file extension. This demonstrates using the streaming reader to aggregate data across all files.

**Run the example:**

```bash
cargo run --example demo_sizes <filename.xml>
```

**Output format:**

The output is a formatted table with columns:
- Ext: File extension (lowercase, or "(none)" for files without extension)
- Count: Number of files with that extension
- Total: Total size in bytes
- Average: Average file size in bytes
- StdDev: Standard deviation of file sizes

**Example output:**

```
     Ext       Count        Total      Average       StdDev
  (none)          15         4096        273.1        512.3
     doc          23      1048576      45590.3      12453.2
     pdf         142     52428800     369216.9     156842.1
     txt         891       102400        114.9         87.6
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
| `ExternalElement` | Non-DFXML namespace XML element (for round-tripping) |
| `Externals` | Collection of external elements |

### Container Nesting

Each container type can hold specific child types, matching the Python DFXML library:

| Container | Can Contain |
|-----------|-------------|
| `DFXMLObject` | `DiskImageObject`, `PartitionSystemObject`, `PartitionObject`, `VolumeObject`, `FileObject` |
| `DiskImageObject` | `PartitionSystemObject`, `PartitionObject`, `VolumeObject`, `FileObject` |
| `PartitionSystemObject` | `PartitionObject`, `FileObject` |
| `PartitionObject` | `PartitionSystemObject`, `PartitionObject`, `VolumeObject`, `FileObject` |
| `VolumeObject` | `DiskImageObject`, `VolumeObject`, `FileObject` |

### Unified Append Methods

All container types support a unified `append()` method using type-safe child enums:

```rust
use dfxml_rs::objects::{
    DFXMLObject, VolumeObject, FileObject,
    ChildObject,        // For DFXMLObject
    VolumeChild,        // For VolumeObject
    PartitionChild,     // For PartitionObject
    PartitionSystemChild, // For PartitionSystemObject
    DiskImageChild,     // For DiskImageObject
};

let mut doc = DFXMLObject::new();

// Using the unified append with explicit enum (FileObject must be boxed)
doc.append(ChildObject::Volume(VolumeObject::new()));
doc.append(ChildObject::File(Box::new(FileObject::with_filename("test.txt"))));

// Using the From trait for ergonomic conversion (boxing is automatic)
doc.append(VolumeObject::with_ftype("ntfs").into());
doc.append(FileObject::with_filename("another.txt").into());

// Type-specific methods still work (no boxing needed)
doc.append_volume(VolumeObject::new());
doc.append_file(FileObject::new());
```

Each container has its own child enum with `From` implementations. Large variants are boxed to reduce enum size:

| Container | Child Enum | Variants |
|-----------|------------|----------|
| `DFXMLObject` | `ChildObject` | `DiskImage`, `PartitionSystem`, `Partition`, `Volume`, `File(Box<...>)` |
| `DiskImageObject` | `DiskImageChild` | `PartitionSystem`, `Partition`, `Volume`, `File(Box<...>)` |
| `PartitionSystemObject` | `PartitionSystemChild` | `Partition(Box<...>)`, `File(Box<...>)` |
| `PartitionObject` | `PartitionChild` | `PartitionSystem`, `Partition`, `Volume`, `File(Box<...>)` |
| `VolumeObject` | `VolumeChild` | `DiskImage`, `Volume`, `File(Box<...>)` |

### Iteration Methods

#### Direct Children

Use `child_objects()` to iterate over immediate children only:

```rust
use dfxml_rs::objects::{DFXMLObject, DFXMLChild, VolumeChildRef};

// DFXMLObject direct children
for child in doc.child_objects() {
    match child {
        DFXMLChild::DiskImage(di) => println!("Disk image: {:?}", di.image_filename),
        DFXMLChild::Volume(v) => println!("Volume: {:?}", v.ftype_str),
        DFXMLChild::File(f) => println!("File: {:?}", f.filename),
        _ => {}
    }
}

// VolumeObject direct children
for child in volume.child_objects() {
    match child {
        VolumeChildRef::DiskImage(di) => println!("Nested disk image"),
        VolumeChildRef::Volume(v) => println!("Nested volume"),
        VolumeChildRef::File(f) => println!("File: {:?}", f.filename),
    }
}
```

#### Recursive Descendants (Depth-First)

Use `iter_descendants()` on `DFXMLObject` for depth-first traversal of all descendants:

```rust
use dfxml_rs::objects::{DFXMLObject, DFXMLChild};

// Iterate all descendants in depth-first order
for child in doc.iter_descendants() {
    match child {
        DFXMLChild::DiskImage(di) => println!("Disk image: {:?}", di.image_filename),
        DFXMLChild::PartitionSystem(ps) => println!("Partition system: {:?}", ps.pstype_str),
        DFXMLChild::Partition(p) => println!("Partition: {:?}", p.partition_index),
        DFXMLChild::Volume(v) => println!("Volume: {:?}", v.ftype_str),
        DFXMLChild::File(f) => println!("File: {:?}", f.filename),
    }
}

// Shorthand alias
for child in doc.iter() {
    // Same as iter_descendants()
}
```

#### Recursive File Iteration

Use `iter_all_files()` to recursively iterate only files:

```rust
// All files anywhere in the document hierarchy
for file in doc.iter_files() {
    println!("{}: {} bytes", 
        file.filename.as_deref().unwrap_or("<unnamed>"),
        file.filesize.unwrap_or(0));
}

// All files in a volume (including nested volumes and disk images)
for file in volume.iter_all_files() {
    println!("{}", file.filename.as_deref().unwrap_or("<unnamed>"));
}

// All files in a disk image
for file in disk_image.iter_all_files() {
    println!("{}", file.filename.as_deref().unwrap_or("<unnamed>"));
}
```

### Child Reference Enums (for Iteration)

When iterating, reference-based enums are used:

| Container | Reference Enum | Description |
|-----------|---------------|-------------|
| `DFXMLObject` | `DFXMLChild<'a>` | References to any DFXML child type |
| `DiskImageObject` | `DiskImageChildRef<'a>` | References to disk image children |
| `PartitionSystemObject` | `PartitionSystemChildRef<'a>` | References to partition system children |
| `PartitionObject` | `PartitionChildRef<'a>` | References to partition children |
| `VolumeObject` | `VolumeChildRef<'a>` | References to volume children |

### External Elements

All container types and `FileObject` have an `externals` field for preserving non-DFXML namespace elements during round-trip processing:

```rust
use dfxml_rs::objects::{DFXMLObject, ExternalElement, Externals};

let mut doc = DFXMLObject::new();

// Create an external element from a custom namespace
let mut custom_elem = ExternalElement::with_namespace(
    "http://example.org/custom",
    "custom_metadata"
);
custom_elem.set_text("Some custom value");
custom_elem.add_attribute("version", "1.0");

// Add a child element
let mut child = ExternalElement::new("nested_info");
child.set_text("Nested content");
custom_elem.add_child(child);

// Add to the document
doc.externals.push(custom_elem);

// Check if there are external elements
if !doc.externals.is_empty() {
    println!("Document has {} external elements", doc.externals.len());
    for ext in &doc.externals {
        println!("  {} (ns: {:?})", ext.tag_name, ext.namespace);
    }
}
```

The `ExternalElement` type provides:

| Method | Description |
|--------|-------------|
| `new(tag_name)` | Create with just a tag name |
| `with_namespace(ns, tag_name)` | Create with namespace URI and tag name |
| `set_text(text)` | Set the text content |
| `add_attribute(name, value)` | Add an attribute |
| `add_child(element)` | Add a child element |
| `qualified_name()` | Get `{namespace}tag_name` format |

The `Externals` collection provides:

| Method | Description |
|--------|-------------|
| `new()` | Create empty collection |
| `is_empty()` | Check if empty |
| `len()` | Get element count |
| `push(element)` | Add element (panics if DFXML namespace) |
| `try_push(element)` | Add element (returns `Result`) |
| `iter()` | Iterate over elements |
| `clear()` | Remove all elements |

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
│   │   ├── mod.rs        # Module exports
│   │   ├── common.rs     # Hashes, Timestamps, ByteRuns, Externals, etc.
│   │   ├── fileobject.rs # FileObject with metadata and externals
│   │   ├── volume.rs     # VolumeObject, PartitionObject, DiskImageObject,
│   │   │                 # PartitionSystemObject, and child enums
│   │   └── dfxml.rs      # DFXMLObject, ChildObject, DFXMLIterator
│   ├── bin/              # CLI tools (requires 'cli' feature)
│   │   ├── walk_to_dfxml.rs
│   │   ├── cat_fileobjects.rs
│   │   ├── cat_partitions.rs
│   │   └── dedup.rs
│   ├── reader.rs         # Streaming XML parser
│   ├── writer.rs         # XML serializer
│   └── validation.rs     # XSD validation (requires 'validation' feature)
├── demos/                # Example programs
│   ├── demo_mac_timeline.rs
│   └── demo_sizes.rs
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
