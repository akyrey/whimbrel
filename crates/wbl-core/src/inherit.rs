//! Headers set on a collection or folder that every request under it sends (FILE_FORMAT.md,
//! D-H22). Levels apply from the collection inwards; a header at a deeper level (a folder, then
//! the request) replaces every inherited header of the same name, ignoring case. A disabled one
//! turns the inherited header off.

use crate::{Header, Request};

/// `inherited` with `level` laid over it.
fn layer(inherited: &[Header], level: &[Header]) -> Vec<Header> {
    let replaced = |h: &Header| level.iter().any(|l| l.name.eq_ignore_ascii_case(&h.name));
    inherited
        .iter()
        .filter(|h| !replaced(h))
        .chain(level)
        .cloned()
        .collect()
}

/// The headers a request inherits from `chain`: its folders' (innermost first), then its
/// collection's, as `auth_chain` orders them.
pub fn inherited(chain: &[Vec<Header>]) -> Vec<Header> {
    chain
        .iter()
        .rev()
        .fold(Vec::new(), |inherited, level| layer(&inherited, level))
}

/// `request` with the headers it inherits from `chain` before its own.
pub fn with_inherited(request: &Request, chain: &[Vec<Header>]) -> Request {
    Request {
        headers: layer(&inherited(chain), &request.headers),
        ..request.clone()
    }
}

#[cfg(test)]
mod tests;
