//! Whimbrel's collection files (FILE_FORMAT.md, D-H03): TOML read and written with `toml_edit`,
//! so a save changes only the values that changed and keeps the user's comments, formatting
//! and unknown keys.

mod error;
mod fields;
mod request_file;

pub use error::FormatError;
pub use request_file::{RequestFile, StoredRequest};

/// The newest file format this version reads and writes.
pub const FORMAT: i64 = 1;
