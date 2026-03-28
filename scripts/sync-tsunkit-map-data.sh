#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

DATA_ROOT=".data/temp"
JSON_OUT=".data/generated/tsunkit_map_catalog.json"
RUST_OUT="crates/emukc_model/src/codex/generated_map_catalog.rs"

echo "[1/2] Syncing Tsunkit map cache..."
echo "This is best-effort: Tsunkit may fail on nodesummary/enemycomps, but cached data will still be normalized."
cargo run --example tsunkit_map_data_downloader -- \
  sync \
  --data-root "$DATA_ROOT" \
  --output "$JSON_OUT" \
  --include-nodesummaries \
  --include-enemycomps

echo "[2/2] Generating baked Rust map catalog..."
cargo run --example tsunkit_map_data_downloader -- \
  codegen \
  --input "$JSON_OUT" \
  --output "$RUST_OUT"

echo "Done."
echo "Normalized JSON: $JSON_OUT"
echo "Generated Rust:  $RUST_OUT"
