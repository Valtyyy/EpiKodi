#!/usr/bin/env bash
# Runs every scenario of spike 2.6 and prints the raw results (stdout + stderr + exit + duration).
# Scenario 1 touches the REAL session bus, with the test entry only (open-mosaik-spike), and deletes it.
# Scenarios 2 to 4 run on private buses (scripts/iso-session.sh). Nothing here locks or unlocks the real keyring.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
BIN="$ROOT/target/debug/secret-service-spike"
. "$HERE/common.sh"

(cd "$ROOT" && cargo build --quiet) || exit 1

step "1. Current session (real bus, test entry only)"
echo "BUS=${DBUS_SESSION_BUS_ADDRESS:-unset}"
run "$BIN" status
run "$BIN" set valeur-de-test-spike
run "$BIN" get
run "$BIN" status
run "$BIN" resolve
run "$BIN" delete
run "$BIN" get
run "$BIN" resolve
OPEN_MOSAIK_TMDB_API_KEY=env-test-value run "$BIN" resolve

step "2b. Invalid DBUS_SESSION_BUS_ADDRESS (real environment, variable overridden per command)"
export DBUS_SESSION_BUS_ADDRESS=unix:path=/tmp/open-mosaik-spike-no-such-bus
run "$BIN" status
run "$BIN" set valeur-de-test-spike
run "$BIN" resolve
OPEN_MOSAIK_TMDB_API_KEY=env-test-value run "$BIN" resolve
unset DBUS_SESSION_BUS_ADDRESS

"$HERE/iso-session.sh" nosecrets -- "$HERE/scenario-nosecrets.sh" "$BIN"

"$HERE/iso-session.sh" lock -- "$HERE/scenario-locked.sh" "$BIN"

"$HERE/iso-session.sh" silent -- "$HERE/scenario-hang.sh" "$BIN" silent
"$HERE/iso-session.sh" prompt -- "$HERE/scenario-hang.sh" "$BIN" locked-prompt

step "Post-check: processes of this spike still running (expected: none from this run)"
ps -eo pid,cmd | grep -E 'target/i/|fake-secrets|secret-service-spike' | grep -v grep || echo "none"
