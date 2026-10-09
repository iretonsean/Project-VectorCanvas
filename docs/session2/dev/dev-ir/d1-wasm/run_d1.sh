#!/usr/bin/env bash
# Re-run the native-vs-wasm32 D1 check (A1 specimens + the 15 Part-2 pilot refs). Output: ../evidence/d1-wasm.txt
set -euo pipefail
D=/workspace/dev_work/dev-ir
mkdir -p $D/evidence /tmp/pvc-d1-native
export CARGO_TARGET_DIR=/workspace/cargo-target
WASM_TARGET=/workspace/cargo-target-wasm
flock -s /workspace/orchestrator_work/pm/CPU_LOCK bash -c "
  cd $D/pvc-ir && cargo build --quiet --bins &&
  cd $D/d1-wasm && CARGO_TARGET_DIR=$WASM_TARGET cargo build --quiet --release --target wasm32-unknown-unknown"
for s in L01 F01; do
  $CARGO_TARGET_DIR/debug/pvc-compile $D/specimens/$s.pvc /tmp/pvc-d1-native/$s.pvca --canonical /tmp/pvc-d1-native/$s.canonical.json >/dev/null
done
for s in L01 L02 L03 L04 L05 L06 L07 L08 L09 L10 G01 G02 G03 G04 G05; do
  $CARGO_TARGET_DIR/debug/pvc-compile $D/pilot-refs/$s.pvc /tmp/pvc-d1-native/ref-$s.pvca --canonical /tmp/pvc-d1-native/ref-$s.canonical.json >/dev/null
done
{
  echo "# D1 native vs wasm32-unknown-unknown (node $(node --version)), $(date -u +%FT%TZ), host=container-xeon4"
  echo "# method: native = pvc-compile output; wasm32 = d1-wasm cdylib driven from node WebAssembly API (no wasm-bindgen)"
  node $D/d1-wasm/run_d1.mjs $WASM_TARGET/wasm32-unknown-unknown/release/pvc_ir_d1_wasm.wasm /tmp/pvc-d1-native $D/specimens $D/pilot-refs
} | tee $D/evidence/d1-wasm.txt
