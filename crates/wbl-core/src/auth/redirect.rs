//! A redirect URI the user registered with the authorization server, when it is one Whimbrel
//! can listen on: plain http on a loopback address (RFC 8252 §7.3), with a fixed port.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Where to listen for the authorization code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackRedirect {
    pub ip: IpAddr,
    pub port: u16,
    /// Starts with `/`; the redirect's query, if any, is not part of it.
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RedirectError {
    #[error("The redirect URI is not valid: {0}")]
    NotUrl(String),
    #[error("The redirect URI needs http:// (Whimbrel listens on this computer without TLS)")]
    NotHttp,
    #[error("The redirect URI needs localhost, 127.0.0.1 or [::1] as its host")]
    NotLoopback,
    #[error("The redirect URI needs a port other than 0")]
    NoPort,
    #[error("The redirect URI cannot have a #fragment")]
    Fragment,
}

impl RedirectError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotUrl(_) => "redirect_not_url",
            Self::NotHttp => "redirect_not_http",
            Self::NotLoopback => "redirect_not_loopback",
            Self::NoPort => "redirect_no_port",
            Self::Fragment => "redirect_fragment",
        }
    }
}

fn loopback_ip(host: url::Host<&str>) -> Option<IpAddr> {
    match host {
        // A browser tries both addresses for "localhost" and falls back to IPv4 when IPv6 is
        // refused, so one listener on 127.0.0.1 is enough.
        url::Host::Domain(name) if name.eq_ignore_ascii_case("localhost") => {
            Some(IpAddr::V4(Ipv4Addr::LOCALHOST))
        }
        url::Host::Ipv4(ip) if ip == Ipv4Addr::LOCALHOST => Some(IpAddr::V4(ip)),
        url::Host::Ipv6(ip) if ip == Ipv6Addr::LOCALHOST => Some(IpAddr::V6(ip)),
        _ => None,
    }
}

/// `uri` as an address and path to listen on, or why Whimbrel cannot receive the code there.
pub fn loopback_redirect(uri: &str) -> Result<LoopbackRedirect, RedirectError> {
    let url = url::Url::parse(uri.trim()).map_err(|e| RedirectError::NotUrl(e.to_string()))?;
    if url.scheme() != "http" {
        return Err(RedirectError::NotHttp);
    }
    if url.fragment().is_some() {
        return Err(RedirectError::Fragment);
    }
    let ip = url
        .host()
        .and_then(loopback_ip)
        .ok_or(RedirectError::NotLoopback)?;
    let port = url
        .port_or_known_default()
        .filter(|p| *p != 0)
        .ok_or(RedirectError::NoPort)?;
    Ok(LoopbackRedirect {
        ip,
        port,
        path: url.path().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn localhost_listens_on_ipv4_with_the_registered_port_and_path() {
        let target = loopback_redirect("http://localhost:8080/oauth/cb?app=1").unwrap();

        assert_eq!(
            target,
            LoopbackRedirect {
                ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
                port: 8080,
                path: "/oauth/cb".into(),
            }
        );
    }

    #[test]
    fn loopback_addresses_keep_their_family_and_a_missing_port_is_80() {
        let v6 = loopback_redirect("http://[::1]:9000").unwrap();
        let v4 = loopback_redirect(" http://127.0.0.1/callback ").unwrap();

        assert_eq!(
            (v6.ip, v6.port, v6.path.as_str()),
            (IpAddr::V6(Ipv6Addr::LOCALHOST), 9000, "/")
        );
        assert_eq!((v4.port, v4.path.as_str()), (80, "/callback"));
    }

    #[test]
    fn what_cannot_be_listened_on_is_refused_with_a_reason() {
        let cases = [
            ("not a url", "redirect_not_url"),
            ("https://localhost:8443/cb", "redirect_not_http"),
            ("https://oauth.pstmn.io/v1/callback", "redirect_not_http"),
            ("http://example.com:8080/cb", "redirect_not_loopback"),
            ("http://127.0.0.2:8080/cb", "redirect_not_loopback"),
            ("http://localhost:0/cb", "redirect_no_port"),
            ("http://localhost:8080/cb#x", "redirect_fragment"),
        ];

        for (uri, code) in cases {
            assert_eq!(loopback_redirect(uri).unwrap_err().code(), code, "{uri}");
        }
    }
}
