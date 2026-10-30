# Privacy permissions for the macOS tests (D-073, D-078), sourced by live.sh, update.sh and
# permission.sh. System Settings stores a permission together with the designated requirement of
# the app it was given to, and macOS lets a later build use it only while that build satisfies the
# requirement. grant stores a permission the same way, through the privacy database, which the
# runners let the shell change.

# designated <app>: the app's designated requirement, as `codesign -d -r-` prints it
designated() { codesign -d -r- "$1" 2>&1 | sed -n 's/^#* *designated => //p'; }

# grant <service> <system|user> <bundle id> [<requirement>]: allows the app that service, bound to
# the requirement; without one, to any code with that bundle id, as the runner image grants it
grant() {
  local now req=NULL blob row
  now=$(date +%s)
  if [ -n "${4:-}" ]; then
    blob=$(mktemp)
    csreq -r "=$4" -b "$blob" || { rm -f "$blob"; return 1; }
    req="X'$(xxd -p "$blob" | tr -d '\n')'"
    rm -f "$blob"
  fi
  row="'$1','$3',0,2,4,1,$req,NULL,0,'UNUSED',NULL,0,$now,NULL,NULL,'UNUSED',$now"
  if [ "$2" = system ]; then
    sudo sqlite3 "/Library/Application Support/com.apple.TCC/TCC.db" "INSERT OR REPLACE INTO access VALUES($row);"
  else
    sqlite3 "$HOME/Library/Application Support/com.apple.TCC/TCC.db" "INSERT OR REPLACE INTO access VALUES($row);"
  fi
}

# reread: restarts the privacy daemons, so a permission changed behind their back takes effect at
# once (launchd starts them again when asked)
reread() { sudo killall tccd 2> /dev/null; sleep 1; }

# tcc_log <minutes> <text> <file>: the privacy daemons' messages that mention the text
tcc_log() { log show --last "${1}m" --style compact --predicate "subsystem == \"com.apple.TCC\" AND eventMessage CONTAINS \"$2\"" > "$3" 2>&1; }
