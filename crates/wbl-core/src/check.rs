//! Chaining without scripts (FR-6): captures take values out of a response into variables, and
//! assertions check it. Both work on [`Observed`], what was received, so this stays free of I/O.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json_path::JsonPath;

/// What a check looks at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Observed {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    /// The body as text; `None` when it is not text.
    pub body: Option<String>,
    pub total_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum CaptureFrom {
    Body,
    Header,
}

/// `var` gets the value at `path` (JSONPath in the body, or a header name), cut by `regex`
/// (its first group, else the whole match) when there is one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Capture {
    pub var: String,
    pub from: CaptureFrom,
    /// JSONPath for the body (`""`: the whole body), the header name for a header.
    pub path: String,
    pub regex: Option<String>,
    /// Masked wherever shown, and saved to the keychain when the environment declares it.
    pub secret: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(
    tag = "check",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum Assertion {
    Status {
        equals: u16,
    },
    /// Exists, or equals when `equals` is set.
    Header {
        name: String,
        equals: Option<String>,
    },
    /// Exists, or equals (as JSON) when `equals` is set.
    Json {
        path: String,
        #[cfg_attr(feature = "ts", ts(type = "unknown"))]
        equals: Option<Value>,
    },
    Time {
        below_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CheckResult {
    pub passed: bool,
    /// What was checked and what was found, e.g. `status 404, expected 200`.
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CaptureError {
    #[error("`{0}` is not a JSONPath ({1})")]
    BadPath(String, String),
    #[error("`{0}` is not a regular expression ({1})")]
    BadRegex(String, String),
    #[error("The body is not JSON")]
    NotJson,
    #[error("Nothing found at {0}")]
    NotFound(String),
}

fn parse_json(observed: &Observed) -> Result<Value, CaptureError> {
    let body = observed.body.as_deref().ok_or(CaptureError::NotJson)?;
    serde_json::from_str(body).map_err(|_| CaptureError::NotJson)
}

fn query(observed: &Observed, path: &str) -> Result<Option<Value>, CaptureError> {
    let parsed =
        JsonPath::parse(path).map_err(|e| CaptureError::BadPath(path.to_owned(), e.to_string()))?;
    let json = parse_json(observed)?;
    Ok(parsed.query(&json).first().cloned())
}

/// Every value at `path` in the JSON `body`, in document order (task 9.4).
pub fn select(body: &str, path: &str) -> Result<Vec<Value>, CaptureError> {
    let parsed =
        JsonPath::parse(path).map_err(|e| CaptureError::BadPath(path.to_owned(), e.to_string()))?;
    let json: Value = serde_json::from_str(body).map_err(|_| CaptureError::NotJson)?;
    Ok(parsed.query(&json).all().into_iter().cloned().collect())
}

fn header<'a>(observed: &'a Observed, name: &str) -> Option<&'a str> {
    observed
        .headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// Text of a JSON value as it goes into a variable: strings without quotes.
fn text_of(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn cut(text: &str, pattern: &str) -> Result<String, CaptureError> {
    let regex = Regex::new(pattern)
        .map_err(|e| CaptureError::BadRegex(pattern.to_owned(), e.to_string()))?;
    let found = regex
        .captures(text)
        .ok_or_else(|| CaptureError::NotFound(format!("/{pattern}/")))?;
    let matched = found
        .get(1)
        .or_else(|| found.get(0))
        .map(|m| m.as_str())
        .unwrap_or_default();
    Ok(matched.to_owned())
}

/// The value `capture` takes from `observed`.
pub fn capture(capture: &Capture, observed: &Observed) -> Result<String, CaptureError> {
    let raw = match capture.from {
        CaptureFrom::Header => header(observed, &capture.path)
            .map(str::to_owned)
            .ok_or_else(|| CaptureError::NotFound(format!("header {}", capture.path)))?,
        CaptureFrom::Body if capture.path.trim().is_empty() => {
            observed.body.clone().unwrap_or_default()
        }
        CaptureFrom::Body => query(observed, &capture.path)?
            .map(|v| text_of(&v))
            .ok_or_else(|| CaptureError::NotFound(capture.path.clone()))?,
    };
    match capture.regex.as_deref().filter(|r| !r.is_empty()) {
        Some(pattern) => cut(&raw, pattern),
        None => Ok(raw),
    }
}

fn result(passed: bool, message: String) -> CheckResult {
    CheckResult { passed, message }
}

fn json_check(observed: &Observed, path: &str, equals: Option<&Value>) -> CheckResult {
    match (query(observed, path), equals) {
        (Err(e), _) => result(false, format!("{path}: {e}")),
        (Ok(None), _) => result(false, format!("{path} is missing")),
        (Ok(Some(_)), None) => result(true, format!("{path} exists")),
        (Ok(Some(found)), Some(expected)) if found == *expected => {
            result(true, format!("{path} is {found}"))
        }
        (Ok(Some(found)), Some(expected)) => {
            result(false, format!("{path} is {found}, expected {expected}"))
        }
    }
}

/// Whether `observed` passes `assertion`.
pub fn assert(assertion: &Assertion, observed: &Observed) -> CheckResult {
    match assertion {
        Assertion::Status { equals } => {
            let ok = observed.status == *equals;
            let detail = if ok {
                String::new()
            } else {
                format!(", expected {equals}")
            };
            result(ok, format!("status {}{detail}", observed.status))
        }
        Assertion::Header { name, equals } => match (header(observed, name), equals) {
            (None, _) => result(false, format!("header {name} is missing")),
            (Some(_), None) => result(true, format!("header {name} exists")),
            (Some(v), Some(e)) if v == e => result(true, format!("header {name} is {v}")),
            (Some(v), Some(e)) => result(false, format!("header {name} is {v}, expected {e}")),
        },
        Assertion::Json { path, equals } => json_check(observed, path, equals.as_ref()),
        Assertion::Time { below_ms } => {
            let ok = observed.total_ms < *below_ms;
            result(
                ok,
                format!(
                    "{} ms{}",
                    observed.total_ms,
                    if ok {
                        String::new()
                    } else {
                        format!(", expected under {below_ms} ms")
                    }
                ),
            )
        }
    }
}
