#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use wbl_core::auth::Grant;

use super::*;

fn secrets() -> Secrets {
    Secrets::new(Arc::new(roost_keychain::MemorySecretStore::default()))
}

fn config() -> OAuth2Config {
    OAuth2Config {
        grant: Grant::ClientCredentials,
        token_url: "http://127.0.0.1/token".into(),
        auth_url: String::new(),
        device_url: String::new(),
        client_id: "app".into(),
        client_secret: String::new(),
        scope: String::new(),
        redirect_uri: String::new(),
    }
}

fn token(access: &str) -> Token {
    Token {
        access_token: access.into(),
        refresh_token: Some(format!("refresh-{access}")),
        expires_at_ms: Some(5_000),
        scope: Some("read".into()),
    }
}

fn names(tokens: &[OAuthToken]) -> Vec<&str> {
    tokens.iter().map(|t| t.name.as_str()).collect()
}

#[test]
fn added_tokens_are_named_listed_and_the_newest_is_used() {
    let s = secrets();
    let tokens = Tokens::new(&s, "c1", &config());

    let first = tokens
        .add(&Index::default(), None, &token("a"), 10)
        .unwrap();
    let second = tokens
        .add(&first, Some("  Admin "), &token("b"), 20)
        .unwrap();
    tokens.add(&second, None, &token("c"), 30).unwrap();
    let listed = tokens.list().unwrap();

    assert_eq!(names(&listed), ["Token 1", "Admin", "Token 2"]);
    assert_eq!(listed.iter().filter(|t| t.active).count(), 1);
    assert!(listed[2].active);
    assert_eq!(tokens.active().unwrap().unwrap().1.access_token, "c");
    assert_eq!(listed[1].obtained_at_ms, 20);
    assert_eq!(listed[1].scope.as_deref(), Some("read"));
    assert!(listed[1].can_refresh);
}

#[test]
fn a_token_can_be_renamed_selected_and_removed() {
    let s = secrets();
    let tokens = Tokens::new(&s, "c1", &config());
    let index = tokens
        .add(&Index::default(), None, &token("a"), 10)
        .unwrap();
    let index = tokens.add(&index, None, &token("b"), 20).unwrap();
    let (a, b) = (index.tokens[0].id.clone(), index.tokens[1].id.clone());

    tokens.rename(&a, "Reader").unwrap();
    tokens.select(&a).unwrap();
    let used = tokens.active().unwrap().unwrap();
    tokens.remove(&a).unwrap();

    assert_eq!(used, (a.clone(), token("a")));
    assert_eq!(names(&tokens.list().unwrap()), ["Token 2"]);
    assert!(
        tokens.active().unwrap().is_none(),
        "removing the token in use leaves none"
    );
    assert!(
        tokens.get(&a).unwrap().is_none(),
        "its keychain entry is gone"
    );
    assert_eq!(tokens.rename(&a, "x").unwrap_err().code, "not_found");
    assert_eq!(tokens.select(&a).unwrap_err().code, "not_found");
    tokens.select(&b).unwrap();
    assert!(tokens.active().unwrap().is_some());
}

#[test]
fn names_are_trimmed_and_must_be_short_and_not_empty() {
    assert_eq!(clean_name("  Ops ").unwrap(), "Ops");
    assert_eq!(clean_name("   ").unwrap_err().code, "invalid_argument");
    let long = "x".repeat(MAX_NAME_CHARS + 1);
    assert_eq!(clean_name(&long).unwrap_err().code, "invalid_argument");
    assert!(clean_name(&"é".repeat(MAX_NAME_CHARS)).is_ok());
}

#[test]
fn a_replaced_token_keeps_its_name_and_place() {
    let s = secrets();
    let tokens = Tokens::new(&s, "c1", &config());
    let index = tokens
        .add(&Index::default(), Some("Main"), &token("a"), 10)
        .unwrap();
    let id = index.tokens[0].id.clone();

    tokens.replace(&id, &token("fresh")).unwrap();

    assert_eq!(tokens.active().unwrap().unwrap().1.access_token, "fresh");
    assert_eq!(names(&tokens.list().unwrap()), ["Main"]);
}

#[test]
fn an_older_single_token_becomes_the_first_of_the_list() {
    let s = secrets();
    let c = config();
    s.set("c1", SLOT, &c.cache_key(), &to_json(&token("old")).unwrap())
        .unwrap();
    let tokens = Tokens::new(&s, "c1", &c);

    let listed = tokens.list().unwrap();

    assert_eq!(names(&listed), ["Token 1"]);
    assert!(listed[0].active);
    assert_eq!(tokens.active().unwrap().unwrap().1.access_token, "old");
}

#[test]
fn tokens_belong_to_their_collection_and_configuration() {
    let s = secrets();
    let other = OAuth2Config {
        scope: "admin".into(),
        ..config()
    };
    Tokens::new(&s, "c1", &config())
        .add(&Index::default(), None, &token("a"), 10)
        .unwrap();

    assert!(Tokens::new(&s, "c2", &config()).list().unwrap().is_empty());
    assert!(Tokens::new(&s, "c1", &other).list().unwrap().is_empty());
}

#[test]
fn defaults_fill_the_first_free_number_and_ids_never_clash() {
    let index = Index::default()
        .with(Entry {
            id: "a".into(),
            name: "Token 2".into(),
            obtained_at_ms: 10,
        })
        .with(Entry {
            id: format!("{:x}", 42),
            name: "Token 1".into(),
            obtained_at_ms: 42,
        });

    assert_eq!(index.default_name(), "Token 3");
    assert_eq!(index.new_id(42), format!("{:x}", 43));
    assert_eq!(
        index.without("a").unwrap().default_name(),
        "Token 2",
        "a freed number is used again"
    );
}

#[test]
fn an_unreadable_index_reads_as_no_tokens() {
    let s = secrets();
    let c = config();
    s.set("c1", SLOT, &c.cache_key(), "not json").unwrap();

    assert!(Tokens::new(&s, "c1", &c).list().unwrap().is_empty());
}
