#![allow(clippy::unwrap_used, clippy::expect_used)]

mod fake_server;

use std::time::Duration;

use tokio_util::sync::CancellationToken;
use wbl_auth::{FlowError, OAuthClient};
use wbl_core::auth::{Grant, OAuth2Config};

fn config(base: &str, grant: Grant) -> OAuth2Config {
    OAuth2Config {
        grant,
        token_url: format!("{base}/token"),
        auth_url: format!("{base}/authorize"),
        device_url: format!("{base}/device"),
        client_id: fake_server::CLIENT_ID.into(),
        client_secret: fake_server::CLIENT_SECRET.into(),
        scope: "read".into(),
        redirect_uri: String::new(),
    }
}

#[tokio::test]
async fn client_credentials_gets_a_token_that_expires() {
    let (base, _) = fake_server::start().await;

    let token = OAuthClient::new()
        .client_credentials(&config(&base, Grant::ClientCredentials))
        .await
        .unwrap();

    assert_eq!(token.access_token, "access-1");
    assert_eq!(token.refresh_token.as_deref(), Some("refresh-1"));
    let expires = token.expires_at_ms.unwrap();
    assert!(token.is_fresh(expires - 60_000 - 1));
    assert!(!token.is_fresh(expires - 30_000), "renewed a minute early");
}

#[tokio::test]
async fn a_wrong_secret_is_the_servers_error() {
    let (base, _) = fake_server::start().await;
    let wrong = OAuth2Config {
        client_secret: "nope".into(),
        ..config(&base, Grant::ClientCredentials)
    };

    let error = OAuthClient::new()
        .client_credentials(&wrong)
        .await
        .unwrap_err();

    assert!(
        matches!(&error, FlowError::Server { error, .. } if error == "invalid_client"),
        "{error}"
    );
    assert_eq!(error.code(), "oauth_error");
    assert!(error.to_string().contains("invalid_client (test)"));
}

#[tokio::test]
async fn a_refresh_token_gets_a_new_access_token() {
    let (base, state) = fake_server::start().await;
    let client = OAuthClient::new();
    let cfg = config(&base, Grant::ClientCredentials);
    let first = client.client_credentials(&cfg).await.unwrap();

    let second = client
        .refresh(&cfg, first.refresh_token.as_deref().unwrap())
        .await
        .unwrap();

    assert_eq!(second.access_token, "access-2");
    let sent = state
        .lock()
        .unwrap()
        .token_requests
        .last()
        .cloned()
        .unwrap();
    assert_eq!(
        sent.get("grant_type").map(String::as_str),
        Some("refresh_token")
    );
    assert_eq!(
        sent.get("client_id").map(String::as_str),
        Some(fake_server::CLIENT_ID)
    );
}

#[tokio::test]
async fn authorization_code_with_pkce_through_a_browser_and_a_loopback_redirect() {
    let (base, state) = fake_server::start().await;
    let browser = reqwest::Client::new();

    let token = OAuthClient::new()
        .authorization_code(
            &config(&base, Grant::AuthorizationCode),
            &CancellationToken::new(),
            |url| {
                assert!(url.contains("code_challenge_method=S256"), "{url}");
                // The "browser": follows the authorization server's redirect to the loopback.
                let browser = browser.clone();
                let url = url.to_owned();
                tokio::spawn(async move {
                    let page = browser.get(url).send().await.unwrap().text().await.unwrap();
                    assert!(page.contains("You can close this window"), "{page}");
                });
                Ok(())
            },
        )
        .await
        .unwrap();

    assert_eq!(token.access_token, "access-1");
    let sent = state
        .lock()
        .unwrap()
        .token_requests
        .last()
        .cloned()
        .unwrap();
    assert!(
        sent.get("code_verifier").is_some_and(|v| v.len() >= 43),
        "a long enough verifier"
    );
    assert!(
        sent.get("redirect_uri")
            .is_some_and(|r| r.starts_with("http://127.0.0.1:"))
    );
}

/// A port nothing listens on now (freed again before it is used).
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// A "browser" that follows the authorization server's redirect to wherever it points.
fn follow(url: &str) -> Result<(), String> {
    let url = url.to_owned();
    tokio::spawn(async move {
        let page = reqwest::get(url).await.unwrap().text().await.unwrap();
        assert!(page.contains("You can close this window"), "{page}");
    });
    Ok(())
}

#[tokio::test]
async fn authorization_code_listens_on_the_registered_loopback_redirect() {
    let (base, state) = fake_server::start().await;
    let redirect_uri = format!("http://localhost:{}/oauth/done", free_port());
    let config = OAuth2Config {
        redirect_uri: redirect_uri.clone(),
        ..config(&base, Grant::AuthorizationCode)
    };

    let token = OAuthClient::new()
        .authorization_code(&config, &CancellationToken::new(), follow)
        .await
        .unwrap();

    assert_eq!(token.access_token, "access-1");
    let sent = state
        .lock()
        .unwrap()
        .token_requests
        .last()
        .cloned()
        .unwrap();
    assert_eq!(
        sent.get("redirect_uri"),
        Some(&redirect_uri),
        "sent as registered"
    );
}

#[tokio::test]
async fn a_redirect_that_cannot_be_listened_on_is_refused_before_the_browser_opens() {
    let mut opened = false;
    let config = OAuth2Config {
        redirect_uri: "https://oauth.example/callback".into(),
        ..config("https://idp.example", Grant::AuthorizationCode)
    };

    let error = OAuthClient::new()
        .authorization_code(&config, &CancellationToken::new(), |_| {
            opened = true;
            Ok(())
        })
        .await
        .unwrap_err();

    assert_eq!(error.code(), "oauth_endpoint");
    assert!(error.to_string().contains("http://"), "{error}");
    assert!(!opened, "the browser stays closed");
}

#[tokio::test]
async fn a_registered_port_in_use_is_named() {
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port();
    let config = OAuth2Config {
        redirect_uri: format!("http://127.0.0.1:{port}/cb"),
        ..config("https://idp.example", Grant::AuthorizationCode)
    };

    let error = OAuthClient::new()
        .authorization_code(&config, &CancellationToken::new(), |_| Ok(()))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "oauth_redirect");
    assert!(error.to_string().contains(&port.to_string()), "{error}");
}

#[tokio::test]
async fn authorization_code_can_be_cancelled_while_waiting_for_the_browser() {
    let (base, _) = fake_server::start().await;
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        trigger.cancel();
    });

    let error = OAuthClient::new()
        .authorization_code(
            &config(&base, Grant::AuthorizationCode),
            &cancel,
            |_| Ok(()),
        )
        .await
        .unwrap_err();

    assert_eq!(error.code(), "cancelled");
}

#[tokio::test]
async fn device_code_shows_the_code_and_polls_until_granted() {
    let (base, state) = fake_server::start().await;
    let client = OAuthClient::new();
    let cfg = config(&base, Grant::DeviceCode);

    let device = client.device_start(&cfg).await.unwrap();
    let token = client
        .device_poll(&cfg, &device, &CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(device.user_code, "WDJB-MJHT");
    assert!(device.verification_uri.ends_with("/activate"));
    assert_eq!(token.access_token, "access-1");
    assert_eq!(
        state.lock().unwrap().token_requests.len(),
        3,
        "two pending answers, then the token"
    );
}

#[tokio::test]
async fn an_unreachable_server_is_an_http_error() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);

    let error = OAuthClient::new()
        .client_credentials(&config(&base, Grant::ClientCredentials))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "oauth_unreachable");
}

fn with_token_url(token_url: &str, grant: Grant) -> OAuth2Config {
    OAuth2Config {
        token_url: token_url.into(),
        ..config("https://idp.example", grant)
    }
}

#[tokio::test]
async fn an_empty_token_url_is_named_before_anything_is_sent() {
    let error = OAuthClient::new()
        .client_credentials(&with_token_url("  ", Grant::ClientCredentials))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "oauth_endpoint");
    assert_eq!(error.to_string(), "Enter the token URL");
}

#[tokio::test]
async fn a_token_url_without_a_scheme_says_it_needs_one() {
    let error = OAuthClient::new()
        .client_credentials(&with_token_url(
            "idp.example/token",
            Grant::ClientCredentials,
        ))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "oauth_endpoint");
    assert_eq!(error.to_string(), "The token URL needs http:// or https://");
}

#[tokio::test]
async fn an_invalid_token_url_says_why() {
    let error = OAuthClient::new()
        .client_credentials(&with_token_url("https://", Grant::ClientCredentials))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "oauth_endpoint");
    assert!(
        error
            .to_string()
            .starts_with("The token URL is not valid: "),
        "{error}"
    );
}

#[tokio::test]
async fn authorization_code_checks_the_token_url_before_opening_the_browser() {
    let mut opened = false;
    let error = OAuthClient::new()
        .authorization_code(
            &with_token_url("", Grant::AuthorizationCode),
            &CancellationToken::new(),
            |_| {
                opened = true;
                Ok(())
            },
        )
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "Enter the token URL");
    assert!(!opened, "the browser stays closed");
}

#[tokio::test]
async fn the_device_flow_names_a_missing_device_url() {
    let config = OAuth2Config {
        device_url: String::new(),
        ..config("https://idp.example", Grant::DeviceCode)
    };

    let error = OAuthClient::new().device_start(&config).await.unwrap_err();

    assert_eq!(error.to_string(), "Enter the device URL");
}
