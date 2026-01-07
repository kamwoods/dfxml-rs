//! demo_mac_timeline - Produce a MAC-times timeline from a DFXML file.
//!
//! This demo reads a DFXML file and produces a sorted timeline of file
//! modification, access, change, and creation times. It is a Rust port
//! of the Python `demo_mac_timeline_iter.py` from the dfxml_python project.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example demo_mac_timeline <filename.xml>
//! ```
//!
//! # Output
//!
//! The output is a tab-separated timeline with three columns:
//! - Timestamp (ISO 8601 format)
//! - Filename
//! - Event type (modified, created, changed, or accessed)

use std::cmp::Ordering;
use std::env;
use std::fs::File;
use std::io::BufReader;

use dfxml_rs::reader::{DFXMLReader, Event};

/// A timeline entry representing a single timestamp event.
#[derive(Debug)]
struct TimelineEntry {
    /// The timestamp of the event
    timestamp: String,
    /// The filename associated with the event
    filename: String,
    /// The type of event (modified, created, changed, accessed)
    event_type: &'static str,
}

impl TimelineEntry {
    fn new(timestamp: String, filename: String, event_type: &'static str) -> Self {
        Self {
            timestamp,
            filename,
            event_type,
        }
    }
}

impl Ord for TimelineEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp
            .cmp(&other.timestamp)
            .then_with(|| self.filename.cmp(&other.filename))
            .then_with(|| self.event_type.cmp(other.event_type))
    }
}

impl PartialOrd for TimelineEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for TimelineEntry {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
            && self.filename == other.filename
            && self.event_type == other.event_type
    }
}

impl Eq for TimelineEntry {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <filename.xml>", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];
    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let dfxml_reader = DFXMLReader::from_reader(reader);

    let mut timeline: Vec<TimelineEntry> = Vec::new();

    for result in dfxml_reader {
        match result {
            Ok(Event::FileObject(fi)) => {
                let filename = fi.filename.clone().unwrap_or_default();

                // Add mtime (modification time)
                if let Some(ref mtime) = fi.mtime {
                    if let Some(ref time) = mtime.time {
                        timeline.push(TimelineEntry::new(
                            time.to_rfc3339(),
                            filename.clone(),
                            "modified",
                        ));
                    }
                }

                // Add crtime (creation time)
                if let Some(ref crtime) = fi.crtime {
                    if let Some(ref time) = crtime.time {
                        timeline.push(TimelineEntry::new(
                            time.to_rfc3339(),
                            filename.clone(),
                            "created",
                        ));
                    }
                }

                // Add ctime (change time / metadata change)
                if let Some(ref ctime) = fi.ctime {
                    if let Some(ref time) = ctime.time {
                        timeline.push(TimelineEntry::new(
                            time.to_rfc3339(),
                            filename.clone(),
                            "changed",
                        ));
                    }
                }

                // Add atime (access time)
                if let Some(ref atime) = fi.atime {
                    if let Some(ref time) = atime.time {
                        timeline.push(TimelineEntry::new(
                            time.to_rfc3339(),
                            filename.clone(),
                            "accessed",
                        ));
                    }
                }
            }
            Ok(_) => {
                // Ignore other events (volumes, partitions, etc.)
            }
            Err(e) => {
                eprintln!("Error parsing DFXML: {}", e);
                return Err(e.into());
            }
        }
    }

    // Sort the timeline by timestamp
    timeline.sort();

    // Print the timeline
    for entry in &timeline {
        println!(
            "{}\t{}\t{}",
            entry.timestamp, entry.filename, entry.event_type
        );
    }

    Ok(())
}
