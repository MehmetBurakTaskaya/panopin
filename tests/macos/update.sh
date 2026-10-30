#!/bin/bash
# An update that installs by itself on a Mac (D-076). A local server offers version 9.9.9: the
# app's own update archive with a marker file added, signed again with the run's certificate
# (APPLE_SIGNING_IDENTITY, D-078) and signed with the run's update key for that version. Panopin,
# started from Applications with the shelf closed and no window in front, has to install it, start
# again from the new bundle and keep the Accessibility and Desktop access given to the build it
# replaced. Run after live.sh, which builds the test helpers in the output folder.
# Usage: bash tests/macos/update.sh <Panopin.app> <update archive> <update key file> <output folder>
# Exits with 1 when a check failed.
set -u
APP=${1:?usage: update.sh <Panopin.app> <update archive> <key file> <output folder>}
ARCHIVE=${2:?usage: update.sh <Panopin.app> <update archive> <key file> <output folder>}
KEY=${3:?usage: update.sh <Panopin.app> <update archive> <key file> <output folder>}
OUT=${4:?usage: update.sh <Panopin.app> <update archive> <key file> <output folder>}
HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/tcc.sh"
ID=com.mehmetburaktaskaya.panopin
DB="$HOME/Library/Application Support/Panopin/data.db"
LOG="$HOME/Library/Logs/$ID/Panopin.log"
WORK="$OUT/update"
RESULTS="$OUT/update-results.txt"
PORT=8767
RUN=$RANDOM$RANDOM
mkdir -p "$WORK/serve" "$WORK/unpacked"
: > "$RESULTS"
failed=0

pass() { echo "PASS $1" | tee -a "$RESULTS"; }
fail() { echo "FAIL $1${2:+ ($2)}" | tee -a "$RESULTS"; failed=1; }
note() { echo "NOTE $*" | tee -a "$RESULTS"; }
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
# The keys and windows of live.sh, for using Panopin after the update
UI() { osascript "$HERE/ui.applescript" "$@" 2>> "$OUT/osascript.log"; }
AXT() { "$OUT/axtool" "$1" "$(pid)" "${@:2}" 2>> "$OUT/ax.log"; }
key() { UI key "$@" > /dev/null; sleep 0.7; }
front_is() { [ "$(UI front)" = "$1" ]; }
shelf_shown() { AXT shown | grep -q -x Panopin; }
sql() { sqlite3 "$DB" "$1" 2>> "$OUT/sqlite.log"; }
has_text() { [ "$(sql "SELECT COUNT(*) FROM items WHERE text_body = '$1';")" = 1 ]; }
new_screenshot() { [ "$(sql "SELECT COUNT(*) FROM items WHERE id > $1 AND kind = 'image' AND source = 'folder';")" -ge 1 ]; }
satisfies() { codesign --verify -R "=$REQ" "$APP" 2> /dev/null; }
# pastes <text>: the newest text card, ⌘Enter on the shelf, reaches TextEdit
pastes() {
  printf '%s' "$1" | pbcopy
  wait_for 6 has_text "$1" || return 1
  # The clipboard then holds something else, so only Panopin's paste can bring the text.
  "$OUT/pbtool" marked org.nspasteboard.ConcealedType "something else"
  open -a TextEdit "$WORK/note.txt"
  wait_for 15 front_is com.apple.TextEdit || return 1
  sleep 1
  key 9 "option command"
  wait_for 6 shelf_shown || return 1
  sleep 0.8
  # Down from the first image, if any, reaches the texts; below them the empty links row is skipped.
  key 125
  key 36 command
  sleep 2
  UI textedit | grep -q "$1"
}

# The update: the same app with a marker inside, its seal renewed with the same certificate, as
# the next release would be.
tar -xzf "$ARCHIVE" -C "$WORK/unpacked"
echo "update $RANDOM" > "$WORK/unpacked/Panopin.app/Contents/Resources/update-marker.txt"
codesign --force --deep --sign "${APPLE_SIGNING_IDENTITY:--}" --preserve-metadata=identifier,entitlements,requirements,flags,runtime "$WORK/unpacked/Panopin.app"
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
# The permissions go to the installed build, as a user gives them in System Settings.
REQ=$(designated "$APP")
note "installed build's designated requirement: $REQ"
note "the update's: $(designated "$WORK/unpacked/Panopin.app")"
permitted() {
  grant kTCCServiceAccessibility system "$ID" "$REQ" && grant kTCCServiceAccessibility user "$ID" "$REQ" &&
    grant kTCCServiceSystemPolicyDesktopFolder user "$ID" "$REQ" && reread
}
check "Accessibility and Desktop access are given to the installed build" "" permitted
: > "$WORK/note.txt"
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
check "the new bundle satisfies the replaced build's designated requirement" "$(designated "$APP")" satisfies

sleep 4
if [ -x "$OUT/pbtool" ] && [ -x "$OUT/axtool" ]; then
  name="⌘Enter still pastes into TextEdit, so Accessibility was kept"
  if pastes "PANOPIN-UPDATE paste $RUN"; then
    pass "$name"
  else
    fail "$name" "TextEdit: $(UI textedit | head -c 80); front: $(UI front)"
    screencapture -x "$OUT/update-paste-failed.png"
  fi
  shelf_shown && key 53
  loc=$(defaults read com.apple.screencapture location 2> /dev/null || echo "$HOME/Desktop")
  base=$(sql "SELECT COALESCE(MAX(id), 0) FROM items;")
  key 20 "shift command"
  name="a ⇧⌘3 screenshot in $loc still becomes a card, so Desktop access was kept"
  if wait_for 25 new_screenshot "$base"; then pass "$name"; else fail "$name" "newest files: $(ls -t "$loc" | head -3 | tr '\n' ' ')"; fi
else
  fail "the test helpers from live.sh are there" "$OUT/pbtool, $OUT/axtool"
fi
screencapture -x "$OUT/update-after.png"
tcc_log 15 "$ID" "$WORK/tcc.log"

pkill -x panopin
kill "$server" 2> /dev/null
cp "$LOG" "$OUT/update-Panopin.log" 2> /dev/null
echo "$(grep -c '^PASS' "$RESULTS") passed, $(grep -c '^FAIL' "$RESULTS") failed" | tee -a "$RESULTS"
exit "$failed"
