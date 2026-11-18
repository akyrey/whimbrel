#![allow(clippy::unwrap_used)]

use super::*;

fn data(events: &[StreamEvent]) -> Vec<&str> {
    events.iter().map(|e| e.data.as_str()).collect()
}

#[test]
fn sse_events_end_at_a_blank_line_and_may_span_chunks() {
    let mut parser = Parser::new(StreamKind::Sse);
    let mut events = parser.feed(b"event: tick\nid: 7\ndata: a");
    assert!(events.is_empty(), "not finished yet");
    events.extend(parser.feed(b"b\ndata: c\n\n: comment\n\ndata: d\r\n\r\n"));

    assert_eq!(data(&events), vec!["ab\nc", "d"]);
    assert_eq!(events[0].event.as_deref(), Some("tick"));
    assert_eq!(events[0].id.as_deref(), Some("7"));
    assert_eq!(events[1].event, None);
    assert_eq!(events[1].index, 1);
}

#[test]
fn json_lines_are_one_event_each_and_the_last_line_is_kept_until_the_end() {
    let mut parser = Parser::new(StreamKind::JsonLines);
    let mut events = parser.feed(b"{\"a\":1}\n\n{\"b\"");
    events.extend(parser.feed(b":2}\n{\"c\":3}"));
    events.extend(parser.finish());

    assert_eq!(
        data(&events),
        vec![r#"{"a":1}"#, r#"{"b":2}"#, r#"{"c":3}"#]
    );
}

#[test]
fn a_character_split_across_chunks_is_kept_whole() {
    let mut parser = Parser::new(StreamKind::JsonLines);
    let bytes = "\"é\"\n".as_bytes();
    let mut events = parser.feed(&bytes[..2]);
    events.extend(parser.feed(&bytes[2..]));
    assert_eq!(data(&events), vec!["\"é\""]);
}

#[test]
fn kinds_come_from_the_content_type() {
    assert_eq!(
        StreamKind::of(Some("text/event-stream; charset=utf-8")),
        Some(StreamKind::Sse)
    );
    assert_eq!(
        StreamKind::of(Some("application/x-ndjson")),
        Some(StreamKind::JsonLines)
    );
    assert_eq!(
        StreamKind::of(Some("application/jsonl")),
        Some(StreamKind::JsonLines)
    );
    assert_eq!(StreamKind::of(Some("application/json")), None);
    assert_eq!(StreamKind::of(None), None);
}
