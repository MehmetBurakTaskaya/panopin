#!/bin/bash
# Drives the macOS app on a GitHub macOS runner as a user would (D-073): the first start, copies,
# screenshots, the global shortcuts, the shelf, paste into TextEdit, text recognition, Settings,
# the Library and the login item. The runner allows System Events, keystrokes and screen captures
# from the shell; Panopin gets Accessibility and Desktop access through the same privacy database.
# Usage: bash tests/macos/live.sh <Panopin.app> <output folder>
# Results, screenshots, window texts and the app log go to the output folder. Exits with 1 when a
# check failed.
set -u
APP=${1:?usage: live.sh <Panopin.app> <output folder>}
OUT=${2:?usage: live.sh <Panopin.app> <output folder>}
HERE=$(cd "$(dirname "$0")" && pwd)
ID=com.mehmetburaktaskaya.panopin
DATA="$HOME/Library/Application Support/Panopin"
DB="$DATA/data.db"
RUN=$RANDOM$RANDOM
mkdir -p "$OUT/shots" "$OUT/texts"
RESULTS="$OUT/results.txt"
: > "$RESULTS"
failed=0

pass() { echo "PASS $1" | tee -a "$RESULTS"; }
fail() { echo "FAIL $1${2:+ ($2)}" | tee -a "$RESULTS"; failed=1; }
skip() { echo "SKIP $1${2:+ ($2)}" | tee -a "$RESULTS"; }
note() { echo "NOTE $*" | tee -a "$RESULTS"; }
section() { echo "--- $*" | tee -a "$RESULTS"; }
# check <name> <detail> <command...>
check() { local name=$1 detail=$2; shift 2; if "$@"; then pass "$name"; else fail "$name" "$detail"; fi; }
# wait_for <seconds> <command...>: until the command succeeds
wait_for() {
  local end=$((SECONDS + $1)); shift
  until "$@" > /dev/null 2>&1; do [ "$SECONDS" -ge "$end" ] && return 1; sleep 0.3; done
}

UI() { osascript "$HERE/ui.applescript" "$@" 2>> "$OUT/osascript.log"; }
PB="$OUT/pbtool"
sql() { sqlite3 "$DB" "$1" 2>> "$OUT/sqlite.log"; }
# setting <key> <JSON value>, with Panopin stopped
setting() { sql "UPDATE settings SET value = json_set(value, '\$.$1', json('$2')) WHERE key = 'app';"; }
shot() { screencapture -x "$OUT/shots/$1.png"; }
key() { UI key "$@" > /dev/null; sleep 0.7; }
front_is() { [ "$(UI front)" = "$1" ]; }
window_is_open() { UI windows | grep -q "$1"; }
running() { pgrep -x panopin > /dev/null; }
stopped() { ! pgrep -x panopin > /dev/null; }
start_app() { open "$APP"; wait_for 30 running; sleep 4; }
stop_app() { pkill -x panopin; wait_for 15 stopped; sleep 1; }
max_id() { sql "SELECT COALESCE(MAX(id), 0) FROM items;"; }
has_text() { [ "$(sql "SELECT COUNT(*) FROM items WHERE text_body = '$1';")" = 1 ]; }
# new_count <base id> <extra condition> <count>: exactly that many items after the base id
new_count() { [ "$(sql "SELECT COUNT(*) FROM items WHERE id > $1 $2;")" = "$3" ]; }
textedit() { open -a TextEdit "$OUT/note.txt"; wait_for 15 front_is com.apple.TextEdit; sleep 1; }
open_shelf() { key 9 "option command"; wait_for 6 front_is "$ID"; }
# Letters outside the search box are shelf actions (P pins, L opens the Library): type only into it.
search_focused() { UI focus | grep -q -E 'AXTextField|AXSearchField'; }
type_in_search() {
  if wait_for 3 search_focused; then UI type "$1" > /dev/null; else fail "the search box has the focus" "focused: $(UI focus)"; return 1; fi
}
# grant <service> <system|user>: allows Panopin a privacy service the way the runner image does
grant() {
  local now row
  now=$(date +%s)
  row="'$1','$ID',0,2,4,1,NULL,NULL,0,'UNUSED',NULL,0,$now,NULL,NULL,'UNUSED',$now"
  if [ "$2" = system ]; then
    sudo sqlite3 "/Library/Application Support/com.apple.TCC/TCC.db" "INSERT OR REPLACE INTO access VALUES($row);"
  else
    sqlite3 "$HOME/Library/Application Support/com.apple.TCC/TCC.db" "INSERT OR REPLACE INTO access VALUES($row);"
  fi
}

swiftc -O -o "$PB" "$HERE/pasteboard.swift" 2> "$OUT/swiftc.log" || { fail "the test helper builds" "$(tail -3 "$OUT/swiftc.log")"; exit 1; }
: > "$OUT/note.txt"
note "macOS $(sw_vers -productVersion), $(uname -m), Panopin $(defaults read "$APP/Contents/Info" CFBundleShortVersionString 2>/dev/null)"
note "app architectures: $(lipo -archs "$APP/Contents/MacOS/panopin" 2>&1)"
note "$(csrutil status 2>&1 | head -1)"

section "the app bundle"
check "the code signature is intact" "$(codesign --verify --deep --strict "$APP" 2>&1 | head -2 | tr '\n' ' ')" codesign --verify --deep --strict "$APP"
adhoc() { codesign -dv "$APP" 2>&1 | grep -q 'Signature=adhoc'; }
check "the app is ad-hoc signed" "" adhoc
note "Gatekeeper: $(spctl --assess --type execute -v "$APP" 2>&1 | tr '\n' ' ')"

section "privacy permissions for the test"
if grant kTCCServiceAccessibility system && grant kTCCServiceAccessibility user; then ax=1; note "Accessibility granted to Panopin"; else ax=0; note "Accessibility could not be granted"; fi
grant kTCCServiceSystemPolicyDesktopFolder user && note "Desktop folder access granted to Panopin" || note "Desktop folder access could not be granted"

section "first start"
rm -rf "$DATA"
start_app
check "Panopin starts" "" running
wait_for 20 window_is_open Welcome
sleep 2
shot first-run
UI texts Welcome > "$OUT/texts/first-run.txt"
mac_onboarding() { grep -q '⇧⌘3' "$OUT/texts/first-run.txt" && grep -q 'this Mac' "$OUT/texts/first-run.txt"; }
check "the first-run window speaks of ⇧⌘3 and this Mac" "$(head -c 200 "$OUT/texts/first-run.txt" | tr '\n' '|')" mac_onboarding
UI click Welcome Start > /dev/null
onboarded() { [ "$(sql "SELECT json_extract(value, '\$.onboarded') FROM settings WHERE key = 'app';")" = 1 ]; }
if wait_for 10 onboarded; then
  pass "Start ends the first run"
else
  fail "Start ends the first run" "set in the database instead"
  stop_app; setting onboarded true; start_app
fi
sleep 2

section "menu bar and Dock"
no_dock_icon() { ! UI dock | grep -q Panopin; }
check "no Dock icon" "Dock: $(UI dock | tr '\n' ' ')" no_dock_icon
status_item() { [ "$(UI statusitems)" -ge 1 ]; }
check "a menu bar icon" "" status_item
shot menubar-light

section "copies"
textedit
T1="PANOPIN-LIVE text $RUN"
printf '%s' "$T1" | pbcopy
check "a copied text becomes a card" "" wait_for 6 has_text "$T1"
note "source app of that copy: $(sql "SELECT COALESCE(source_app, '-') FROM items WHERE text_body = '$T1';")"
T2="PANOPIN-LIVE İstanbul ₺ ğüşöç $RUN"
printf '%s' "$T2" | pbcopy
check "Turkish letters arrive intact" "" wait_for 6 has_text "$T2"
L1="https://example.com/panopin-live-$RUN"
is_link() { [ "$(sql "SELECT kind FROM items WHERE url = '$1';")" = link ]; }
printf '%s' "$L1" | pbcopy
check "a copied address becomes a link card" "" wait_for 6 is_link "$L1"
"$PB" render "Kestane $RUN" "$OUT/ocr.png"
base=$(max_id)
"$PB" png "$OUT/ocr.png"
check "a copied PNG becomes an image card" "" wait_for 6 new_count "$base" "AND kind = 'image'" 1
IMG=$(sql "SELECT id FROM items WHERE id > $base AND kind = 'image' LIMIT 1;")
note "its size: $(sql "SELECT width || 'x' || height FROM items WHERE id = ${IMG:-0};")"
"$PB" render "TIFF $RUN" "$OUT/tiff.png"
base=$(max_id)
"$PB" tiff "$OUT/tiff.png"
tiff_kept() { [ "$(sql "SELECT media_ext FROM items WHERE id > $1 AND kind = 'image' LIMIT 1;")" = png ]; }
check "a copied TIFF becomes an image card stored as PNG" "" wait_for 6 tiff_kept "$base"
for marker in org.nspasteboard.ConcealedType org.nspasteboard.TransientType com.agilebits.onepassword; do
  base=$(max_id)
  "$PB" marked "$marker" "PANOPIN-LIVE private $marker $RUN"
  sleep 3
  check "a copy marked $marker is not kept" "" new_count "$base" "" 0
done
ocr_reads() { sql "SELECT COALESCE(ocr_text, '') FROM items WHERE id = ${IMG:-0};" | grep -q "$RUN"; }
if wait_for 90 ocr_reads; then
  pass "Vision reads the text in the copied picture"
else
  fail "Vision reads the text in the copied picture" "$(sql "SELECT ocr_state || ': ' || COALESCE(ocr_text, '') FROM items WHERE id = ${IMG:-0};")"
fi
note "text read: $(sql "SELECT COALESCE(ocr_text, '') FROM items WHERE id = ${IMG:-0};" | tr '\n' ' ')"

section "screenshots"
loc=$(defaults read com.apple.screencapture location 2>/dev/null || echo "$HOME/Desktop")
note "screenshot folder: $loc"
base=$(max_id)
key 20 "shift command"
if wait_for 25 new_count "$base" "AND kind = 'image' AND source = 'folder'" 1; then
  pass "a ⇧⌘3 screenshot becomes a card"
else
  fail "a ⇧⌘3 screenshot becomes a card" "newest files: $(ls -t "$loc" | head -3 | tr '\n' ' ')"
fi
newest=$(ls -t "$loc"/*.png 2>/dev/null | head -1)
note "newest picture there: $(basename "$newest"); screenshot mark: $(xattr -l "$newest" 2>/dev/null | grep -c kMDItemIsScreenCapture)"
base=$(max_id)
"$PB" render "Plain $RUN" "$loc/plain-$RUN.png"
sleep 4
check "another picture saved there is not kept" "" new_count "$base" "" 0
stop_app
mkdir -p "$HOME/live-watched"
sql "UPDATE settings SET value = json_set(value, '\$.watchedFolders', json_array('$HOME/live-watched')) WHERE key = 'app';"
start_app
base=$(max_id)
"$PB" render "Extra $RUN" "$HOME/live-watched/extra-$RUN.png"
check "a picture in a folder added in Settings becomes a card" "" wait_for 10 new_count "$base" "AND kind = 'image' AND source = 'folder'" 1

section "the shelf"
textedit
T3="PANOPIN-LIVE shelf $RUN"
printf '%s' "$T3" | pbcopy
wait_for 6 has_text "$T3"
if open_shelf; then pass "⌥⌘V opens the shelf in front"; else fail "⌥⌘V opens the shelf in front" "front: $(UI front)"; fi
sleep 1
shot shelf
UI texts Panopin > "$OUT/texts/shelf.txt"
check "the shelf's hints name Mac keys" "$(head -c 200 "$OUT/texts/shelf.txt" | tr '\n' '|')" grep -q '⌘Enter' "$OUT/texts/shelf.txt"
key 53
check "Esc closes it and TextEdit comes back" "front: $(UI front)" wait_for 5 front_is com.apple.TextEdit
open_shelf
key 125
key 36
if [ "$(pbpaste)" = "$T3" ]; then pass "Enter copies the text card"; else fail "Enter copies the text card" "clipboard: $(pbpaste | head -c 80)"; fi
check "and TextEdit comes back" "front: $(UI front)" wait_for 5 front_is com.apple.TextEdit
sleep 2
check "Panopin's own copy is not kept again" "" has_text "$T3"
if [ "$ax" = 1 ]; then
  textedit
  T4="PANOPIN-LIVE paste $RUN"
  printf '%s' "$T4" | pbcopy
  wait_for 6 has_text "$T4"
  "$PB" marked org.nspasteboard.ConcealedType "something else"
  open_shelf
  key 125
  key 36 command
  sleep 2
  content=$(UI textedit)
  if printf '%s' "$content" | grep -q "$T4"; then
    pass "⌘Enter pastes into TextEdit"
  else
    fail "⌘Enter pastes into TextEdit" "TextEdit: ${content:0:80}; front: $(UI front)"
    shot paste-failed
  fi
else
  skip "⌘Enter pastes into TextEdit" "Accessibility could not be granted on this runner"
fi
textedit
key 8 "option command"
wait_for 6 front_is "$ID"
sleep 1
type_in_search "PANOPIN-LIVE"
sleep 1.5
shot search
UI texts Panopin > "$OUT/texts/search.txt"
check "⌥⌘C opens the shelf with search, and typing finds the test cards" "$(grep -m1 -i result "$OUT/texts/search.txt")" grep -q -i 'results' "$OUT/texts/search.txt"
key 53
key 53
key 8 "option command"
wait_for 6 front_is "$ID"
sleep 1
type_in_search kestane
sleep 1.5
shot search-ocr
UI texts Panopin > "$OUT/texts/search-ocr.txt"
check "search finds the picture by the text read in it" "$(grep -m1 -i result "$OUT/texts/search-ocr.txt")" grep -q -i '1 result' "$OUT/texts/search-ocr.txt"
key 53
key 53
textedit
open_shelf
key 125
key 51 command
sleep 0.8
shot trash-confirm
key 36
sleep 1.2
trashed() { [ "$(sql "SELECT COUNT(*) FROM items WHERE trashed_at IS NOT NULL AND text_body LIKE 'PANOPIN-LIVE%';")" -ge 1 ]; }
untrashed() { [ "$(sql "SELECT COUNT(*) FROM items WHERE trashed_at IS NOT NULL AND text_body LIKE 'PANOPIN-LIVE%';")" = 0 ]; }
check "⌘⌫ asks first, and Enter moves the card to Trash" "" trashed
key 51
sleep 1.2
check "⌫ brings it back" "" untrashed
key 53
textedit
key 3 "control command"
sleep 4
if open_shelf; then pass "the shelf opens over a full-screen app"; else fail "the shelf opens over a full-screen app" "front: $(UI front)"; fi
sleep 1
shot fullscreen
key 53
sleep 1
key 3 "control command"
sleep 4
capturing() { pgrep -x screencapture > /dev/null; }
not_capturing() { ! pgrep -x screencapture > /dev/null; }
key 1 "option command"
check "⌥⌘S starts the system capture" "" wait_for 6 capturing
sleep 1
shot snip
key 53
wait_for 6 not_capturing

section "Settings and the Library"
UI menu Settings > /dev/null
check "Settings opens from the menu bar icon" "windows: $(UI windows | tr '\n' ' ')" wait_for 15 window_is_open Settings
sleep 2
shot settings
: > "$OUT/texts/settings.txt"
for tab in General Sources History Shortcuts Appearance Privacy Updates About; do
  UI click Settings "$tab" > /dev/null
  sleep 1.2
  { echo "== $tab"; UI texts Settings; } >> "$OUT/texts/settings.txt"
done
for want in "Open at login" "Same as macOS" "Use the macOS accent colour" "⌥⌘V"; do
  check "Settings says \"$want\"" "" grep -q "$want" "$OUT/texts/settings.txt"
done
bad=$(grep -E 'Windows|Ctrl|this PC' "$OUT/texts/settings.txt" | head -5 | tr '\n' '|')
check "no Settings text names Windows, Ctrl or this PC" "$bad" [ -z "$bad" ]
UI click Settings Sources > /dev/null
sleep 1
UI click Settings "Add a folder…" > /dev/null
panel_open() { UI windows | grep -q -x Open; }
check "Add a folder opens the macOS folder panel" "windows: $(UI windows | tr '\n' ' ')" wait_for 8 panel_open
sleep 1
shot folder-panel
key 53
sleep 1
UI close Settings > /dev/null
defaults write -g AppleAccentColor -int 1
sleep 1
UI menu Settings > /dev/null
wait_for 15 window_is_open Settings
sleep 2
UI click Settings Appearance > /dev/null
sleep 1
shot settings-orange-accent
UI close Settings > /dev/null
defaults delete -g AppleAccentColor
UI menu "Open Library" > /dev/null
check "the Library opens from the menu bar icon" "windows: $(UI windows | tr '\n' ' ')" wait_for 15 window_is_open Library
sleep 2
shot library
UI texts Library > "$OUT/texts/library.txt"
bad=$(grep -E 'Windows|Ctrl|this PC' "$OUT/texts/library.txt" | head -5 | tr '\n' '|')
check "no Library text names Windows, Ctrl or this PC" "$bad" [ -z "$bad" ]
UI close Library > /dev/null
osascript -e 'tell application "System Events" to tell appearance preferences to set dark mode to true' 2>> "$OUT/osascript.log"
sleep 2
textedit
open_shelf
sleep 1
shot shelf-dark
key 53
shot menubar-dark
osascript -e 'tell application "System Events" to tell appearance preferences to set dark mode to false' 2>> "$OUT/osascript.log"

section "open at login"
stop_app
setting launchAtLogin true
start_app
agent() { grep -l -s -i panopin "$HOME"/Library/LaunchAgents/*.plist > /dev/null; }
no_agent() { ! agent; }
check "Open at login adds a login item" "LaunchAgents: $(ls "$HOME/Library/LaunchAgents" 2>&1 | tr '\n' ' ')" wait_for 8 agent
stop_app
setting launchAtLogin false
start_app
check "turning it off removes the login item" "" wait_for 8 no_agent

section "quit"
UI menu Quit > /dev/null
check "Quit in the menu ends Panopin" "" wait_for 10 stopped

section "a first start without permissions, as a new user meets it"
tccutil reset All "$ID" > /dev/null 2>&1
rm -rf "$DATA"
start_app
wait_for 20 window_is_open Welcome
UI click Welcome Start > /dev/null
sleep 5
shot fresh-after-start
printf '%s' "PANOPIN-LIVE fresh $RUN" | pbcopy
sleep 4
shot fresh-after-copy
note "a fresh copy was kept: $(sql "SELECT COUNT(*) FROM items WHERE text_body = 'PANOPIN-LIVE fresh $RUN';")"
note "apps with windows on screen: $(osascript -e 'tell application "System Events" to get name of every process whose visible is true' 2>&1)"
stop_app

cp "$HOME/Library/Logs/$ID/Panopin.log" "$OUT/" 2>/dev/null
echo
echo "$(grep -c '^PASS' "$RESULTS") passed, $(grep -c '^FAIL' "$RESULTS") failed, $(grep -c '^SKIP' "$RESULTS") skipped" | tee -a "$RESULTS"
exit "$failed"
