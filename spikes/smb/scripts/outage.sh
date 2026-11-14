#!/usr/bin/env bash
# Outage matrix for spike 2.3. Usage: scripts/outage.sh <stop|pause> <list|read> [op_timeout_ms] [inject_after_s]
# Starts the client in background, injects the fault, and prints time-to-error measured from injection.
set -uo pipefail
cd "$(dirname "$0")/.."
MODE=${1:?stop|pause}; WHAT=${2:?list|read}; OPT=${3:-0}; AFTER=${4:-4}
C=epikodi-smb-spike; B=./target/release/smb-spike
case $WHAT in
  list) CMD=(list --recursive --pause-ms 300) ;;
  read) CMD=(throughput --chunk 65536 --pause-ms 20) ;;
esac
now() { date +%s.%N; }
LOG=$(mktemp)
timeout 120 $B --share secure --op-timeout-ms "$OPT" "${CMD[@]}" >"$LOG" 2>&1 &
PID=$!
sleep "$AFTER"
T0=$(now)
if [ "$MODE" = stop ]; then docker stop -t 1 $C >/dev/null; else docker pause $C >/dev/null; fi
wait $PID; RC=$?
T1=$(now)
echo "mode=$MODE what=$WHAT op_timeout_ms=$OPT rc=$RC time_to_exit_after_fault=$(echo "$T1 - $T0" | bc)s"
tail -2 "$LOG"; rm -f "$LOG"
# restore
if [ "$MODE" = stop ]; then docker start $C >/dev/null; else docker unpause $C >/dev/null; fi
sleep 2
