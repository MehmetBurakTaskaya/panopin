#!/bin/bash
# Code signing for the Mac app (D-078). Every build is signed with the same self-signed
# certificate, so its designated requirement names that certificate rather than one build, and
# macOS keeps the permissions given to Panopin across updates. No Apple account is involved; the
# app stays unnotarised (D-075).
#   bash scripts/mac-signing.sh new <file.p12> <days> <name>
#       makes a code signing certificate valid for that many days and saves it with its key in a
#       .p12 file, protected with the password in MACOS_CERTIFICATE_PASSWORD
#   bash scripts/mac-signing.sh import <file.p12>
#       on a Mac: puts the .p12 (password in MACOS_CERTIFICATE_PASSWORD) into a keychain of its own
#       for this run and writes APPLE_SIGNING_IDENTITY, which `tauri build` signs with, to
#       $GITHUB_ENV
set -euo pipefail
: "${MACOS_CERTIFICATE_PASSWORD:?the .p12 password}"
work=

new() {
  local out=$1 days=$2 name=$3
  umask 077
  work=$(mktemp -d)
  trap 'rm -rf "$work"' EXIT
  cat > "$work/certificate.cnf" << EOF
[req]
distinguished_name = subject
x509_extensions = codesigning
prompt = no
utf8 = yes
string_mask = utf8only

[subject]
CN = $name
O = Mehmet Burak Taşkaya

[codesigning]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
subjectKeyIdentifier = hash
EOF
  openssl req -x509 -newkey rsa:3072 -sha256 -nodes -days "$days" -config "$work/certificate.cnf" \
    -keyout "$work/key.pem" -out "$work/certificate.pem" 2> /dev/null
  # Triple DES with a SHA-1 MAC: `security import` has failed on OpenSSL 3's default, AES with a
  # SHA-256 MAC.
  openssl pkcs12 -export -inkey "$work/key.pem" -in "$work/certificate.pem" -name "$name" \
    -keypbe PBE-SHA1-3DES -certpbe PBE-SHA1-3DES -macalg sha1 \
    -passout env:MACOS_CERTIFICATE_PASSWORD -out "$out"
  openssl x509 -in "$work/certificate.pem" -noout -subject -enddate -fingerprint -sha1 -nameopt utf8
}

import() {
  local p12=$1 keychain="${RUNNER_TEMP:?}/panopin-signing.keychain-db" password line identity
  local search=()
  password=$(openssl rand -hex 16)
  security create-keychain -p "$password" "$keychain"
  security set-keychain-settings -lut 21600 "$keychain"
  security unlock-keychain -p "$password" "$keychain"
  security import "$p12" -k "$keychain" -f pkcs12 -P "$MACOS_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
  security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$password" "$keychain" > /dev/null
  # codesign finds the identity through the user's keychain search list.
  while read -r line; do search+=("${line//\"/}"); done < <(security list-keychains -d user)
  security list-keychains -d user -s "$keychain" "${search[@]}"
  # Nothing on the Mac vouches for a self-signed certificate, so the listing says
  # CSSMERR_TP_NOT_TRUSTED and finds no valid identity; codesign signs with it all the same.
  security find-identity -p codesigning "$keychain"
  identity=$(security find-identity -p codesigning "$keychain" | awk '$1 == "1)" { print $2; exit }')
  [ -n "$identity" ] || { echo "no code signing identity in $p12" >&2; exit 1; }
  echo "APPLE_SIGNING_IDENTITY=$identity" >> "${GITHUB_ENV:?}"
}

case ${1:-} in
  new) new "${2:?file.p12}" "${3:?days}" "${4:?name}" ;;
  import) import "${2:?file.p12}" ;;
  *) echo "usage: mac-signing.sh new <file.p12> <days> <name> | import <file.p12>" >&2; exit 2 ;;
esac
