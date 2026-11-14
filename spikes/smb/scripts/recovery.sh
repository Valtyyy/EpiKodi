#!/usr/bin/env bash
# Recovery test: one client probes every second; fault injected at t=5s, lifted at t=20s.
# Usage: scripts/recovery.sh <stop|pause> [extra smb-spike args e.g. --reconnect]
set -uo pipefail
cd "$(dirname "$0")/.."
MODE=${1:?stop|pause}; shift
C=epikodi-smb-spike
timeout 120 ./target/release/smb-spike --share secure watch --seconds 50 "$@" &
PID=$!
sleep 5;  echo ">>> $(date +%T) inject $MODE"; if [ $MODE = stop ]; then docker stop -t 1 $C >/dev/null; else docker pause $C >/dev/null; fi
sleep 15; echo ">>> $(date +%T) restore"; if [ $MODE = stop ]; then docker start $C >/dev/null; else docker unpause $C >/dev/null; fi
wait $PID; echo "rc=$?"
