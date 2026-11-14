#!/usr/bin/env bash
# Usage: ./run-measure.sh <name> [env assignments...] -- [app args...]
# Runs one measured session under timeout, logging to target/logs/<name>.{log,json}.
set -u
cd "$(dirname "$0")"
name="$1"; shift
envs=()
while [ $# -gt 0 ] && [ "$1" != "--" ]; do envs+=("$1"); shift; done
[ "${1:-}" = "--" ] && shift
mkdir -p target/logs "target/logs/$name"
env "${envs[@]}" timeout 120 ./target/debug/video-overlay-spike --measure \
  --out "target/logs/$name.json" --screenshots "target/logs/$name" "$@" \
  >"target/logs/$name.stdout" 2>"target/logs/$name.log"
echo "exit=$?" | tee "target/logs/$name.exit"
