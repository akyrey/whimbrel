#!/usr/bin/env bash
# Makes the self-signed code-signing certificate the release workflow signs the
# macOS app with. It is not trusted by Gatekeeper (that needs a paid Apple
# Developer ID), but it gives every build the same signing identity, so macOS
# keeps the app's keychain access across updates. An ad-hoc signature changes
# with every build and makes macOS ask again.
#
# Usage: desktop/scripts/make-macos-signing-cert.sh [output-dir]
#
# Writes into output-dir (default: ./whimbrel-signing, which must not exist):
#   certificate.p12         the certificate and private key (keep it safe)
#   certificate.p12.base64  for the secret APPLE_CERTIFICATE
#   password.txt            for the secret APPLE_CERTIFICATE_PASSWORD
#
# Keep certificate.p12 and the password outside the repository. A new
# certificate is a new identity: macOS asks once more for keychain access.
set -euo pipefail

readonly COMMON_NAME="whimbrel Self-Signed"
readonly VALID_DAYS=7300 # 20 years: renewing also changes the identity
# macOS's LibreSSL writes a .p12 that `security import` reads; OpenSSL 3
# would need -legacy.
readonly OPENSSL=/usr/bin/openssl

out_dir="${1:-whimbrel-signing}"
if [[ -e "$out_dir" ]]; then
  echo "error: $out_dir already exists; pick another directory" >&2
  exit 1
fi
mkdir -m 700 "$out_dir"

config="$out_dir/openssl.cnf"
cat >"$config" <<EOF
[req]
distinguished_name = dn
prompt = no
x509_extensions = ext
[dn]
CN = $COMMON_NAME
[ext]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
EOF

"$OPENSSL" req -x509 -newkey rsa:2048 -nodes -sha256 -days "$VALID_DAYS" \
  -config "$config" -keyout "$out_dir/key.pem" -out "$out_dir/cert.pem" 2>/dev/null

password="$("$OPENSSL" rand -base64 24)"
printf '%s' "$password" >"$out_dir/password.txt"
"$OPENSSL" pkcs12 -export -name "$COMMON_NAME" \
  -inkey "$out_dir/key.pem" -in "$out_dir/cert.pem" \
  -out "$out_dir/certificate.p12" -passout "pass:$password"
base64 -i "$out_dir/certificate.p12" | tr -d '\n' >"$out_dir/certificate.p12.base64"

rm "$out_dir/key.pem" "$out_dir/cert.pem" "$config"
chmod 600 "$out_dir"/*

cat <<EOF
Done: $out_dir
Add the secrets to the repository:
  gh secret set SELF_SIGNED_CERTIFICATE < $out_dir/certificate.p12.base64
  gh secret set SELF_SIGNED_CERTIFICATE_PASSWORD < $out_dir/password.txt
Then keep $out_dir somewhere safe (a password manager) and delete it here.
EOF
