#!/usr/bin/env bash
# tools/measure/run_all.sh — runs every path x specimen x scenario cell of METHOD.md and rebuilds SUMMARY.md.
# Long (tens of minutes); run with nohup. Each repetition holds the exclusive CPU lock; set PATHS="gl-llvmpipe" to restrict.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"; ROOT="$(cd "$HERE/../.." && pwd)"
PATHS=${PATHS:-"tier0c-vello_cpu gl-llvmpipe chromium141-webgl2-swiftshader"}
for P in $PATHS; do
  for SP in L01 F01; do
    if [ $SP = L01 ]; then SCS="first-frame idle ptr-move rss"; else SCS="first-frame idle tab type-latin ime-zh toggle-show checkbox rss"; fi
    for SC in $SCS; do
      if [ "$P" = chromium141-webgl2-swiftshader ]; then
        for V in raf sync; do "$HERE/run.sh" "$P" "$SP" "$SC" --variant $V --mirror on || echo "FAILED $P $SP $SC $V"; done
        case $SC in ptr-move|tab|type-latin|toggle-show|checkbox) "$HERE/run.sh" "$P" "$SP" "$SC" --variant raf --mirror off || echo "FAILED $P $SP $SC mirror-off";; esac
      else
        "$HERE/run.sh" "$P" "$SP" "$SC" || echo "FAILED $P $SP $SC"
      fi
    done
  done
done
"$ROOT/.venv/bin/python" "$HERE/make_summary.py"
