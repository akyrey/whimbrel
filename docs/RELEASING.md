# Releasing

## One-time setup (not done yet)

1. The updater key pair is at `~/.tauri/whimbrel-updater.key` (no password) and its public key is
   in `apps/desktop/src-tauri/tauri.conf.json`. **Back up the private key now.** Then add it as a
   repository secret: `gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/whimbrel-updater.key`.
2. Make the self-signed certificate below (or set the Developer ID secrets) and set its secrets.
3. Acceptance for Phase 8: tag `v0.1.0`, publish the draft release, install it on macOS and
   Linux; then tag `v0.1.1`, publish, and check that the installed copies update themselves.

## A release

1. Run `scripts/release.sh X.Y.Z` on a clean `main` (needs `git-cliff`: `brew install git-cliff`).
   It writes the `## [X.Y.Z] - date` section of `CHANGELOG.md` and sets the version in the
   workspace `Cargo.toml`, `Cargo.lock` and `apps/desktop/package.json` (the `packaging` test
   fails if they differ, or if the version has no changelog section).
   - Notes written by hand under `## [Unreleased]` become the section as they are.
   - With none, git-cliff (`cliff.toml`) drafts it from the conventional commits since the last
     tag: `feat` under Added, `perf` under Changed, `fix` under Fixed; other types are left out.
     Commit subjects are terse, so reword the draft for users where it helps.
2. Review `CHANGELOG.md`, then commit (`chore: release X.Y.Z`), tag `vX.Y.Z` and push both.
   `.github/workflows/release.yml` (which runs the shared `release-app.yml` from Roost, D-H09)
   first checks that the tag matches `Cargo.toml` and that `CHANGELOG.md` has a non-empty section
   for it, then builds the universal macOS app and dmg (and checks both architectures), and the
   Linux deb, rpm and AppImage for x86_64 and arm64. Then it creates a draft release with every
   package, a `SHA256SUMS` file, and that changelog section as its notes; the same notes go into
   `latest.json`, so the in-app update prompt shows them. Review the draft and publish it.

`workflow_dispatch` runs the same builds without a release; the packages are downloadable
artifacts of the run (kept 14 days).

## In-app updates

Installed copies update from the latest published release (FR-9.2). The release workflow signs the
updater bundles (the macOS `.app.tar.gz`, the AppImage, the deb and the rpm) and attaches a
`latest.json` that points at them. Set these repository secrets once:

| Secret | Value |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | The contents of the updater private key (`~/.tauri/whimbrel-updater.key`) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Its password; leave it unset for a key without one |

A tag build fails without the key. A manual run without it builds without updater bundles.
The public key is `plugins.updater.pubkey` in `tauri.conf.json`. **Back up the private key**:
without it no installed copy can update again, and replacing it means shipping the new public key
in a release users must download by hand.

To make a new key pair: `pnpm tauri signer generate -w ~/.tauri/whimbrel-updater.key`, then put the
`.pub` contents in `tauri.conf.json`.

Installed copies see a release only once it is published: `releases/latest/download/latest.json`
never resolves to a draft or a pre-release.

## Linux packages

Built on Ubuntu 22.04 runners, so they need glibc 2.35 or newer. `tauri.linux.conf.json` picks
the targets; Tauri adds the WebKitGTK and GTK dependencies to the deb and rpm. The job checks that
the binary and the desktop entry are in both packages, installs the deb, and starts the app under
Xvfb with no keyring running: it must still be running after 15 seconds.

The arm64 job uses the `ubuntu-22.04-arm` runner, which is free for public repositories. A
private repository needs an arm64 runner of its own (a larger runner, or a self-hosted one), or
remove that matrix entry.

The packages are not signed. An apt or dnf repository, which would sign them, is not set up.

## Signing and notarisation

The macOS app is always signed (FR-9.3). The release workflow picks the first of these that its
secrets allow:

| Signature | Needs | First launch | Keychain after an update |
|---|---|---|---|
| Developer ID, notarised | Apple Developer Program membership | Opens without a warning | Still trusted |
| Self-signed certificate | Nothing | Warns once | Still trusted |
| Ad hoc | Nothing | Warns once | Asks again for each stored secret |

The keychain grants access to an app by its signature's designated requirement. With a
certificate that is the bundle id and the certificate, which stay the same from build to build.
Ad hoc it is a hash of the binary, which changes with every build, so ad hoc is the fallback only.

A warned first launch: macOS says Apple could not verify Whimbrel. On macOS 15 and later, open
System Settings → Privacy & Security and click **Open Anyway** (right-click → Open no longer
does it). The in-app updater downloads without the quarantine flag, so updates do not warn again.

### Developer ID (paid)

Set these repository secrets once:

| Secret | Value |
|---|---|
| `APPLE_CERTIFICATE` | A "Developer ID Application" certificate exported as `.p12`, base64 encoded (`base64 -i cert.p12`) |
| `APPLE_CERTIFICATE_PASSWORD` | The password chosen when exporting the `.p12` |
| `APPLE_SIGNING_IDENTITY` | E.g. `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_ID` | The Apple ID email used for notarisation |
| `APPLE_PASSWORD` | An app-specific password for that Apple ID |
| `APPLE_TEAM_ID` | The ten-character team id |

The Tauri CLI imports the certificate, signs with the hardened runtime and notarises when these
are present, and the workflow checks the result with `spctl`. The build step removes empty
`APPLE_*` variables first: Tauri tries to sign whenever `APPLE_CERTIFICATE` exists, even when it
is empty. These secrets take precedence over the self-signed ones.

If the import fails with `SecKeychainItemImport: Unable to decode the provided data`, the
`APPLE_CERTIFICATE` secret is not a base64 `.p12`. Re-create it with `base64 -i cert.p12 | pbcopy`;
the export must include the private key ("Developer ID Application" with its key, from Keychain
Access → My Certificates). Also check that `APPLE_CERTIFICATE_PASSWORD` is the export password.

### Self-signed certificate (free)

Make the certificate once, on a Mac, with the system `openssl` (LibreSSL; with OpenSSL 3 add
`-legacy` to the `pkcs12` command, or `security import` cannot read the file):

```sh
cat > whimbrel-signing.cnf <<'CNF'
[req]
distinguished_name=dn
prompt=no
x509_extensions=v3
[dn]
CN=Whimbrel Self-Signed
[v3]
basicConstraints=critical,CA:false
keyUsage=critical,digitalSignature
extendedKeyUsage=critical,codeSigning
CNF
/usr/bin/openssl req -x509 -newkey rsa:2048 -nodes -days 7300 -config whimbrel-signing.cnf \
  -keyout whimbrel-signing.key -out whimbrel-signing.crt
/usr/bin/openssl pkcs12 -export -inkey whimbrel-signing.key -in whimbrel-signing.crt \
  -name "Whimbrel Self-Signed" -out whimbrel-signing.p12
base64 -i whimbrel-signing.p12 | pbcopy
```

Then set these repository secrets:

| Secret | Value |
|---|---|
| `SELF_SIGNED_CERTIFICATE` | The base64 `.p12` (on the clipboard after the last command) |
| `SELF_SIGNED_CERTIFICATE_PASSWORD` | The export password `pkcs12` asked for |

Tauri imports only Apple-issued certificates, so the workflow imports this one into a keychain of
its own and passes its SHA-1 hash as `APPLE_SIGNING_IDENTITY`. The certificate is not trusted by
anyone, and does not need to be: `codesign` signs with it, and only its identity matters.
Tauri signs the DMG with it too (an ad hoc build leaves the DMG unsigned).

**Back up the `.p12` and keep using it.** A new certificate changes the designated requirement,
so every installed copy asks for keychain access again once after the update that brings it. The
same happens once when moving to a Developer ID certificate.

### Ad hoc (no secrets)

With neither set of secrets the build signs ad hoc (`APPLE_SIGNING_IDENTITY=-`). Users must
approve the first launch as above, and after each update macOS asks again before Whimbrel can read
a stored secret. Good enough for testing; set up the self-signed certificate for releases.

The workflow checks every build with `codesign --verify --deep --strict` and prints its authority
and designated requirement; only a notarised build is also checked with `spctl`.
