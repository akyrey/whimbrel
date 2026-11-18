//! A small OAuth 2 authorization server for the tests (RFC 6749, 7636, 8628): just enough to
//! check what the client sends and to answer as a real server would.
#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use base64::Engine;
use sha2::Digest;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub const CLIENT_ID: &str = "whimbrel-test";
pub const CLIENT_SECRET: &str = "shh";

#[derive(Default)]
pub struct State {
    /// code → (code challenge, redirect uri)
    codes: HashMap<String, (String, String)>,
    device_polls: u32,
    issued: u32,
    pub token_requests: Vec<HashMap<String, String>>,
}

struct Request {
    method: String,
    target: String,
    form: HashMap<String, String>,
}

async fn read(socket: &mut TcpStream) -> Option<Request> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        let n = socket.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        data.extend_from_slice(&chunk[..n]);
        if let Some(at) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
    };
    let head = String::from_utf8_lossy(&data[..head_end]).to_string();
    let mut first = head.lines().next()?.split(' ');
    let method = first.next()?.to_owned();
    let target = first.next()?.to_owned();
    let length: usize = head
        .lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length: ")
                .map(str::to_owned)
        })
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    while data.len() < head_end + length {
        let n = socket.read(&mut chunk).await.ok()?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..n]);
    }
    let form = url::form_urlencoded::parse(&data[head_end..])
        .into_owned()
        .collect();
    Some(Request {
        method,
        target,
        form,
    })
}

fn json(status: &str, body: &serde_json::Value) -> Vec<u8> {
    let text = body.to_string();
    format!("HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}", text.len()).into_bytes()
}

fn token(state: &mut State) -> Vec<u8> {
    state.issued += 1;
    json(
        "200 OK",
        &serde_json::json!({
            "access_token": format!("access-{}", state.issued),
            "token_type": "Bearer",
            "expires_in": 3600,
            "refresh_token": format!("refresh-{}", state.issued),
            "scope": "read",
        }),
    )
}

fn error(code: &str) -> Vec<u8> {
    json(
        "400 Bad Request",
        &serde_json::json!({ "error": code, "error_description": format!("{code} (test)") }),
    )
}

fn challenge_of(verifier: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(sha2::Sha256::digest(verifier.as_bytes()))
}

fn token_endpoint(state: &mut State, form: &HashMap<String, String>) -> Vec<u8> {
    state.token_requests.push(form.clone());
    match form.get("grant_type").map(String::as_str) {
        Some("client_credentials") => {
            if form.get("client_secret").map(String::as_str) != Some(CLIENT_SECRET) {
                return error("invalid_client");
            }
            token(state)
        }
        Some("refresh_token") => {
            if form
                .get("refresh_token")
                .is_some_and(|r| r.starts_with("refresh-"))
            {
                token(state)
            } else {
                error("invalid_grant")
            }
        }
        Some("authorization_code") => {
            let Some((challenge, redirect)) = form.get("code").and_then(|c| state.codes.remove(c))
            else {
                return error("invalid_grant");
            };
            let verifier_ok = form
                .get("code_verifier")
                .is_some_and(|v| challenge_of(v) == challenge);
            let redirect_ok = form.get("redirect_uri") == Some(&redirect);
            if verifier_ok && redirect_ok {
                token(state)
            } else {
                error("invalid_grant")
            }
        }
        Some("urn:ietf:params:oauth:grant-type:device_code") => {
            state.device_polls += 1;
            if state.device_polls < 3 {
                error("authorization_pending")
            } else {
                token(state)
            }
        }
        _ => error("unsupported_grant_type"),
    }
}

fn authorize(state: &mut State, target: &str) -> Vec<u8> {
    let query: HashMap<String, String> = url::Url::parse(&format!("http://x{target}"))
        .unwrap()
        .query_pairs()
        .into_owned()
        .collect();
    let redirect = query.get("redirect_uri").cloned().unwrap_or_default();
    if query.get("code_challenge_method").map(String::as_str) != Some("S256")
        || query.get("client_id").map(String::as_str) != Some(CLIENT_ID)
    {
        return error("invalid_request");
    }
    let code = format!("code-{}", state.codes.len() + 1);
    state.codes.insert(
        code.clone(),
        (
            query.get("code_challenge").cloned().unwrap_or_default(),
            redirect.clone(),
        ),
    );
    let location = format!(
        "{redirect}?code={code}&state={}",
        query.get("state").cloned().unwrap_or_default()
    );
    format!("HTTP/1.1 302 Found\r\nlocation: {location}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n").into_bytes()
}

/// Starts the server; returns its base URL and its state.
pub async fn start() -> (String, Arc<Mutex<State>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let state = Arc::new(Mutex::new(State::default()));
    let shared = Arc::clone(&state);
    let device_base = base.clone();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let shared = Arc::clone(&shared);
            let device_base = device_base.clone();
            tokio::spawn(async move {
                let Some(request) = read(&mut socket).await else {
                    return;
                };
                let reply = {
                    let mut state = shared.lock().unwrap();
                    match (
                        request.method.as_str(),
                        request.target.split('?').next().unwrap_or(""),
                    ) {
                        ("POST", "/token") => token_endpoint(&mut state, &request.form),
                        ("GET", "/authorize") => authorize(&mut state, &request.target),
                        ("POST", "/device") => json(
                            "200 OK",
                            &serde_json::json!({
                                "device_code": "dev-1",
                                "user_code": "WDJB-MJHT",
                                "verification_uri": format!("{device_base}/activate"),
                                "expires_in": 600,
                                "interval": 0,
                            }),
                        ),
                        _ => error("not_found"),
                    }
                };
                let _ = socket.write_all(&reply).await;
            });
        }
    });
    (base, state)
}
