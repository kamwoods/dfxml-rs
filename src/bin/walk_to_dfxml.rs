//! walk_to_dfxml - Walk a directory tree and generate DFXML output.
//!
//! This tool recursively walks a directory, collecting file metadata and
//! optionally computing cryptographic hashes, then outputs the results
//! as DFXML to stdout.
//!
//! # Usage
//!
//! ```bash
//! walk_to_dfxml [OPTIONS] [PATH]
//! ```
//!
//! # Examples
//!
//! ```bash
//! # Walk current directory
//! walk_to_dfxml > manifest.dfxml
//!
//! # Walk specific directory with 4 threads
//! walk_to_dfxml -j 4 /path/to/directory > manifest.dfxml
//!
//! # Skip hash computation for faster scanning
//! walk_to_dfxml --ignore-hashes /path/to/directory
//!
//! # Ignore specific properties
//! walk_to_dfxml -i inode -i mtime /path/to/directory
//! ```

use std::collections::{HashMap, HashSet};
use std::fs::{self, File, Metadata};
use std::io::{self, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use clap::Parser;
use digest::Digest;
use rayon::prelude::*;
use walkdir::WalkDir;

use dfxml_rs::objects::{
    DFXMLObject, FileObject, HashType, Hashes, LibraryObject, NameType, Timestamp, TimestampName,
};
use dfxml_rs::writer;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Walk a directory tree and generate DFXML output.
#[derive(Parser, Debug)]
#[command(name = "walk_to_dfxml")]
#[command(version = VERSION)]
#[command(about = "Walk a directory tree and generate DFXML output")]
#[command(long_about = "Recursively walks a directory, collecting file metadata and \
    optionally computing cryptographic hashes, then outputs the results as DFXML to stdout.")]
struct Args {
    /// Directory to walk (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Enable debug output
    #[arg(short, long)]
    debug: bool,

    /// Ignore a property on file objects (can be specified multiple times).
    /// Use 'property@type' to ignore only for specific file types (e.g., 'mtime@d' for directories).
    #[arg(short, long = "ignore", value_name = "PROPERTY")]
    ignore_properties: Vec<String>,

    /// Do not calculate any hashes
    #[arg(long)]
    ignore_hashes: bool,

    /// Number of file-processing threads to run
    #[arg(short, long, default_value = "1")]
    jobs: usize,

    /// Follow symbolic links when walking directories
    #[arg(long)]
    follow_links: bool,

    /// Output compact XML (no indentation)
    #[arg(long)]
    compact: bool,
}

/// Properties that can be ignored
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Property {
    Filename,
    NameType,
    Filesize,
    Alloc,
    Inode,
    Mode,
    Nlink,
    Uid,
    Gid,
    Mtime,
    Atime,
    Ctime,
    Crtime,
    LinkTarget,
    Md5,
    Sha1,
    Sha256,
    Sha384,
    Sha512,
    Error,
}

impl Property {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "filename" => Some(Property::Filename),
            "name_type" => Some(Property::NameType),
            "filesize" => Some(Property::Filesize),
            "alloc" => Some(Property::Alloc),
            "inode" => Some(Property::Inode),
            "mode" => Some(Property::Mode),
            "nlink" => Some(Property::Nlink),
            "uid" => Some(Property::Uid),
            "gid" => Some(Property::Gid),
            "mtime" => Some(Property::Mtime),
            "atime" => Some(Property::Atime),
            "ctime" => Some(Property::Ctime),
            "crtime" => Some(Property::Crtime),
            "link_target" | "linktarget" => Some(Property::LinkTarget),
            "md5" => Some(Property::Md5),
            "sha1" => Some(Property::Sha1),
            "sha256" => Some(Property::Sha256),
            "sha384" => Some(Property::Sha384),
            "sha512" => Some(Property::Sha512),
            "error" => Some(Property::Error),
            _ => None,
        }
    }

    fn all_hashes() -> Vec<Property> {
        vec![
            Property::Md5,
            Property::Sha1,
            Property::Sha256,
            Property::Sha384,
            Property::Sha512,
        ]
    }
}

/// Tracks which properties to ignore, optionally by file type
#[derive(Debug, Default, Clone)]
struct IgnoreConfig {
    /// Map from property to set of name_types to ignore for (None = all types)
    ignores: HashMap<Property, HashSet<Option<char>>>,
}

impl IgnoreConfig {
    fn new() -> Self {
        Self::default()
    }

    fn add(&mut self, property: Property, name_type: Option<char>) {
        self.ignores
            .entry(property)
            .or_default()
            .insert(name_type);
    }

    fn should_ignore(&self, property: Property, name_type: Option<char>) -> bool {
        if let Some(types) = self.ignores.get(&property) {
            // Check for wildcard (all types)
            if types.contains(&None) {
                return true;
            }
            // Check for specific type
            if let Some(nt) = name_type {
                if types.contains(&Some(nt)) {
                    return true;
                }
            }
        }
        false
    }

    fn add_all_hashes(&mut self) {
        for prop in Property::all_hashes() {
            self.add(prop, None);
        }
    }
}

/// Parse ignore property specifications from command line
fn parse_ignore_specs(specs: &[String], ignore_hashes: bool) -> IgnoreConfig {
    let mut config = IgnoreConfig::new();

    for spec in specs {
        let parts: Vec<&str> = spec.split('@').collect();
        let prop_name = parts[0];
        let name_type = if parts.len() > 1 {
            parts[1].chars().next()
        } else {
            None // Means "all types"
        };

        if let Some(property) = Property::from_str(prop_name) {
            config.add(property, name_type);
        } else {
            eprintln!("Warning: Unknown property '{}', ignoring", prop_name);
        }
    }

    if ignore_hashes {
        config.add_all_hashes();
    }

    config
}

/// Determine the name_type character for a file based on its metadata
fn get_name_type(_path: &Path, metadata: &Metadata) -> char {
    let file_type = metadata.file_type();

    if file_type.is_symlink() {
        'l'
    } else if file_type.is_dir() {
        'd'
    } else if file_type.is_file() {
        'r'
    } else if file_type.is_char_device() {
        'c'
    } else if file_type.is_block_device() {
        'b'
    } else if file_type.is_fifo() {
        'p'
    } else if file_type.is_socket() {
        's'
    } else {
        // Unknown type
        '?'
    }
}

/// Convert a SystemTime to a Timestamp
fn system_time_to_timestamp(st: SystemTime, name: TimestampName) -> Option<Timestamp> {
    let datetime: DateTime<Utc> = st.into();
    Some(Timestamp {
        name: Some(name),
        time: Some(datetime.fixed_offset()),
        prec: None,
    })
}

/// Compute hashes for a file
fn compute_hashes(
    path: &Path,
    ignore_config: &IgnoreConfig,
    name_type: char,
) -> (Hashes, Option<String>) {
    let mut hashes = Hashes::new();
    let mut error = None;

    // Only compute hashes for regular files
    if name_type != 'r' && name_type != '-' {
        return (hashes, error);
    }

    // Check if all hashes are ignored
    let compute_md5 = !ignore_config.should_ignore(Property::Md5, Some(name_type));
    let compute_sha1 = !ignore_config.should_ignore(Property::Sha1, Some(name_type));
    let compute_sha256 = !ignore_config.should_ignore(Property::Sha256, Some(name_type));
    let compute_sha384 = !ignore_config.should_ignore(Property::Sha384, Some(name_type));
    let compute_sha512 = !ignore_config.should_ignore(Property::Sha512, Some(name_type));

    if !compute_md5 && !compute_sha1 && !compute_sha256 && !compute_sha384 && !compute_sha512 {
        return (hashes, error);
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            error = Some(format!("Failed to open file for hashing: {}", e));
            return (hashes, error);
        }
    };

    let mut reader = io::BufReader::with_capacity(4 * 1024 * 1024, file); // 4MB buffer
    let mut buffer = vec![0u8; 4 * 1024 * 1024];

    let mut md5_hasher = md5::Md5::new();
    let mut sha1_hasher = sha1::Sha1::new();
    let mut sha256_hasher = sha2::Sha256::new();
    let mut sha384_hasher = sha2::Sha384::new();
    let mut sha512_hasher = sha2::Sha512::new();

    let mut any_error = false;

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                let data = &buffer[..n];
                if compute_md5 {
                    md5_hasher.update(data);
                }
                if compute_sha1 {
                    sha1_hasher.update(data);
                }
                if compute_sha256 {
                    sha256_hasher.update(data);
                }
                if compute_sha384 {
                    sha384_hasher.update(data);
                }
                if compute_sha512 {
                    sha512_hasher.update(data);
                }
            }
            Err(e) => {
                any_error = true;
                error = Some(format!("Error reading file for hashing: {}", e));
                break;
            }
        }
    }

    if !any_error {
        if compute_md5 {
            hashes.set(HashType::Md5, format!("{:x}", md5_hasher.finalize()));
        }
        if compute_sha1 {
            hashes.set(HashType::Sha1, format!("{:x}", sha1_hasher.finalize()));
        }
        if compute_sha256 {
            hashes.set(HashType::Sha256, format!("{:x}", sha256_hasher.finalize()));
        }
        if compute_sha384 {
            hashes.set(HashType::Sha384, format!("{:x}", sha384_hasher.finalize()));
        }
        if compute_sha512 {
            hashes.set(HashType::Sha512, format!("{:x}", sha512_hasher.finalize()));
        }
    }

    (hashes, error)
}

/// Create a FileObject from a path
fn path_to_fileobject(
    path: &Path,
    base_path: &Path,
    ignore_config: &IgnoreConfig,
) -> Result<FileObject, String> {
    let mut fobj = FileObject::new();

    // Get metadata (use symlink_metadata to not follow symlinks)
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) => {
            // Create minimal FileObject with error
            let rel_path = path
                .strip_prefix(base_path)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            fobj.filename = Some(rel_path);
            fobj.error = Some(format!("Failed to get metadata: {}", e));
            return Ok(fobj);
        }
    };

    let name_type_char = get_name_type(path, &metadata);
    let name_type_opt = Some(name_type_char);

    // Set filename (relative to base path)
    if !ignore_config.should_ignore(Property::Filename, name_type_opt) {
        let rel_path = path
            .strip_prefix(base_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        // Use "." for the base directory itself
        fobj.filename = Some(if rel_path.is_empty() {
            ".".to_string()
        } else {
            rel_path
        });
    }

    // Set name_type
    if !ignore_config.should_ignore(Property::NameType, name_type_opt) {
        fobj.name_type = NameType::from_char(name_type_char);
    }

    // Set filesize (for regular files)
    if !ignore_config.should_ignore(Property::Filesize, name_type_opt) {
        if name_type_char == 'r' || name_type_char == '-' {
            fobj.filesize = Some(metadata.len());
        }
    }

    // Set allocation status (assume allocated for live filesystem)
    if !ignore_config.should_ignore(Property::Alloc, name_type_opt) {
        fobj.alloc = Some(true);
    }

    // Set inode
    if !ignore_config.should_ignore(Property::Inode, name_type_opt) {
        fobj.inode = Some(metadata.ino());
    }

    // Set mode
    if !ignore_config.should_ignore(Property::Mode, name_type_opt) {
        fobj.mode = Some(metadata.permissions().mode());
    }

    // Set nlink
    if !ignore_config.should_ignore(Property::Nlink, name_type_opt) {
        fobj.nlink = Some(metadata.nlink() as u32);
    }

    // Set uid
    if !ignore_config.should_ignore(Property::Uid, name_type_opt) {
        fobj.uid = Some(metadata.uid());
    }

    // Set gid
    if !ignore_config.should_ignore(Property::Gid, name_type_opt) {
        fobj.gid = Some(metadata.gid());
    }

    // Set mtime
    if !ignore_config.should_ignore(Property::Mtime, name_type_opt) {
        if let Ok(mtime) = metadata.modified() {
            fobj.mtime = system_time_to_timestamp(mtime, TimestampName::Mtime);
        }
    }

    // Set atime
    if !ignore_config.should_ignore(Property::Atime, name_type_opt) {
        if let Ok(atime) = metadata.accessed() {
            fobj.atime = system_time_to_timestamp(atime, TimestampName::Atime);
        }
    }

    // Set ctime (Unix only - metadata change time)
    #[cfg(unix)]
    if !ignore_config.should_ignore(Property::Ctime, name_type_opt) {
        use std::time::UNIX_EPOCH;
        let ctime_secs = metadata.ctime();
        if ctime_secs >= 0 {
            if let Some(ctime) = UNIX_EPOCH.checked_add(std::time::Duration::from_secs(ctime_secs as u64)) {
                fobj.ctime = system_time_to_timestamp(ctime, TimestampName::Ctime);
            }
        }
    }

    // Set crtime (creation time - platform specific)
    if !ignore_config.should_ignore(Property::Crtime, name_type_opt) {
        if let Ok(crtime) = metadata.created() {
            fobj.crtime = system_time_to_timestamp(crtime, TimestampName::Crtime);
        }
    }

    // Set link target for symlinks
    if !ignore_config.should_ignore(Property::LinkTarget, name_type_opt) {
        if name_type_char == 'l' {
            if let Ok(target) = fs::read_link(path) {
                fobj.link_target = Some(target.to_string_lossy().to_string());
            }
        }
    }

    // Compute hashes
    let (hashes, hash_error) = compute_hashes(path, ignore_config, name_type_char);
    fobj.hashes = hashes;

    // Set error if any occurred during hashing
    if let Some(err) = hash_error {
        if !ignore_config.should_ignore(Property::Error, name_type_opt) {
            fobj.error = Some(err);
        }
    }

    Ok(fobj)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.debug {
        eprintln!("Debug mode enabled");
        eprintln!("Walking: {:?}", args.path);
        eprintln!("Jobs: {}", args.jobs);
    }

    // Validate jobs count
    if args.jobs == 0 {
        eprintln!("Error: jobs must be at least 1");
        std::process::exit(1);
    }

    // Parse ignore configuration
    let ignore_config = parse_ignore_specs(&args.ignore_properties, args.ignore_hashes);

    if args.debug {
        eprintln!("Ignore config: {:?}", ignore_config);
    }

    // Canonicalize base path
    let base_path = args.path.canonicalize().unwrap_or_else(|_| args.path.clone());

    // Collect all paths first
    let mut paths: Vec<PathBuf> = Vec::new();

    let walker = WalkDir::new(&base_path)
        .follow_links(args.follow_links)
        .sort_by_file_name();

    for entry in walker {
        match entry {
            Ok(e) => {
                paths.push(e.path().to_path_buf());
            }
            Err(e) => {
                if args.debug {
                    eprintln!("Warning: Error walking directory: {}", e);
                }
            }
        }
    }

    if args.debug {
        eprintln!("Found {} paths", paths.len());
    }

    // Process paths (in parallel if jobs > 1)
    let file_objects: Vec<FileObject> = if args.jobs > 1 {
        // Configure rayon thread pool
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.jobs)
            .build_global()
            .ok();

        // Process in parallel
        let results: Vec<_> = paths
            .par_iter()
            .map(|path| path_to_fileobject(path, &base_path, &ignore_config))
            .collect();

        // Collect results, maintaining order by sorting by filename
        let mut file_objects: Vec<FileObject> = results
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        // Sort by filename to ensure deterministic output
        file_objects.sort_by(|a, b| a.filename.cmp(&b.filename));
        file_objects
    } else {
        // Process sequentially
        paths
            .iter()
            .filter_map(|path| path_to_fileobject(path, &base_path, &ignore_config).ok())
            .collect()
    };

    // Build DFXML document
    let mut dobj = DFXMLObject::new();
    dobj.program = Some("walk_to_dfxml".to_string());
    dobj.program_version = Some(VERSION.to_string());
    dobj.command_line = Some(std::env::args().collect::<Vec<_>>().join(" "));

    // Add creator libraries
    dobj.add_creator_library(LibraryObject {
        name: Some("Rust".to_string()),
        version: Some(env!("CARGO_PKG_RUST_VERSION").to_string()),
    });
    dobj.add_creator_library(LibraryObject {
        name: Some("dfxml-rs".to_string()),
        version: Some(dfxml_rs::VERSION.to_string()),
    });

    // Add all file objects
    for fobj in file_objects {
        dobj.append_file(fobj);
    }

    // Output DFXML
    let config = if args.compact {
        writer::WriterConfig::compact()
    } else {
        writer::WriterConfig::default()
    };

    let xml = writer::DFXMLWriter::with_config(config).write_to_string(&dobj)?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(xml.as_bytes())?;
    handle.write_all(b"\n")?;

    Ok(())
}
