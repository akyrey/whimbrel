//! Reading a body: kept in memory up to the inline limit; beyond it, written whole to a file in
//! the spill folder (FR-3.5), its start kept for showing. Stream events are parsed as chunks come.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::stream::{Parser, StreamEvent};
use crate::{HttpError, HttpErrorCode, SendOptions};

pub(crate) struct Collected {
    /// The start of the body (all of it when it fits).
    pub head: Vec<u8>,
    pub total: u64,
    /// The whole body, when it was written to a file.
    pub file: Option<PathBuf>,
    /// The shown bytes are not all of the body.
    pub truncated: bool,
    pub events: u32,
}

fn io_failed(error: std::io::Error) -> HttpError {
    HttpError::new(
        HttpErrorCode::Body,
        format!("The body could not be written to disk ({error})"),
    )
}

fn spill(dir: &Path, head: &[u8]) -> Result<(PathBuf, std::fs::File), HttpError> {
    std::fs::create_dir_all(dir).map_err(io_failed)?;
    let path = dir.join(format!("body-{}.bin", next_name()));
    let mut file = std::fs::File::create(&path).map_err(io_failed)?;
    file.write_all(head).map_err(io_failed)?;
    Ok((path, file))
}

/// Unique enough within one run; the folder is emptied at start.
fn next_name() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}-{n}")
}

struct Sink {
    buffer: Vec<u8>,
    file: Option<(PathBuf, std::fs::File)>,
    total: u64,
    stopped: bool,
}

pub(crate) async fn collect(
    mut response: reqwest::Response,
    options: &SendOptions,
    mut parser: Option<Parser>,
    on_event: &mut (dyn FnMut(StreamEvent) + Send),
) -> Result<Collected, HttpError> {
    let mut sink = Sink {
        buffer: Vec::new(),
        file: None,
        total: 0,
        stopped: false,
    };
    let mut events = 0;
    while let Some(chunk) = response.chunk().await.map_err(HttpError::from)? {
        if let Some(parser) = parser.as_mut() {
            for event in parser.feed(&chunk) {
                events += 1;
                on_event(event);
            }
        }
        if !take(&mut sink, &chunk, options)? {
            break;
        }
    }
    if let Some(parser) = parser.as_mut() {
        for event in parser.finish() {
            events += 1;
            on_event(event);
        }
    }
    Ok(finish(sink, options, events))
}

/// Takes a chunk; `false` when no more should be read.
fn take(sink: &mut Sink, chunk: &[u8], options: &SendOptions) -> Result<bool, HttpError> {
    let len = u64::try_from(chunk.len()).unwrap_or(u64::MAX);
    if let Some((_, file)) = sink.file.as_mut() {
        let room = options.max_file_bytes.saturating_sub(sink.total);
        let part = &chunk[..usize::try_from(room.min(len)).unwrap_or(chunk.len())];
        file.write_all(part).map_err(io_failed)?;
        sink.total += u64::try_from(part.len()).unwrap_or(0);
        sink.stopped = part.len() < chunk.len();
        return Ok(!sink.stopped);
    }
    let room = options.max_body_bytes.saturating_sub(sink.buffer.len());
    if chunk.len() <= room {
        sink.buffer.extend_from_slice(chunk);
        sink.total += len;
        return Ok(true);
    }
    match &options.spill_dir {
        Some(dir) => {
            sink.buffer.extend_from_slice(chunk);
            sink.total += len;
            sink.file = Some(spill(dir, &sink.buffer)?);
            Ok(true)
        }
        None => {
            sink.buffer.extend_from_slice(&chunk[..room]);
            sink.total += u64::try_from(room).unwrap_or(0);
            sink.stopped = true;
            Ok(false)
        }
    }
}

fn finish(mut sink: Sink, options: &SendOptions, events: u32) -> Collected {
    match sink.file.take() {
        Some((path, _)) => {
            sink.buffer.truncate(options.preview_bytes);
            Collected {
                head: sink.buffer,
                total: sink.total,
                file: Some(path),
                truncated: true,
                events,
            }
        }
        None => Collected {
            head: sink.buffer,
            total: sink.total,
            file: None,
            truncated: sink.stopped,
            events,
        },
    }
}
