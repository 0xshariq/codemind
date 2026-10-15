#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v pnpm >/dev/null 2>&1; then
  printf '%s\n' "pnpm is required. Install the version declared in package.json first." >&2
  exit 1
fi

pnpm install
cargo build --manifest-path engine/Cargo.toml

printf '%s\n' "CodeMind dependencies and the Rust engine are ready."