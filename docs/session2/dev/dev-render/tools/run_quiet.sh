#!/bin/bash
# Exclusive CPU lock; waits (max 240 s) for the 1-min load average to fall below 1.0 so the run starts on a quiet box.
for i in $(seq 1 48); do l=$(cut -d' ' -f1 /proc/loadavg); if awk "BEGIN{exit !($l < 1.0)}"; then break; fi; sleep 5; done
/workspace/dev_work/dev-render/tools/run_frame_bench.sh
/workspace/dev_work/dev-render/tools/run_rss.sh
