#!/usr/bin/env bash
# One-command runner: ./run_web.sh <scenario> [--variant raf|sync] [--mirror on|off] [--dsf N] [--out DIR]
# Serves www/ locally, runs Playwright Chromium (headless, navigator.gpu hidden, WebGL2 via SwiftShader).
# CPU lock: shared by default (functional runs). Timing runs: PVC_LOCK=x ./run_web.sh ... (exclusive).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
export PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers
MODE="${PVC_LOCK:-s}"
exec flock -"$MODE" /workspace/orchestrator_work/pm/CPU_LOCK node "$HERE/tests/run.mjs" "$@"
