#!/bin/bash
# host=container-xeon4 path=tier0c-vello_cpu, headless runtime stages + vello_cpu render; exclusive CPU lock.
OUT=/workspace/dev_work/dev-render/measurements/frame_bench
BIN=/workspace/cargo-target/release/examples/frame_bench
{
echo "# host=container-xeon4 path=tier0c-vello_cpu (headless, no window/present; general constraint solve excluded, D-4)"
echo "# date=$(date -u +%FT%TZ) loadavg_at_start=$(cat /proc/loadavg) binary_sha256=$(sha256sum $BIN | cut -d' ' -f1)"
echo "# viewport 1366x768 dsf 1.0; 200 measured typing/IME frames after 10 warm-up frames per scenario; render_ms = vello_cpu render + RGBA convert"
$BIN $OUT/frames.csv
} > $OUT/summary.jsonl
