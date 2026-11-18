#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;

use super::*;

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Captured {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn capture() -> (Captured, tracing::subscriber::DefaultGuard) {
    let captured = Captured::default();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_writer(captured.clone())
        .finish();
    (captured, tracing::subscriber::set_default(subscriber))
}

fn text(captured: &Captured) -> String {
    String::from_utf8(captured.0.lock().unwrap().clone()).unwrap()
}

#[tokio::test]
async fn a_command_is_logged_by_name_duration_and_error_code_only() {
    let (captured, _guard) = capture();
    let secret_url = "https://api.example.com/?key=hunter2";

    let ok = logged("request_send", async {
        Ok::<_, AppError>(secret_url.len())
    })
    .await;
    let failed = logged("request_cancel", async {
        Err::<(), _>(AppError::new(
            "invalid_request",
            format!("bad {secret_url}"),
        ))
    })
    .await;

    let log = text(&captured);
    assert_eq!(ok.unwrap(), secret_url.len());
    assert_eq!(failed.unwrap_err().code, "invalid_request");
    assert!(log.contains(r#""command":"request_send""#), "{log}");
    assert!(log.contains(r#""code":"invalid_request""#), "{log}");
    assert!(log.contains("elapsed_ms"), "{log}");
    assert!(
        !log.contains("hunter2"),
        "the log must not hold URLs or messages: {log}"
    );
}

#[test]
fn the_cold_start_is_measured_once() {
    mark_start();
    let first = ready();
    std::thread::sleep(std::time::Duration::from_millis(5));
    assert_eq!(ready(), first);
}

#[test]
fn log_files_are_daily_in_the_given_folder() {
    let dir = tempfile::tempdir().unwrap();
    let mut appender = appender(dir.path()).unwrap();

    appender.write_all(b"{}\n").unwrap();
    appender.flush().unwrap();

    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 1);
    let name = &names[0];
    assert!(
        name.starts_with("whimbrel.") && name.ends_with(".log"),
        "{name}"
    );
}

#[test]
fn init_writes_to_the_folder() {
    let dir = tempfile::tempdir().unwrap();
    let guard = init(dir.path()).unwrap();
    drop(guard);
    assert!(std::fs::read_dir(dir.path()).unwrap().next().is_some());
}
