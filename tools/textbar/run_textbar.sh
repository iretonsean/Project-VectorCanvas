#!/usr/bin/env bash
# Regenerates the whole A8 text bar (renders, FLIP, line breaks). Heavy job: shared CPU lock, NOT a timing measurement.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
TB="$ROOT/measurements/session2/text-bar"
W="$TB/work"
LOCK=/workspace/orchestrator_work/pm/CPU_LOCK
rm -rf "$W"; mkdir -p "$W"
export PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers
{
  echo "date_utc=$(date -u +%FT%TZ)"; echo "loadavg=$(cat /proc/loadavg)"
  echo "fc-match: $(fc-match -v sans | grep -E 'hintstyle|rgba|antialias' | tr -s ' \t' ' ' | tr '\n' ';')"
  sha256sum "$TB/corpus.json" /workspace/dev_work/dev-render/pvc-runtime/fonts/DejaVuSans.ttf /workspace/dev_work/dev-render/pvc-runtime/fonts/DejaVuSans-Bold.ttf /workspace/dev_work/dev-render/pvc-runtime/fonts/NotoSansCJKsc-Regular.otf
  echo "dev-render src tree sha256 (cat of src/*.rs):"; cat /workspace/dev_work/dev-render/pvc-runtime/src/*.rs | sha256sum
} > "$TB/run_env.txt"
# Chromium reference = grayscale (--disable-lcd-text, Addendum A1) plus the default LCD render as secondary; Firefox default
PVC_BROWSER_ARGS="--disable-lcd-text" flock -s $LOCK node "$HERE/browser_render.mjs" chromium "$W/chromium" "$TB/corpus.json"
flock -s $LOCK node "$HERE/browser_render.mjs" chromium "$W/chromium-lcd" "$TB/corpus.json"
PVC_BROWSER_ARGS="--disable-lcd-text --font-render-hinting=none" flock -s $LOCK node "$HERE/browser_render.mjs" chromium "$W/chromium-nohint" "$TB/corpus.json"   # informational, Addendum A5
flock -s $LOCK node "$HERE/browser_render.mjs" firefox "$W/firefox" "$TB/corpus.json"
export CARGO_TARGET_DIR=/workspace/cargo-target/dev-measure
(cd "$HERE/pvc_text" && flock -s $LOCK cargo build --release 2>&1 | tail -2)
flock -s $LOCK /workspace/cargo-target/dev-measure/release/pvc_text "$TB/corpus.json" "$W/pvc"
# the node script names breaks_<basename(outDir)>.json; pvc writes breaks_pvc.json
flock -s $LOCK "$ROOT/.venv/bin/python" "$HERE/compare.py" "$W" "$TB"
