#!/bin/bash
# run_native.sh <scenario> [--artifact FILE] [--locale en|zh-CN] [--bin PATH] [--out DIR] [--keep-env]
# Brings up Xvfb(:77) + dbus + AT-SPI + fcitx5(pinyin), runs pvc-native, runs scenarios/<scenario>.sh against it,
# sends SIGTERM (host writes meta.json incl. VmHWM), stops the host. Outputs in --out (default run/out/<scenario>).
set -u
ROOT=/workspace/dev_work/dev-native
SCEN=${1:?scenario (a11y|ime_zh|idle|ptr-move|tab|type-latin|toggle-show|checkbox|smoke)}; shift
RENDERER=gl; ART=/workspace/dev_work/dev-ir/specimens/F01.pvca; LOCALE=en; BIN=${CARGO_TARGET_DIR:-/workspace/cargo-target}/release/pvc-native; OUT=$ROOT/run/out/$SCEN; KEEP=0
while [ $# -gt 0 ]; do case $1 in
  --artifact) ART=$2; shift 2;; --locale) LOCALE=$2; shift 2;; --bin) BIN=$2; shift 2;; --out) OUT=$2; shift 2;; --keep-env) KEEP=1; shift;; --renderer) RENDERER=$2; shift 2;; *) echo "bad arg $1"; exit 2;; esac; done
mkdir -p "$OUT"; rm -f "$OUT"/*
[ -x "$BIN" ] || BIN=${CARGO_TARGET_DIR:-/workspace/cargo-target}/debug/pvc-native
if [ -z "${PVC_ENV_READY:-}" ]; then source $ROOT/scripts/env_up.sh >/dev/null 2>&1; fi
export HOME=${REAL_HOME:-/root}
export PVC_TRACE=$OUT/trace.csv PVC_META=$OUT/meta.json PVC_RECTS=$OUT/rects.json PVC_STATE=$OUT/state.json PVC_SCENARIO=$SCEN
{ date -u +%FT%TZ; echo "cmd: $BIN $ART --locale $LOCALE --renderer $RENDERER"; echo "scenario: $SCEN"; echo "loadavg: $(cat /proc/loadavg)"; echo "DISPLAY=$DISPLAY XMODIFIERS=${XMODIFIERS:-} DBUS=$DBUS_SESSION_BUS_ADDRESS"; } > "$OUT/run.txt"
"$BIN" "$ART" --locale "$LOCALE" --renderer "$RENDERER" > "$OUT/host.log" 2>&1 &
HP=$!
for i in $(seq 100); do [ -s "$OUT/rects.json" ] && break; sleep 0.1; done
[ -s "$OUT/rects.json" ] || { echo "host did not present a first frame"; cat "$OUT/host.log"; kill $HP 2>/dev/null; exit 1; }
sleep 0.5
source $ROOT/scripts/lib.sh
if [ "$SCEN" != smoke ]; then source $ROOT/scripts/scenarios/$SCEN.sh; fi
echo "VmHWM_before_term: $(grep -E 'VmHWM|VmRSS' /proc/$HP/status | tr -s ' \t' ' ' | tr '\n' ';')" >> "$OUT/run.txt"
kill -TERM $HP; wait $HP 2>/dev/null; echo "host_exit=$?" >> "$OUT/run.txt"
[ $KEEP = 1 ] || source $ROOT/scripts/env_up.sh down
echo "outputs in $OUT"
