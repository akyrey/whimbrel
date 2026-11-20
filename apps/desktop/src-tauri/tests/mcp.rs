//! Task 12.10: an MCP client starts the built `whimbrel mcp`, creates a request in a
//! collection, sends it to a local server, and reads only protocol messages on stdout.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test code

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

/// Answers every request with `{"pong":true}`.
fn server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let body = r#"{"pong":true}"#;
            let reply = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(reply.as_bytes());
        }
    });
    base
}

struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next: u64,
}

impl Client {
    fn start(collection: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_whimbrel"))
            .args(["mcp", "--collection"])
            .arg(collection)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
            next: 1,
        }
    }

    fn write(&mut self, message: &Value) {
        writeln!(self.stdin, "{message}").unwrap();
        self.stdin.flush().unwrap();
    }

    /// Sends a request and reads lines up to its response; every line must be JSON-RPC.
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.write(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let mut line = String::new();
            assert!(
                self.stdout.read_line(&mut line).unwrap() > 0,
                "stdout closed"
            );
            let message: Value = serde_json::from_str(&line)
                .unwrap_or_else(|e| panic!("not a protocol message on stdout ({e}): {line}"));
            assert_eq!(message["jsonrpc"], "2.0", "{line}");
            if message["id"] == id {
                return message;
            }
        }
    }

    fn call(&mut self, tool: &str, arguments: Value) -> Value {
        let answer = self.request(
            "tools/call",
            json!({ "name": tool, "arguments": arguments }),
        );
        let result = &answer["result"];
        assert_ne!(result["isError"], true, "{tool}: {answer}");
        result["structuredContent"].clone()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn an_ai_tool_creates_and_sends_a_request_through_the_binary() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("collection.toml"),
        "id = \"c-e2e\"\nname = \"E2E\"\n",
    )
    .unwrap();
    let base = server();
    let mut client = Client::start(dir.path());

    let hello = client.request(
        "initialize",
        json!({ "protocolVersion": "2025-11-25", "capabilities": {},
                "clientInfo": { "name": "e2e", "version": "0" } }),
    );
    client.write(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    let env = client.call(
        "create_environment",
        json!({ "collection": "E2E", "name": "Local", "variables": { "base": base } }),
    );
    let created = client.call(
        "create_request",
        json!({ "collection": "E2E",
                "request": { "name": "Ping", "request": { "url": "{{base}}/ping" },
                             "assertions": [{ "check": "status", "equals": 200 }] } }),
    );
    let path = created["path"].as_str().unwrap();
    let sent = client.call(
        "send_request",
        json!({ "collection": "E2E", "path": path, "environment": env["id"] }),
    );

    assert_eq!(hello["result"]["serverInfo"]["name"], "whimbrel");
    assert!(
        dir.path().join(path).is_file(),
        "the file the app's watcher sees"
    );
    assert_eq!(sent["response"]["status"], 200);
    assert_eq!(sent["response"]["body"]["text"], r#"{"pong":true}"#);
    assert_eq!(sent["checks"]["assertions"][0]["passed"], true);
}
