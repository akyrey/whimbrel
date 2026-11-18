# Phase 5: Auth

Goal: requests authenticate the way APIs ask, set once on the collection or a folder and
inherited; OAuth 2 tokens are fetched, cached in the keychain and refreshed; collections can
trust their own CA and present a client certificate.

Design: [PRD.md](../PRD.md) FR-5; D-H04, D-H12.

## Tasks

- [x] 5.1 `wbl-core` auth: none, inherit, Basic, Bearer, API key (header or query), OAuth 2.
      Fields take `{{variables}}` (so credentials can be keychain secrets) and are masked like
      them. "Inherit" uses the nearest folder's auth, then the collection's, else none.
- [x] 5.2 `wbl-format`: `[auth]` in request files, `_folder.toml` and `collection.toml`, with the
      Phase 2 write rules; `[tls]` in `collection.toml` (CA file, client certificate PEM).
- [x] 5.3 `wbl-auth`: OAuth 2 client credentials, authorization code with PKCE (system browser,
      a loopback redirect on 127.0.0.1), device authorization; refresh with the refresh token;
      tokens cached in the keychain per collection and configuration. Tested against a fake
      authorization server.
- [x] 5.4 Backend: sending and previewing apply the effective auth (OAuth: the cached token,
      refreshed when expired, else `oauth_required`); commands to get, show and forget a token;
      the device code reaches the frontend while the flow waits.
- [x] 5.5 Per-collection TLS: a CA (PEM) added to the trusted roots and a client certificate
      (PEM, certificate and key) for the collection's requests.
- [x] 5.6 Frontend: an Auth tab on requests, auth for folders and the collection, OAuth "Get
      token" (browser or device code shown), token status and "Forget", the collection's TLS.

## Notes

- Tested: each grant against a fake authorization server (PKCE challenge, redirect URI, client
  secret and device polling checked by the server); a stale token refreshed on send; Basic,
  bearer and API key applied and masked; a localhost TLS server trusted only with the
  collection's CA.
- Not tested end to end: a client certificate presented to a server that requires one (the PEM
  is loaded and checked; mutual TLS itself is rustls'), and a real identity provider.
- The device flow opens the verification page in the browser and shows the code; the
  authorization code flow opens the authorization page. Both can be cancelled.
- After saving a folder's or the collection's auth, open requests read their inherited auth
  again.

## Not in this phase

- PKCS#12 client certificates: rustls in reqwest reads PEM only; convert with
  `openssl pkcs12 -in client.p12 -out client.pem -nodes`.
- A run against a real identity provider (Keycloak in a container): the fake server follows
  RFC 6749, 7636 and 8628; a real one is to be checked by hand before release.
- OAuth 2 implicit and password grants (deprecated by OAuth 2.1), AWS SigV4, Digest, NTLM.
