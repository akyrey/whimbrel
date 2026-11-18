//! cURL both ways (FR-2.6): a request as a command to paste in a shell, and a pasted command as
//! a request. Import covers what browsers' "Copy as cURL" and API docs use; flags it does not
//! understand are listed, not silently dropped.

use base64::Engine;

use crate::{Body, BodyKind, Header, Request, RequestSettings};

/// `text` quoted for a POSIX shell: single quotes, with `'` written as `'\''`.
fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// The request as a cURL command, one option per line.
pub fn to_curl(request: &Request) -> String {
    let has_body = matches!(request.body, Body::Text { .. });
    let mut parts = Vec::new();
    let method = if request.method == "GET" && !has_body {
        String::new()
    } else {
        format!("-X {} ", request.method)
    };
    parts.push(format!("curl {method}{}", quote(&request.url)));
    for header in request.enabled_headers() {
        parts.push(format!(
            "-H {}",
            quote(&format!("{}: {}", header.name, header.value))
        ));
    }
    if let Body::Text { text, .. } = &request.body {
        parts.push(format!("--data-raw {}", quote(text)));
    }
    parts.join(" \\\n  ")
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CurlError {
    #[error("This is not a cURL command")]
    NotCurl,
    #[error("The command has no URL")]
    NoUrl,
    #[error("A quote is not closed")]
    Unterminated,
}

/// A request read from a command, its settings, and the options that were not used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    pub request: Request,
    pub settings: RequestSettings,
    pub ignored: Vec<String>,
}

/// Splits a command into words as a POSIX shell would (quotes, backslashes, line continuations).
fn words(text: &str) -> Result<Vec<String>, CurlError> {
    let mut words = Vec::new();
    let mut word: Option<String> = None;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                let w = word.get_or_insert_with(String::new);
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => w.push(c),
                        None => return Err(CurlError::Unterminated),
                    }
                }
            }
            '"' => {
                let w = word.get_or_insert_with(String::new);
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(c @ ('"' | '\\' | '$' | '`')) => w.push(c),
                            Some('\n') => {}
                            Some(c) => {
                                w.push('\\');
                                w.push(c);
                            }
                            None => return Err(CurlError::Unterminated),
                        },
                        Some(c) => w.push(c),
                        None => return Err(CurlError::Unterminated),
                    }
                }
            }
            '\\' => match chars.next() {
                Some('\n') | None => {}
                Some(c) => word.get_or_insert_with(String::new).push(c),
            },
            c if c.is_whitespace() => {
                if let Some(w) = word.take() {
                    words.push(w);
                }
            }
            c => word.get_or_insert_with(String::new).push(c),
        }
    }
    words.extend(word);
    Ok(words)
}

/// Options that take a value but are not used.
const IGNORED_WITH_VALUE: [&str; 16] = [
    "-m",
    "--max-time",
    "-o",
    "--output",
    "--connect-timeout",
    "-w",
    "--write-out",
    "--retry",
    "-x",
    "--proxy",
    "--resolve",
    "--cacert",
    "--cert",
    "--key",
    "-F",
    "--form",
];

/// Flags that change nothing here (`-L`: redirects are followed by default).
const HARMLESS: [&str; 13] = [
    "-s",
    "--silent",
    "-S",
    "--show-error",
    "-v",
    "--verbose",
    "-i",
    "--include",
    "--compressed",
    "-L",
    "--location",
    "-f",
    "--fail",
];

#[derive(Default)]
struct Draft {
    method: Option<String>,
    url: Option<String>,
    headers: Vec<Header>,
    data: Vec<String>,
    head: bool,
    json: bool,
    settings: RequestSettings,
    ignored: Vec<String>,
}

fn header(name: &str, value: &str) -> Header {
    Header {
        name: name.to_owned(),
        value: value.to_owned(),
        enabled: true,
    }
}

/// Handles one option; returns whether it took the next word as its value.
fn option(draft: &mut Draft, flag: &str, value: Option<&str>) -> bool {
    let v = value.unwrap_or_default();
    match flag {
        "-X" | "--request" => draft.method = Some(v.to_ascii_uppercase()),
        "-H" | "--header" => {
            if let Some((name, value)) = v.split_once(':') {
                draft.headers.push(header(name.trim(), value.trim_start()));
            }
        }
        "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii" => {
            if v.starts_with('@') && flag != "--data-raw" {
                draft.ignored.push(format!("{flag} {v}"));
            } else {
                draft.data.push(v.to_owned());
            }
        }
        "--json" => {
            draft.json = true;
            draft.data.push(v.to_owned());
        }
        "-u" | "--user" => {
            let encoded = base64::engine::general_purpose::STANDARD.encode(v);
            draft
                .headers
                .push(header("Authorization", &format!("Basic {encoded}")));
        }
        "-A" | "--user-agent" => draft.headers.push(header("User-Agent", v)),
        "-b" | "--cookie" => draft.headers.push(header("Cookie", v)),
        "-e" | "--referer" => draft.headers.push(header("Referer", v)),
        "--url" => draft.url = Some(v.to_owned()),
        f if IGNORED_WITH_VALUE.contains(&f) => draft.ignored.push(format!("{f} {v}")),
        _ => return false,
    }
    true
}

fn flag(draft: &mut Draft, word: &str) {
    match word {
        "-I" | "--head" => draft.head = true,
        "-k" | "--insecure" => draft.settings.verify_tls = false,
        w if HARMLESS.contains(&w) => {}
        // Bundled short flags such as -sSL.
        w if w.len() > 2 && !w.starts_with("--") => {
            for c in w[1..].chars() {
                flag(draft, &format!("-{c}"));
            }
        }
        w => draft.ignored.push(w.to_owned()),
    }
}

fn takes_value(flag: &str) -> bool {
    matches!(
        flag,
        "-X" | "--request"
            | "-H"
            | "--header"
            | "-d"
            | "--data"
            | "--data-raw"
            | "--data-binary"
            | "--data-ascii"
            | "--json"
            | "-u"
            | "--user"
            | "-A"
            | "--user-agent"
            | "-b"
            | "--cookie"
            | "-e"
            | "--referer"
            | "--url"
    ) || IGNORED_WITH_VALUE.contains(&flag)
}

/// Reads a cURL command.
pub fn parse(text: &str) -> Result<Imported, CurlError> {
    let words = words(text.trim())?;
    let mut rest = words.iter();
    if rest.next().map(String::as_str) != Some("curl") {
        return Err(CurlError::NotCurl);
    }
    let mut draft = Draft::default();
    while let Some(word) = rest.next() {
        if word.starts_with('-') && word.len() > 1 {
            if takes_value(word) {
                option(&mut draft, word, rest.next().map(String::as_str));
            } else {
                flag(&mut draft, word);
            }
        } else if draft.url.is_none() {
            draft.url = Some(word.clone());
        } else {
            draft.ignored.push(word.clone());
        }
    }
    finish(draft)
}

fn finish(mut draft: Draft) -> Result<Imported, CurlError> {
    let url = draft.url.take().ok_or(CurlError::NoUrl)?;
    if draft.json {
        draft
            .headers
            .push(header("Content-Type", "application/json"));
        draft.headers.push(header("Accept", "application/json"));
    }
    let body = if draft.data.is_empty() {
        Body::None
    } else {
        let text = draft.data.join("&");
        let says_json = draft
            .headers
            .iter()
            .any(|h| h.name.eq_ignore_ascii_case("content-type") && h.value.contains("json"));
        let looks_json = text.trim_start().starts_with(['{', '[']);
        let kind = if says_json || looks_json {
            BodyKind::Json
        } else {
            BodyKind::Text
        };
        Body::Text { kind, text }
    };
    let method = match (&draft.method, draft.head, &body) {
        (Some(m), _, _) => m.clone(),
        (None, true, _) => "HEAD".to_owned(),
        (None, false, Body::Text { .. }) => "POST".to_owned(),
        (None, false, Body::None) => "GET".to_owned(),
    };
    Ok(Imported {
        request: Request {
            method,
            url,
            headers: draft.headers,
            body,
        },
        settings: draft.settings,
        ignored: draft.ignored,
    })
}

#[cfg(test)]
mod tests;
