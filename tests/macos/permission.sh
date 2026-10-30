#!/bin/bash
# Which later build keeps a privacy permission on a Mac (D-078). A small test app asks macOS
# whether it may use Accessibility (AXIsProcessTrusted) after the permission was given to an
# earlier build and stored as System Settings stores it, bound to that build's designated
# requirement (tcc.sh). Each build has code of its own and is installed where the earlier one was,
# as an update does; it is signed with the run's certificate (APPLE_SIGNING_IDENTITY) or ad hoc.
# The ad-hoc builds show that the runner's privacy database holds to the requirement, so the update
# test's checks mean something.
# Usage: bash tests/macos/permission.sh <output folder>
# Exits with 1 when a check failed.
set -u
OUT=${1:?usage: permission.sh <output folder>}
HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/tcc.sh"
IDENTITY=${APPLE_SIGNING_IDENTITY:?the signing identity of this run}
ID=com.mehmetburaktaskaya.panopin.permission-test
APP=/Applications/PanopinPermissionTest.app
WORK="$OUT/permission"
RESULTS="$OUT/permission-results.txt"
mkdir -p "$WORK"
: > "$RESULTS"
failed=0

pass() { echo "PASS $1" | tee -a "$RESULTS"; }
fail() { echo "FAIL $1${2:+ ($2)}" | tee -a "$RESULTS"; failed=1; }
note() { echo "NOTE $*" | tee -a "$RESULTS"; }
section() { echo "--- $*" | tee -a "$RESULTS"; }
check() { local name=$1 detail=$2; shift 2; if "$@"; then pass "$name"; else fail "$name" "$detail"; fi; }
wait_for() {
  local end=$((SECONDS + $1)); shift
  until "$@" > /dev/null 2>&1; do [ "$SECONDS" -ge "$end" ] && return 1; sleep 0.3; done
}

# build <name> <identity>: the test app, whose code carries the name, so every build differs
build() {
  local app="$WORK/$1.app"
  mkdir -p "$app/Contents/MacOS"
  cat > "$app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>$ID</string>
  <key>CFBundleName</key><string>PanopinPermissionTest</string>
  <key>CFBundleExecutable</key><string>asker</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundleVersion</key><string>$1</string>
  <key>LSUIElement</key><true/>
</dict>
</plist>
EOF
  cat > "$WORK/$1.swift" << EOF
import ApplicationServices
import Foundation

let build = "$1"
let answer = AXIsProcessTrusted() ? "allowed" : "refused"
try? "\(answer) \(build)".write(toFile: CommandLine.arguments[1], atomically: true, encoding: .utf8)
EOF
  swiftc -O -o "$app/Contents/MacOS/asker" "$WORK/$1.swift" 2>> "$WORK/swiftc.log" &&
    codesign --force --sign "$2" --options runtime "$app" 2>> "$WORK/codesign.log"
}
# install <name>: that build where the previous one was
install() { rm -rf "$APP" && cp -R "$WORK/$1.app" "$APP"; }
# answer: what the installed build says when started as an app, so it answers for itself
answer() {
  rm -f "$WORK/answer"
  open -n "$APP" --args "$WORK/answer"
  wait_for 20 [ -s "$WORK/answer" ] && cat "$WORK/answer"
}
# expect <check> <answer>: the installed build gives that answer
expect() {
  local said
  said=$(answer)
  echo "${said:-nothing}" >> "$WORK/answers.txt"
  if [ "$said" = "$2" ]; then pass "$1"; else fail "$1" "it said: ${said:-nothing}"; fi
}
give() { grant kTCCServiceAccessibility system "$ID" "$1" && grant kTCCServiceAccessibility user "$ID" "$1" && reread; }
satisfies() { codesign --verify -R "=$1" "$APP" 2> /dev/null; }
fails() { ! satisfies "$1"; }

note "macOS $(sw_vers -productVersion), $(uname -m), $(csrutil status 2>&1 | head -1)"
for name in one two adhoc first second; do
  case $name in one | two) identity=$IDENTITY ;; *) identity=- ;; esac
  build "$name" "$identity" || { fail "the test app $name builds and is signed" "$(tail -3 "$WORK/swiftc.log" "$WORK/codesign.log")"; exit 1; }
done

section "builds signed with the run's certificate"
install one
REQ=$(designated "$APP")
note "build one: $(codesign -dvv "$APP" 2>&1 | grep -m1 -E '^(Authority|Signature)='); designated requirement: $REQ"
check "Accessibility is given to build one" "" give "$REQ"
expect "build one may use Accessibility" "allowed one"
install two
check "build two, other code with the same certificate, satisfies build one's requirement" "$(designated "$APP")" satisfies "$REQ"
expect "build two, installed over build one, may use Accessibility" "allowed two"

section "an ad-hoc signed build with the same permission"
install adhoc
check "an ad-hoc signed build does not satisfy that requirement" "$(designated "$APP")" fails "$REQ"
expect "and may not use Accessibility" "refused adhoc"

section "ad-hoc signed builds, as Panopin was until now"
install first
FIRST=$(designated "$APP")
note "first build's designated requirement: $FIRST"
check "Accessibility is given to the first build" "" give "$FIRST"
expect "the first build may use Accessibility" "allowed first"
install second
check "the second build does not satisfy the first one's requirement" "$(designated "$APP")" fails "$FIRST"
expect "the second build, installed over the first, may not use Accessibility" "refused second"

section "a permission stored without a requirement, as the live test gave it until now"
give ""
note "the second build then answers: $(answer)"

note "answers in order: $(tr '\n' ',' < "$WORK/answers.txt")"
sudo sqlite3 "/Library/Application Support/com.apple.TCC/TCC.db" "SELECT client, auth_value, length(csreq) FROM access WHERE client = '$ID';" > "$WORK/tcc-rows.txt" 2>&1
tcc_log 15 "$ID" "$WORK/tcc.log"
tccutil reset Accessibility "$ID" > /dev/null 2>&1
rm -rf "$APP"
echo "$(grep -c '^PASS' "$RESULTS") passed, $(grep -c '^FAIL' "$RESULTS") failed" | tee -a "$RESULTS"
exit "$failed"
