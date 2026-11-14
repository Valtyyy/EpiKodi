#!/usr/bin/env bash
# Builds spikes/smb/fixtures: >=3 levels deep, ~200 files, one 50 MB random file.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)/fixtures"
rm -rf "$ROOT"; mkdir -p "$ROOT"
n=0
for a in Movies Series Music Docs; do
  for b in A B C; do
    for c in 2019 2020 2021; do
      d="$ROOT/$a/$b/$c"; mkdir -p "$d"
      for i in 1 2 3 4 5; do
        head -c $((RANDOM * 4 + 1000)) /dev/urandom > "$d/file_${i}.bin"; n=$((n+1))
      done
    done
  done
done
mkdir -p "$ROOT/Movies/big"
head -c $((50*1024*1024)) /dev/urandom > "$ROOT/Movies/big/big50.bin"
mkdir -p "$ROOT/Docs/accents"
echo hello > "$ROOT/Docs/accents/été film (1080p) [x265].txt"
chmod -R a+rX "$ROOT"
echo "fixtures: $(find "$ROOT" -type f | wc -l) files, $(find "$ROOT" -type d | wc -l) dirs"
