//! Streams shown as they arrive (FR-3.4): Server-Sent Events and JSON Lines, parsed from the
//! body's chunks into events.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    Sse,
    JsonLines,
}

impl StreamKind {
    /// The stream kind a `Content-Type` announces, if any.
    pub fn of(content_type: Option<&str>) -> Option<Self> {
        let essence = content_type?.split(';').next()?.trim().to_ascii_lowercase();
        match essence.as_str() {
            "text/event-stream" => Some(Self::Sse),
            "application/jsonl"
            | "application/x-ndjson"
            | "application/x-jsonlines"
            | "application/jsonlines" => Some(Self::JsonLines),
            _ => None,
        }
    }
}

/// One event of a stream: an SSE event (its `event:` and `id:` fields, its `data:` lines
/// joined) or one JSON line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StreamEvent {
    /// 0, 1, 2… in arrival order.
    pub index: u32,
    pub event: Option<String>,
    pub id: Option<String>,
    pub data: String,
}

pub(crate) struct Parser {
    kind: StreamKind,
    /// Bytes of a character split across chunks.
    pending_bytes: Vec<u8>,
    /// Text of the line being received.
    line: String,
    /// The SSE event being built.
    event: Option<String>,
    id: Option<String>,
    data: Vec<String>,
    next_index: u32,
}

impl Parser {
    pub(crate) fn new(kind: StreamKind) -> Self {
        Self {
            kind,
            pending_bytes: Vec::new(),
            line: String::new(),
            event: None,
            id: None,
            data: Vec::new(),
            next_index: 0,
        }
    }

    /// The events `chunk` completes.
    pub(crate) fn feed(&mut self, chunk: &[u8]) -> Vec<StreamEvent> {
        self.pending_bytes.extend_from_slice(chunk);
        let valid = match std::str::from_utf8(&self.pending_bytes) {
            Ok(_) => self.pending_bytes.len(),
            Err(e) if e.error_len().is_none() => e.valid_up_to(),
            // Not text: decode what can be, replacing the rest.
            Err(_) => self.pending_bytes.len(),
        };
        let text = String::from_utf8_lossy(&self.pending_bytes[..valid]).into_owned();
        self.pending_bytes.drain(..valid);
        let mut events = Vec::new();
        for c in text.chars() {
            if c == '\n' {
                let line = std::mem::take(&mut self.line);
                events.extend(self.line_done(line.trim_end_matches('\r')));
            } else {
                self.line.push(c);
            }
        }
        events
    }

    /// The events still open when the body ends.
    pub(crate) fn finish(&mut self) -> Vec<StreamEvent> {
        let line = std::mem::take(&mut self.line);
        let mut events: Vec<StreamEvent> = self.line_done(&line).into_iter().collect();
        if self.kind == StreamKind::Sse {
            events.extend(self.dispatch());
        }
        events
    }

    fn make(&mut self, event: Option<String>, id: Option<String>, data: String) -> StreamEvent {
        let index = self.next_index;
        self.next_index += 1;
        StreamEvent {
            index,
            event,
            id,
            data,
        }
    }

    fn line_done(&mut self, line: &str) -> Option<StreamEvent> {
        match self.kind {
            StreamKind::JsonLines => {
                let trimmed = line.trim();
                (!trimmed.is_empty()).then(|| self.make(None, None, trimmed.to_owned()))
            }
            StreamKind::Sse => self.sse_line(line),
        }
    }

    fn sse_line(&mut self, line: &str) -> Option<StreamEvent> {
        if line.is_empty() {
            return self.dispatch();
        }
        if line.starts_with(':') {
            return None;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value).to_owned();
        match field {
            "data" => self.data.push(value),
            "event" => self.event = Some(value),
            "id" => self.id = Some(value),
            _ => {}
        }
        None
    }

    fn dispatch(&mut self) -> Option<StreamEvent> {
        if self.data.is_empty() {
            self.event = None;
            return None;
        }
        let data = std::mem::take(&mut self.data).join("\n");
        let event = self.event.take();
        let id = self.id.take();
        Some(self.make(event, id, data))
    }
}

#[cfg(test)]
mod tests;
