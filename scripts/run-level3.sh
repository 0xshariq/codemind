#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT/engine"

if [[ -f .env.test ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env.test
  set +a
fi

: "${FALKORDB_URL:=redis://127.0.0.1:6380}"
export FALKORDB_URL

cargo test --test level3_engines -- --ignored
cargo test --test level3_storage -- --ignored
cargo test --test level3_memory -- --ignored
cargo test --test level3_transport -- --ignored
cargo test --test level3_operation_grpc -- --ignored
cargo test --test operation_recovery -- --ignored
