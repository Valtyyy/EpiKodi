#!/usr/bin/env bash
# Scenario 2a (run INSIDE iso-session.sh): bus without any Secret Service.
# iso-session.sh gives an empty service directory, so org.freedesktop.secrets
# cannot be auto-activated.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
BIN="$1"
. "$HERE/common.sh"

step "2a. Bus without Secret Service (BUS=$DBUS_SESSION_BUS_ADDRESS)"
run "$BIN" status
run "$BIN" set valeur-de-test-spike
run "$BIN" get
run "$BIN" delete
run "$BIN" resolve
OPEN_MOSAIK_TMDB_API_KEY=env-test-value run "$BIN" resolve
run "$BIN" raw-get
