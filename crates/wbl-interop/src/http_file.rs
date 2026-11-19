//! `.http` files (JetBrains HTTP Client, VS Code REST Client): requests separated by `###`, file
//! variables (`@name = value`), headers until a blank line, then the body (`< path` sends a
//! file). Response handlers are scripts and are not run.

use std::collections::BTreeMap;

use wbl_core::auth::Auth;
use wbl_core::{Body, BodyKind, Header, Request};

use crate::{Imported, ImportedRequest, InteropError, stored};

const METHODS: [&str; 10] = [
    "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "TRACE", "CONNECT", "QUERY",
];

#[derive(Default)]
struct Block {
    title: Option<String>,
    named: Option<String>,
    lines: Vec<String>,
}

fn comment(line: &str) -> Option<&str> {
    line.strip_prefix('#')
        .or_else(|| line.strip_prefix("//"))
        .map(str::trim)
}

/// The request blocks, and the file's `@variables`.
fn blocks(text: &str) -> (Vec<Block>, BTreeMap<String, String>) {
    let mut blocks = vec![Block::default()];
    let mut variables = BTreeMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("###") {
            let title = rest.trim();
            blocks.push(Block {
                title: (!title.is_empty()).then(|| title.to_owned()),
                ..Block::default()
            });
            continue;
        }
        let Some(current) = blocks.last_mut() else {
            continue;
        };
        let in_request = current.lines.iter().any(|l| !l.trim().is_empty());
        if !in_request {
            if let Some(c) = comment(trimmed) {
                if let Some(name) = c.strip_prefix("@name") {
                    current.named = Some(name.trim().trim_start_matches('=').trim().to_owned());
                }
                continue;
            }
            if let Some(def) = trimmed.strip_prefix('@')
                && let Some((name, value)) = def.split_once('=')
            {
                variables.insert(name.trim().to_owned(), value.trim().to_owned());
                continue;
            }
            if trimmed.is_empty() {
                continue;
            }
        }
        current.lines.push(line.to_owned());
    }
    (
        blocks
            .into_iter()
            .filter(|b| b.lines.iter().any(|l| !l.trim().is_empty()))
            .collect(),
        variables,
    )
}

/// `GET https://x HTTP/1.1` → method and URL (a bare URL is a GET).
fn request_line(line: &str) -> (String, String) {
    let mut parts = line.split_whitespace();
    let first = parts.next().unwrap_or_default();
    let (method, rest): (String, Vec<&str>) =
        if METHODS.contains(&first.to_ascii_uppercase().as_str()) {
            (first.to_ascii_uppercase(), parts.collect())
        } else {
            ("GET".into(), std::iter::once(first).chain(parts).collect())
        };
    let url: Vec<&str> = rest
        .into_iter()
        .filter(|p| !p.starts_with("HTTP/"))
        .collect();
    (method, url.join(" "))
}

fn body_of(lines: &[String], headers: &[Header], warnings: &mut Vec<String>) -> Body {
    let mut text: Vec<&str> = Vec::new();
    for line in lines {
        if line.trim_start().starts_with("> {%") || line.trim_start().starts_with(">>") {
            push_once(
                warnings,
                "Response handlers and output redirects are not run",
            );
            break;
        }
        text.push(line);
    }
    let body = text.join("\n").trim().to_owned();
    if body.is_empty() {
        return Body::None;
    }
    if let Some(path) = body.strip_prefix('<')
        && !body.contains('\n')
    {
        return Body::File {
            path: path.trim().to_owned(),
        };
    }
    let content_type = headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("content-type"))
        .map(|h| h.value.to_ascii_lowercase())
        .unwrap_or_default();
    let kind = if content_type.contains("json") || body.starts_with(['{', '[']) {
        BodyKind::Json
    } else if content_type.contains("xml") || body.starts_with('<') {
        BodyKind::Xml
    } else {
        BodyKind::Text
    };
    Body::Text { kind, text: body }
}

fn push_once(warnings: &mut Vec<String>, warning: &str) {
    if !warnings.iter().any(|w| w == warning) {
        warnings.push(warning.to_owned());
    }
}

/// A request block: the request line (with `?…`/`&…` continuation lines), headers, body.
fn request_of(block: &Block, warnings: &mut Vec<String>) -> ImportedRequest {
    let mut lines = block
        .lines
        .iter()
        .skip_while(|l| l.trim().is_empty())
        .peekable();
    let (method, mut url) = request_line(lines.next().map(String::as_str).unwrap_or_default());
    while let Some(next) = lines.peek() {
        let t = next.trim();
        if t.starts_with('?') || t.starts_with('&') {
            url.push_str(t);
            lines.next();
        } else {
            break;
        }
    }
    let mut headers = Vec::new();
    for line in lines.by_ref() {
        let t = line.trim();
        if t.is_empty() {
            break;
        }
        if comment(t).is_some() {
            continue;
        }
        if let Some((name, value)) = t.split_once(':') {
            headers.push(Header {
                name: name.trim().to_owned(),
                value: value.trim().to_owned(),
                enabled: true,
            });
        }
    }
    let rest: Vec<String> = lines.cloned().collect();
    let body = body_of(&rest, &headers, warnings);
    let name = block
        .named
        .clone()
        .or_else(|| block.title.clone())
        .unwrap_or_else(|| format!("{method} {url}"));
    ImportedRequest {
        folder: Vec::new(),
        stored: stored(
            &name,
            Request {
                method,
                url,
                headers,
                body,
            },
        ),
    }
}

/// Reads a `.http` file; `name` (the file name) names the import.
pub fn read(name: &str, text: &str) -> Result<Imported, InteropError> {
    let (blocks, variables) = blocks(text);
    let mut warnings = Vec::new();
    let requests: Vec<ImportedRequest> = blocks
        .iter()
        .map(|b| request_of(b, &mut warnings))
        .collect();
    if requests.is_empty() {
        return Err(InteropError::Empty);
    }
    let stem = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let stem = stem
        .strip_suffix(".http")
        .or_else(|| stem.strip_suffix(".rest"))
        .unwrap_or(stem);
    Ok(Imported {
        name: stem.to_owned(),
        auth: Auth::Inherit,
        variables,
        folders: Vec::new(),
        requests,
        environments: Vec::new(),
        warnings,
    })
}
