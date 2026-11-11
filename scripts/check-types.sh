#!/usr/bin/env bash
# Fails when the committed ts-rs bindings are out of date with the Rust types.
set -euo pipefail
cd "$(dirname "$0")/.."
pnpm gen:types
if [ -n "$(git status --porcelain -- ui/shared/src/generated)" ]; then
  git --no-pager diff -- ui/shared/src/generated || true
  echo "error: generated TS types are out of date. Run 'pnpm gen:types' and commit ui/shared/src/generated." >&2
  exit 1
fi
