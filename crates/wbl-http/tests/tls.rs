#![allow(clippy::unwrap_used, clippy::expect_used)]
//! A collection's own CA and client certificate (FR-5.3), against a TLS server on localhost
//! whose certificate comes from a CA made for the test.

use std::sync::Arc;

use rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio_util::sync::CancellationToken;
use wbl_core::{Body, Request};
use wbl_http::{Exchange, HttpErrorCode, SendOptions, Sender, TlsFiles};

struct Pki {
    ca_pem: String,
    server_cert: CertificateDer<'static>,
    server_key: PrivateKeyDer<'static>,
}

fn pki() -> Pki {
    let ca_key = KeyPair::generate().unwrap();
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca = ca_params.self_signed(&ca_key).unwrap();
    let issuer = Issuer::new(ca_params, ca_key);
    let server_key = KeyPair::generate().unwrap();
    let server = CertificateParams::new(vec!["localhost".to_owned()])
        .unwrap()
        .signed_by(&server_key, &issuer)
        .unwrap();
    Pki {
        ca_pem: ca.pem(),
        server_cert: server.der().clone(),
        server_key: PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(server_key.serialize_der())),
    }
}

async fn tls_server(pki: &Pki) -> String {
    let _ = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider().install_default();
    let config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![pki.server_cert.clone()], pki.server_key.clone_key())
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                let Ok(mut tls) = acceptor.accept(socket).await else {
                    return;
                };
                let mut buffer = [0u8; 2048];
                let _ = tls.read(&mut buffer).await;
                let _ = tls
                    .write_all(
                        b"HTTP/1.1 200 OK\r\ncontent-length: 7\r\nconnection: close\r\n\r\ntrusted",
                    )
                    .await;
                let _ = tls.shutdown().await;
            });
        }
    });
    format!("https://localhost:{port}/")
}

fn get(url: String) -> Request {
    Request {
        method: "GET".into(),
        url,
        headers: vec![],
        body: Body::None,
    }
}

#[tokio::test]
async fn the_collections_ca_is_trusted_and_others_are_not() {
    let pki = pki();
    let url = tls_server(&pki).await;
    let sender = Sender::new(SendOptions::default()).unwrap();
    let with_ca = Exchange {
        tls: TlsFiles {
            ca_pem: Some(pki.ca_pem.clone().into_bytes()),
            client_pem: None,
        },
        ..Exchange::default()
    };
    let cancel = CancellationToken::new();

    let refused = sender.send(&get(url.clone()), &cancel).await.unwrap_err();
    let trusted = sender
        .send_with(&get(url), &with_ca, &cancel, &mut |_| {})
        .await
        .unwrap();

    assert_eq!(refused.code, HttpErrorCode::Tls, "{}", refused.message);
    assert_eq!(trusted.body.text.as_deref(), Some("trusted"));
}

#[tokio::test]
async fn files_that_are_not_pem_are_explained() {
    let sender = Sender::new(SendOptions::default()).unwrap();
    let cancel = CancellationToken::new();
    let bad = |tls: TlsFiles| Exchange {
        tls,
        ..Exchange::default()
    };

    let no_cert = sender
        .send_with(
            &get("https://localhost/".into()),
            &bad(TlsFiles {
                ca_pem: Some(b"not pem".to_vec()),
                client_pem: None,
            }),
            &cancel,
            &mut |_| {},
        )
        .await
        .unwrap_err();
    let bad_identity = sender
        .send_with(
            &get("https://localhost/".into()),
            &bad(TlsFiles {
                ca_pem: None,
                client_pem: Some(b"nope".to_vec()),
            }),
            &cancel,
            &mut |_| {},
        )
        .await
        .unwrap_err();

    assert_eq!(no_cert.code, HttpErrorCode::Tls);
    assert!(no_cert.message.contains("CA file"), "{}", no_cert.message);
    assert_eq!(bad_identity.code, HttpErrorCode::Tls);
    assert!(
        bad_identity.message.contains("client certificate"),
        "{}",
        bad_identity.message
    );
}
