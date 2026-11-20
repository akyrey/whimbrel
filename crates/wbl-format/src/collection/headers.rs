//! `[[headers]]` on folders (`_folder.toml`) and the collection (`collection.toml`): sent by
//! every request under them (D-H22). `wbl_core::inherit` lays them over a request.

use std::path::Path;

use wbl_core::Header;

use super::auth::{edit_holder, folders_around, read_holder};
use crate::FormatError;
use crate::request_file::{read_headers, write_headers};

pub fn headers_of(root: &Path, folder: &str) -> Result<Vec<Header>, FormatError> {
    read_headers(read_holder(root, folder)?.as_table())
}

/// Saves the headers of `folder` (`""`: the collection); none removes the list.
pub fn save_headers(root: &Path, folder: &str, headers: &[Header]) -> Result<(), FormatError> {
    edit_holder(root, folder, |t| write_headers(t, headers))
}

/// The headers of each folder around `request` (innermost first), then the collection's. A
/// file that cannot be read is an error: sending without its headers would go unnoticed.
pub fn headers_chain(root: &Path, request: &str) -> Result<Vec<Vec<Header>>, FormatError> {
    folders_around(request)
        .into_iter()
        .chain(std::iter::once(""))
        .map(|folder| headers_of(root, folder))
        .collect()
}
