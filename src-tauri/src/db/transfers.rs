//! Portable Clip and History and Organization data movement.
//!
//! Complete SQLite snapshots and replacement restore stay in `full_backups`. Portable transfers
//! merge Pasted-owned records; credential stores and original externally referenced files remain
//! outside both payload ownership and import mutation.

mod clip_transfer;
mod library_export;
mod library_import;
mod library_validation;
