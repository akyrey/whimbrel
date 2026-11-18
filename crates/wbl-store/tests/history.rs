#![allow(clippy::unwrap_used, clippy::expect_used)]

use wbl_core::{Body, Request};
use wbl_store::{History, NewEntry, Outcome};

fn request(url: &str) -> Request {
    Request {
        method: "GET".into(),
        url: url.into(),
        headers: vec![],
        body: Body::None,
    }
}

fn entry(name: &str, url: &str, outcome: Outcome) -> NewEntry {
    NewEntry {
        sent_at_ms: 1_700_000_000_000,
        collection: Some("/c".into()),
        name: name.into(),
        request: request(url),
        outcome,
    }
}

fn failed(code: &str) -> Outcome {
    Outcome::Failed {
        code: code.into(),
        message: "nope".into(),
    }
}

#[test]
fn records_lists_newest_first_and_reads_back() {
    let dir = tempfile::tempdir().unwrap();
    let history = History::open(&dir.path().join("whimbrel.db")).unwrap();

    let first = history
        .record(&entry("Ping", "http://a/ping", failed("connect")))
        .unwrap();
    let second = history
        .record(&entry("Users", "http://a/users", failed("timeout")))
        .unwrap();

    let list = history.list("", 10, 0).unwrap();
    assert_eq!(
        list.iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![second, first]
    );
    assert_eq!(list[0].method, "GET");
    assert_eq!(list[0].error_code.as_deref(), Some("timeout"));
    let read = history.get(first).unwrap().unwrap();
    assert_eq!(read.request, request("http://a/ping"));
    assert_eq!(read.outcome, failed("connect"));
    assert_eq!(history.get(9999).unwrap(), None);
}

#[test]
fn search_matches_name_url_method_and_status() {
    let dir = tempfile::tempdir().unwrap();
    let history = History::open(&dir.path().join("h.db")).unwrap();
    history
        .record(&entry("Login", "http://a/auth", failed("connect")))
        .unwrap();
    history
        .record(&entry(
            "List users",
            "http://a/users?page=1",
            failed("timeout"),
        ))
        .unwrap();

    assert_eq!(history.list("user", 10, 0).unwrap().len(), 1);
    assert_eq!(history.list("LOGIN", 10, 0).unwrap().len(), 1);
    assert_eq!(
        history.list("100%", 10, 0).unwrap().len(),
        0,
        "% is not a wildcard"
    );
    assert_eq!(history.list("", 1, 1).unwrap()[0].name, "Login", "paged");
}

#[test]
fn keeps_only_the_newest_entries_and_can_be_cleared() {
    let dir = tempfile::tempdir().unwrap();
    let history = History::open(&dir.path().join("h.db"))
        .unwrap()
        .with_limit(3);
    for n in 0..5 {
        history
            .record(&entry(&format!("r{n}"), "http://a", failed("x")))
            .unwrap();
    }

    let names: Vec<String> = history
        .list("", 10, 0)
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();
    history.clear().unwrap();

    assert_eq!(names, vec!["r4", "r3", "r2"]);
    assert!(history.list("", 10, 0).unwrap().is_empty());
}

#[test]
fn reopening_keeps_the_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("h.db");
    History::open(&path)
        .unwrap()
        .record(&entry("Ping", "http://a", failed("x")))
        .unwrap();

    assert_eq!(
        History::open(&path).unwrap().list("", 10, 0).unwrap().len(),
        1
    );
}
