#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_dir="$repo_root/apps/web/src/lib/server/rust-client/generated"

rm -rf "$output_dir"
mkdir -p "$output_dir"

(
  cd "$repo_root/apps/web"
  pnpm exec proto-loader-gen-types \
    --keepCase \
    --enums String \
    --longs String \
    --includeDirs "$repo_root/proto" \
    --outDir "$output_dir" \
    --grpcLib "@grpc/grpc-js" \
    "$repo_root"/proto/v1/*.proto
)