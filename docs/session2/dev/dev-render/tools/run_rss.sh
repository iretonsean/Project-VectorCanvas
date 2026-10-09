#!/bin/bash
# Font/RSS probe, 5 fresh processes, exclusive CPU lock. host=container-xeon4 / tier0c-vello_cpu (native release build).
OUT=/workspace/dev_work/dev-render/measurements/rss
BIN=/workspace/cargo-target/release/examples/rss_probe
{
echo "# host=container-xeon4 path=tier0c-vello_cpu (headless; no window)"
echo "# date=$(date -u +%FT%TZ)  loadavg_at_start=$(cat /proc/loadavg)"
echo "# binary=$BIN size_bytes=$(stat -c %s $BIN) sha256=$(sha256sum $BIN | cut -d' ' -f1)"
for i in 1 2 3 4 5; do $BIN; done
} > $OUT/rss_probe.jsonl
