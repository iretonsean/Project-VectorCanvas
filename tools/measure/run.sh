#!/usr/bin/env bash
# tools/measure/run.sh <path> <specimen> <scenario> [--variant raf|sync] [--mirror on|off] [--atspi on|off] [--reps N] [--out DIR]
#   path     : tier0c-vello_cpu | gl-llvmpipe | chromium141-webgl2-swiftshader
#   specimen : L01 | F01
#   scenario : ptr-move | tab | type-latin | ime-zh | toggle-show | checkbox | idle | first-frame | rss     (METHOD.md §2.1)
# Every repetition runs under `flock -x /workspace/orchestrator_work/pm/CPU_LOCK` (METHOD §9). Re-running replaces the cell's raw files.
# One command reproduces one number, e.g.:   tools/measure/run.sh gl-llvmpipe L01 ptr-move
# Overrides (env): PVC_BIN (native host binary, release), PVC_WWW (web www dir), PVC_ART_DIR (dir with L01.pvca/F01.pvca)
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"; ROOT="$(cd "$HERE/../.." && pwd)"
LOCK=/workspace/orchestrator_work/pm/CPU_LOCK
PY="$ROOT/.venv/bin/python"
[ $# -ge 3 ] || { sed -n 2,9p "$0"; exit 2; }
PATHID=$1; SPEC=$2; SCEN=$3; shift 3
VARIANT=raf; MIRROR=on; ATSPI=off; REPS=""; OUT=""
while [ $# -gt 0 ]; do case $1 in
  --variant) VARIANT=$2; shift 2;; --mirror) MIRROR=$2; shift 2;; --atspi) ATSPI=$2; shift 2;; --reps) REPS=$2; shift 2;; --out) OUT=$2; shift 2;;
  *) echo "bad arg $1"; exit 2;; esac; done
if [ -z "$REPS" ]; then case $SCEN in first-frame) REPS=10;; *) REPS=5;; esac; fi
BIN=${PVC_BIN:-/workspace/cargo-target/release/pvc-native}
ARTDIR=${PVC_ART_DIR:-/workspace/dev_work/dev-ir/specimens}
WWW=${PVC_WWW:-/workspace/dev_work/dev-web/www}
case $PATHID in
  chromium141-webgl2-swiftshader) SUB="$SCEN/${VARIANT}_mirror-${MIRROR}";;
  tier0c-vello_cpu|gl-llvmpipe) SUB="$SCEN";;
  *) echo "unknown path $PATHID"; exit 2;; esac
[ -n "$OUT" ] || OUT="$ROOT/measurements/session2/$PATHID/$SPEC/$SUB"
mkdir -p "$OUT"; rm -f "$OUT"/trace_rep* "$OUT"/runmeta_rep* "$OUT"/meta_rep* "$OUT"/rects_rep* "$OUT"/host_rep* "$OUT"/summary.*
echo "[run.sh] $PATHID $SPEC $SCEN reps=$REPS -> $OUT"

RUNDIR="$ROOT/run_native"
cleanup_env() { if [ -d "$RUNDIR" ]; then for n in xvfb fcitx5 atspi-registryd atspi-launcher dbus; do [ -f "$RUNDIR/$n.pid" ] && kill "$(cat "$RUNDIR/$n.pid")" 2>/dev/null || true; done; fi; }
case $PATHID in
chromium141-webgl2-swiftshader)
  for k in $(seq 1 "$REPS"); do
    CMD="tools/measure/run.sh $PATHID $SPEC $SCEN --variant $VARIANT --mirror $MIRROR --reps $REPS"
    flock -x $LOCK node "$HERE/web_run.mjs" --www "$WWW" --specimen "$SPEC" --scenario "$SCEN" --variant "$VARIANT" --mirror "$MIRROR" --out "$OUT" --rep "$k" --cmd "$CMD"
  done;;
*)
  trap cleanup_env EXIT
  mkdir -p "$RUNDIR"
  NEED_FULL=0; { [ "$SCEN" = ime-zh ] || [ "$ATSPI" = on ]; } && NEED_FULL=1
  export PVC_RUN="$RUNDIR" PVC_DISPLAY=:78
  if [ $NEED_FULL = 1 ]; then
    export PVC_IM=fcitx; [ "$SCEN" = ime-zh ] || PVC_IM=none
    # shellcheck disable=SC1091
    source /workspace/dev_work/dev-native/scripts/env_up.sh >/dev/null 2>&1; ATSPI=on
  else
    rm -f /tmp/.X11-unix/X78 /tmp/.X78-lock
    Xvfb :78 -screen 0 1366x768x24 -nolisten tcp >"$RUNDIR/xvfb.log" 2>&1 & echo $! >"$RUNDIR/xvfb.pid"
    export DISPLAY=:78 LC_ALL=C.UTF-8; unset DBUS_SESSION_BUS_ADDRESS XMODIFIERS GTK_IM_MODULE QT_IM_MODULE || true
    for i in $(seq 50); do xdpyinfo -display :78 >/dev/null 2>&1 && break; sleep 0.1; done
  fi
  export PVC_ATSPI=$ATSPI HOME=${REAL_HOME:-/root}
  for k in $(seq 1 "$REPS"); do
    CMD="tools/measure/run.sh $PATHID $SPEC $SCEN --atspi $ATSPI --reps $REPS"
    flock -x $LOCK "$PY" "$HERE/native_run.py" --path "$PATHID" --bin "$BIN" --artifact "$ARTDIR/$SPEC.pvca" --specimen "$SPEC" --scenario "$SCEN" --out "$OUT" --rep "$k" --cmd "$CMD"
  done;;
esac
"$PY" "$HERE/summarize.py" "$OUT"
echo "[run.sh] summary: $OUT/summary.md"
