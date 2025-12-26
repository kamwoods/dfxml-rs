## dfxml-rs

### Digital Forensics XML (DFXML) library and tools for Rust

Early WIP prototype of a DFXML library and supporting utilities in Rust.

Core architecture:

```
dfxml-rs/
├── src/
│   ├── lib.rs
│   ├── objects/          # Hand-crafted structs mirroring Python objects.py
│   │   ├── mod.rs
│   │   ├── dfxml.rs      # DFXMLObject (root document)
│   │   ├── fileobject.rs # FileObject
│   │   ├── volume.rs     # VolumeObject
│   │   └── common.rs     # Shared types (TimestampObject, ByteRuns, etc.)
│   ├── reader.rs         # Streaming XML reader (quick-xml)
│   └── writer.rs         # XML writer
├── schema/
│   └── dfxml.xsd         # Embedded for reference/validation
└── Cargo.toml
```

