#!/bin/bash
# D3: identical solved geometry native vs wasm32. Method: same Rust scenarios (src/lib.rs::dump_all) built
# once as a native binary and once as a wasm32-unknown-unknown cdylib (no wasm-bindgen: the module has no
# imports) executed under node; the two text dumps must be byte-identical.
set -e
cd "$(dirname "$0")"
OUT=${1:-/workspace/dev_work/dev-render/measurements/d3}
mkdir -p "$OUT"
export CARGO_TARGET_DIR=/workspace/cargo-target
flock -s /workspace/orchestrator_work/pm/CPU_LOCK cargo build --release --bin d3_native
CARGO_TARGET_DIR=/workspace/cargo-target-wasm flock -s /workspace/orchestrator_work/pm/CPU_LOCK cargo build --release --lib --target wasm32-unknown-unknown
/workspace/cargo-target/release/d3_native > "$OUT/geometry_native.txt"
WASM=/workspace/cargo-target-wasm/wasm32-unknown-unknown/release/pvc_d3.wasm
node --input-type=module -e "
import fs from 'fs';
const m = await WebAssembly.instantiate(fs.readFileSync('$WASM'), {});
const ex = m.instance.exports;
const n = ex.d3_run();
const b = Buffer.alloc(n);
for (let i=0;i<n;i++) b[i]=ex.d3_byte(i);
fs.writeFileSync('$OUT/geometry_wasm32.txt', b);
"
sha256sum "$OUT"/geometry_native.txt "$OUT"/geometry_wasm32.txt | tee "$OUT/sha256.txt"
if cmp "$OUT/geometry_native.txt" "$OUT/geometry_wasm32.txt"; then echo "D3: native == wasm32 (byte-identical)" | tee -a "$OUT/result.txt"; else echo "D3: MISMATCH" | tee -a "$OUT/result.txt"; exit 1; fi
wc -l "$OUT/geometry_native.txt"
