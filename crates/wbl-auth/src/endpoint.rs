//! The authorization server's URLs, checked before anything is sent, and network failures
//! described with their causes (never the URL: it may hold credentials or a query).

use crate::FlowError;

/// `value` as a URL to send to, or what is wrong with it, naming the field (`label`).
pub(crate) fn endpoint(label: &str, value: &str) -> Result<url::Url, FlowError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(FlowError::Endpoint(format!("Enter the {label}")));
    }
    let lower = value.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err(FlowError::Endpoint(format!(
            "The {label} needs http:// or https://"
        )));
    }
    url::Url::parse(value)
        .map_err(|e| FlowError::Endpoint(format!("The {label} is not valid: {e}")))
}

/// A failed exchange with the server, its kind and causes joined.
pub(crate) fn unreachable(error: reqwest::Error) -> FlowError {
    let error = error.without_url();
    let mut parts = vec![error.to_string()];
    let mut source = std::error::Error::source(&error);
    while let Some(cause) = source {
        let text = cause.to_string();
        if parts.last() != Some(&text) {
            parts.push(text);
        }
        source = cause.source();
    }
    FlowError::Unreachable(parts.join(": "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_valid_url_passes_trimmed() {
        let url = endpoint("token URL", " https://idp.example/token ").unwrap();
        assert_eq!(url.as_str(), "https://idp.example/token");
    }

    #[test]
    fn the_scheme_is_checked_without_regard_to_case() {
        assert!(endpoint("token URL", "HTTPS://idp.example/token").is_ok());
        assert_eq!(
            endpoint("token URL", "ftp://idp.example")
                .unwrap_err()
                .to_string(),
            "The token URL needs http:// or https://"
        );
    }
}
