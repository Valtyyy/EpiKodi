#!/usr/bin/env bash
# Scenario 3 (run INSIDE an isolated bus, see iso-session.sh): locked keyring.
# A. Creates a login keyring protected by a TEST password and stores the test entry.
# B. Locks the collection through org.freedesktop.Secret.Service.Lock.
# C. Stops the daemon it started, then starts a new one WITHOUT --unlock:
#    the login keyring must stay locked (boot / autologin-like state without a password).
# Every daemon started here is a child of this session and is stopped by PID.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
BIN="$1"
KEYRING_PW="testpw"   # throw-away password, only inside the isolated bus
. "$HERE/common.sh"

# PID of the process that owns org.freedesktop.secrets, retried until the name is registered.
owner_pid() {
  local i p
  for i in $(seq 1 50); do
    p=$(dbus-send --session --print-reply --dest=org.freedesktop.DBus / \
          org.freedesktop.DBus.GetConnectionUnixProcessID string:org.freedesktop.secrets 2>/dev/null \
          | awk '/uint32/ {print $2}')
    if [ -n "$p" ]; then echo "$p"; return 0; fi
    sleep 0.1
  done
  return 1
}

step "A. Create login keyring with test password (isolated bus), daemon unlocked"
echo "$KEYRING_PW" | timeout 15 gnome-keyring-daemon --unlock --components=secrets
echo "unlock-exit=$?"
DPID=$(owner_pid) || { echo "ABORT: org.freedesktop.secrets has no owner"; exit 1; }
echo "daemon PID (started by this scenario): $DPID"
run "$BIN" status
run "$BIN" set valeur-de-test-spike

step "B. Lock the login collection with org.freedesktop.Secret.Service.Lock (no password given)"
run dbus-send --session --print-reply --dest=org.freedesktop.secrets /org/freedesktop/secrets org.freedesktop.Secret.Service.Lock array:objpath:/org/freedesktop/secrets/collection/login
run "$BIN" status
run "$BIN" get
run "$BIN" resolve
run "$BIN" raw-get
OPEN_MOSAIK_TMDB_API_KEY=env-test-value run "$BIN" resolve

step "C. Stop that daemon (PID $DPID), start a new one WITHOUT --unlock: keyring stays locked at boot"
kill "$DPID"
for i in $(seq 1 50); do kill -0 "$DPID" 2>/dev/null || break; sleep 0.1; done
echo "old daemon PID $DPID alive after stop? $(kill -0 "$DPID" 2>/dev/null && echo yes || echo no)"
gnome-keyring-daemon --foreground --components=secrets >/dev/null 2>&1 &
NPID=$!
NOWNER=$(owner_pid) || { echo "ABORT: new daemon did not register org.freedesktop.secrets"; kill "$NPID"; exit 1; }
echo "new daemon PID $NPID, owner of org.freedesktop.secrets: $NOWNER"
run "$BIN" status
run "$BIN" get
run "$BIN" resolve
run "$BIN" raw-get
run "$BIN" set autre-valeur
OPEN_MOSAIK_TMDB_API_KEY=env-test-value run "$BIN" resolve

step "cleanup (PIDs started by this scenario)"
kill "$NPID" 2>/dev/null
for i in $(seq 1 50); do kill -0 "$NPID" 2>/dev/null || break; sleep 0.1; done
echo "remaining daemon PIDs of this scenario: $(kill -0 "$NPID" 2>/dev/null && echo "$NPID" || echo none)"
