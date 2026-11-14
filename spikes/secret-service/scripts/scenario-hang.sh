#!/usr/bin/env bash
# Scenario 4 (run INSIDE iso-session.sh): unresponsive Secret Service, test doubles.
# Starts scripts/fake-secrets.py in the given mode, runs the binary, stops the double by PID.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
BIN="$1"
MODE="$2"
. "$HERE/common.sh"

python3 -I "$HERE/fake-secrets.py" "$MODE" > "$XDG_RUNTIME_DIR/fake-$MODE.log" 2>&1 &
FPID=$!
sleep 2
step "4 ($MODE). Test double owns org.freedesktop.secrets (PID $FPID, started by this scenario)"
case "$MODE" in
  silent)
    run "$BIN" status
    run "$BIN" --timeout-secs 3 resolve
    OPEN_MOSAIK_TMDB_API_KEY=env-test-value run "$BIN" resolve
    ;;
  locked-prompt)
    run "$BIN" status
    run "$BIN" resolve
    run "$BIN" raw-get
    OPEN_MOSAIK_TMDB_API_KEY=env-test-value run "$BIN" resolve
    ;;
esac
step "4 ($MODE). Calls received by the test double"
cat "$XDG_RUNTIME_DIR/fake-$MODE.log"
kill "$FPID" 2>/dev/null; sleep 1
echo "test double PID $FPID still alive: $(ps -p "$FPID" -o pid= 2>/dev/null || echo no)"
