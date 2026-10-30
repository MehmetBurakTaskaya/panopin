#!/bin/bash
# An update that installs by itself on a Mac (D-076). A local server offers version 9.9.9: the
# app's own update archive with a marker file added, signed again ad hoc and signed with the run's
# update key for that version. Panopin, started from Applications with the shelf closed and no
# window in front, has to install it and start again from the new bundle.
# Usage: bash tests/macos/update.sh <Panopin.app> <update archive> <update key file> <output folder>
# Exits with 1 when a check failed.
set -u
APP=${1:?usage: update.sh <Panopin.app> <update archive> <key file> <output folder>}
ARCHIVE=${2:?usage: update.sh <Panopin.app> <update archive> <key file> <output folder>}
KEY=${3:?usage: update.sh <Panopin.app> <update archive> <key file> <output folder>}
OUT=${4:?usage: update.sh <Panopin.app> <update archive> <key file> <output folder>}
ID=com.mehmetburaktaskaya.panopin
DB="$HOME/Library/Application Support/Panopin/data.db"
LOG="$HOME/Library/Logs/$ID/Panopin.log"
WORK="$OUT/update"
RESULTS="$OUT/update-results.txt"
PORT=8767
mkdir -p "$WORK/serve" "$WORK/unpacked"
: > "$RESULTS"
failed=0

pass() { echo "PASS $1" | tee -a "$RESULTS"; }
fail() { echo "FAIL $1${2:+ ($2)}" | tee -a "$RESULTS"; failed=1; }
check() { local name=$1 detail=$2; shift 2; if "$@"; then pass "$name"; else fail "$name" "$detail"; fi; }
wait_for() {
  local end=$((SECONDS + $1)); shift
  until "$@" > /dev/null 2>&1; do [ "$SECONDS" -ge "$end" ] && return 1; sleep 0.5; done
}
pid() { pgrep -x panopin | head -n 1; }
running() { pgrep -x panopin > /dev/null; }
stopped() { ! running; }
restarted() { local now; now=$(pid); [ -n "$now" ] && [ "$now" != "$1" ]; }
marked() { [ -f "$APP/Contents/Resources/update-marker.txt" ]; }
logged() { tail -c "+$((log_at + 1))" "$LOG" 2>/dev/null | grep -q "$1"; }

# The update: the same app with a marker inside, its seal renewed so macOS still runs it.
tar -xzf "$ARCHIVE" -C "$WORK/unpacked"
echo "update $RANDOM" > "$WORK/unpacked/Panopin.app/Contents/Resources/update-marker.txt"
codesign --force --deep --sign - --preserve-metadata=identifier,entitlements,requirements,flags,runtime "$WORK/unpacked/Panopin.app"
(cd "$WORK/unpacked" && tar -czf "$WORK/serve/Panopin.app.tar.gz" Panopin.app)
npx tauri signer sign -f "$KEY" -p "" --app-version 9.9.9 "$WORK/serve/Panopin.app.tar.gz" > /dev/null
sig=$(tr -d '\r\n' < "$WORK/serve/Panopin.app.tar.gz.sig")
url="http://127.0.0.1:$PORT/Panopin.app.tar.gz"
cat > "$WORK/serve/latest.json" << EOF
{
  "version": "9.9.9",
  "notes": "Test update",
  "pub_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "platforms": {
    "darwin-aarch64": { "signature": "$sig", "url": "$url" },
    "darwin-x86_64": { "signature": "$sig", "url": "$url" }
  }
}
EOF
python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$WORK/serve" 2> "$WORK/server.log" &
server=$!
sleep 1

pkill -x panopin
wait_for 15 stopped
# A pending first-run window would count as Panopin in use.
[ -f "$DB" ] && sqlite3 "$DB" "UPDATE settings SET value = json_set(value, '\$.onboarded', json('true')) WHERE key = 'app';"
log_at=$(wc -c < "$LOG" 2> /dev/null || echo 0)
open "$APP"
wait_for 30 running
first=$(pid)
check "Panopin starts from Applications" "" [ -n "$first" ]
# 30 s to the first check, a minute out of use, the download and the restart.
check "it installs 9.9.9 by itself and starts again" "pid $first, now $(pid)" wait_for 300 restarted "$first"
check "Applications now holds the update's bundle" "" marked
check "the new bundle's signature is intact" "$(codesign --verify --deep --strict "$APP" 2>&1 | head -2 | tr '\n' ' ')" codesign --verify --deep --strict "$APP"
check "the log names the update and its install" "" logged "installing update 9.9.9"
check "it fetched latest.json and the archive from the update server" "" grep -q "GET /Panopin.app.tar.gz" "$WORK/server.log"

pkill -x panopin
kill "$server" 2> /dev/null
cp "$LOG" "$OUT/update-Panopin.log" 2> /dev/null
echo "$(grep -c '^PASS' "$RESULTS") passed, $(grep -c '^FAIL' "$RESULTS") failed" | tee -a "$RESULTS"
exit "$failed"
