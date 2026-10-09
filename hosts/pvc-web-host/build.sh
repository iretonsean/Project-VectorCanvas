#!/usr/bin/env bash
# Build pvc-web for wasm32 and run wasm-bindgen (0.2.129, installed under ./tools) into www/pkg.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
cd "$HERE/pvc-web"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/workspace/cargo-target-wasm}"
flock -s /workspace/orchestrator_work/pm/CPU_LOCK cargo build --release --target wasm32-unknown-unknown "$@"
"$HERE/tools/bin/wasm-bindgen" --target web --no-typescript --out-dir "$HERE/www/pkg" \
  "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/pvc_web.wasm"
( cd "$HERE/www/pkg" && sha256sum * > "$HERE/www/pkg.sha256" )
ls -la "$HERE/www/pkg"
