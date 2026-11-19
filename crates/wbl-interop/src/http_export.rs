//! Requests as a `.http` file. Auth becomes the header it sends (with its variables, never a
//! value); saved examples, captures and assertions have no `.http` form and are left out.

use wbl_core::auth::{Auth, KeyLocation};
use wbl_core::{Body, Header};
use wbl_format::StoredRequest;

fn auth_header(auth: &Auth) -> Option<Header> {
    let header = |name: &str, value: String| Header {
        name: name.into(),
        value,
        enabled: true,
    };
    match auth {
        Auth::Bearer { token } => Some(header("Authorization", format!("Bearer {token}"))),
        // REST Client and the JetBrains client both encode `Basic user password` themselves.
        Auth::Basic { username, password } => Some(header(
            "Authorization",
            format!("Basic {username} {password}"),
        )),
        Auth::ApiKey {
            name,
            value,
            location: KeyLocation::Header,
        } => Some(header(name, value.clone())),
        _ => None,
    }
}

fn with_key_in_query(url: &str, auth: &Auth) -> String {
    match auth {
        Auth::ApiKey {
            name,
            value,
            location: KeyLocation::Query,
        } => {
            let sep = if url.contains('?') { '&' } else { '?' };
            format!("{url}{sep}{name}={value}")
        }
        _ => url.to_owned(),
    }
}

fn body_lines(body: &Body, out: &mut Vec<String>, headers: &[Header]) {
    let has_type = headers
        .iter()
        .any(|h| h.name.eq_ignore_ascii_case("content-type"));
    match body {
        Body::None => {}
        Body::Text { text, .. } => {
            out.push(String::new());
            out.push(text.clone());
        }
        Body::File { path } => {
            out.push(String::new());
            out.push(format!("< {path}"));
        }
        Body::Form { .. } => {
            if !has_type {
                out.push("Content-Type: application/x-www-form-urlencoded".into());
            }
            out.push(String::new());
            out.push(
                body.enabled_fields()
                    .map(|f| format!("{}={}", f.name, f.value))
                    .collect::<Vec<_>>()
                    .join("&"),
            );
        }
        Body::Multipart { .. } => {
            const BOUNDARY: &str = "WhimbrelBoundary";
            out.push(format!(
                "Content-Type: multipart/form-data; boundary={BOUNDARY}"
            ));
            out.push(String::new());
            for f in body.enabled_fields() {
                out.push(format!("--{BOUNDARY}"));
                if f.file {
                    let file = f.value.rsplit('/').next().unwrap_or(&f.value);
                    out.push(format!(
                        "Content-Disposition: form-data; name=\"{}\"; filename=\"{file}\"",
                        f.name
                    ));
                    out.push(String::new());
                    out.push(format!("< {}", f.value));
                } else {
                    out.push(format!(
                        "Content-Disposition: form-data; name=\"{}\"",
                        f.name
                    ));
                    out.push(String::new());
                    out.push(f.value.clone());
                }
            }
            out.push(format!("--{BOUNDARY}--"));
        }
    }
}

/// One request as a `.http` block; `inherited` is the auth it gets from its folders.
fn block(stored: &StoredRequest, inherited: &Auth) -> String {
    let auth = if matches!(stored.auth, Auth::Inherit) {
        inherited
    } else {
        &stored.auth
    };
    let r = &stored.request;
    let mut lines = vec![
        format!("### {}", stored.name),
        format!("{} {}", r.method, with_key_in_query(&r.url, auth)),
    ];
    let headers: Vec<Header> = r
        .enabled_headers()
        .cloned()
        .chain(auth_header(auth))
        .collect();
    lines.extend(headers.iter().map(|h| format!("{}: {}", h.name, h.value)));
    body_lines(&r.body, &mut lines, &headers);
    lines.join("\n")
}

/// The requests (each with the auth it inherits) as one `.http` file.
pub fn write(requests: &[(StoredRequest, Auth)]) -> String {
    let mut text = requests
        .iter()
        .map(|(s, a)| block(s, a))
        .collect::<Vec<_>>()
        .join("\n\n");
    text.push('\n');
    text
}
