//! Responses as they may be shown or kept outside the app's memory (history, an AI tool): every
//! known secret value masked (D-H12), long bodies cut, binary bodies and spill files dropped.

use wbl_core::interpolate::mask;
use wbl_http::{Response, ResponseBody, ResponseHeader};

/// Body bytes kept per entry (FR-8.2).
pub const KEPT_BODY: usize = 256 * 1024;

fn cut(text: &str) -> (String, bool) {
    if text.len() <= KEPT_BODY {
        return (text.to_owned(), false);
    }
    let mut end = KEPT_BODY;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_owned(), true)
}

/// The response as kept: secrets masked in headers and body, the body cut to 256 KB, binary
/// bodies and spill files dropped (the files are gone after a restart).
pub fn masked_response(response: &Response, secrets: &[String]) -> Response {
    let (text, cut_short) = match &response.body.text {
        Some(text) => {
            let (kept, cut_short) = cut(&mask(text, secrets));
            (Some(kept), cut_short)
        }
        None => (None, false),
    };
    Response {
        headers: response
            .headers
            .iter()
            .map(|h| ResponseHeader {
                name: h.name.clone(),
                value: mask(&h.value, secrets),
            })
            .collect(),
        body: ResponseBody {
            text,
            base64: None,
            truncated: response.body.truncated || cut_short || response.body.base64.is_some(),
            file: None,
            ..response.body.clone()
        },
        url: mask(&response.url, secrets),
        ..response.clone()
    }
}

#[cfg(test)]
mod tests {
    use wbl_http::Timing;

    use super::*;

    fn response(text: Option<String>, base64: Option<String>) -> Response {
        Response {
            status: 200,
            reason: Some("OK".into()),
            http_version: "HTTP/1.1".into(),
            url: "https://x/?key=s3cret".into(),
            headers: vec![ResponseHeader {
                name: "x-echo".into(),
                value: "Bearer s3cret".into(),
            }],
            body: ResponseBody {
                text,
                base64,
                size: 10,
                truncated: false,
                file: Some("/tmp/body".into()),
                content_type: None,
            },
            timing: Timing {
                headers_ms: 1,
                total_ms: 2,
                dns_ms: None,
                connect_ms: Some(1),
            },
            stream_events: 0,
        }
    }

    #[test]
    fn kept_responses_have_secrets_masked_and_long_bodies_cut() {
        let secrets = vec!["s3cret".to_owned()];
        let long = format!("s3cret {}é", "x".repeat(KEPT_BODY));

        let kept = masked_response(&response(Some(long), None), &secrets);
        let binary = masked_response(&response(None, Some("AAAA".into())), &secrets);

        assert_eq!(kept.url, "https://x/?key=••••••");
        assert_eq!(kept.headers[0].value, "Bearer ••••••");
        let text = kept.body.text.unwrap();
        assert!(text.starts_with("••••••"));
        assert!(text.len() <= KEPT_BODY);
        assert!(kept.body.truncated);
        assert_eq!(kept.body.file, None, "spill files are gone after a restart");
        assert_eq!(binary.body.base64, None);
        assert!(binary.body.truncated);
    }
}
