#!/bin/bash
# Bring up Xvfb + private dbus session + AT-SPI + fcitx5(pinyin) for dev-native.
# Usage: source scripts/env_up.sh [up|down]   (up is default). Exports DISPLAY, DBUS_SESSION_BUS_ADDRESS, XMODIFIERS ...
# Display :77 (not :99). All state under $PVC_RUN (default /workspace/dev_work/dev-native/run).
ROOT=/workspace/dev_work/dev-native
PVC_RUN=${PVC_RUN:-$ROOT/run}
PVC_DISPLAY=${PVC_DISPLAY:-:77}
PVC_IM=${PVC_IM:-fcitx}          # fcitx | none
mkdir -p "$PVC_RUN" "$ROOT/logs"
pidf() { echo "$PVC_RUN/$1.pid"; }
stop_all() {
  for n in fcitx5 atspi-registryd atspi-launcher dbus xvfb; do
    if [ -f "$(pidf $n)" ]; then p=$(cat "$(pidf $n)"); kill "$p" 2>/dev/null; for i in $(seq 30); do kill -0 "$p" 2>/dev/null || break; sleep 0.1; done; rm -f "$(pidf $n)"; fi
  done
}
if [ "${1:-up}" = down ]; then stop_all; return 0 2>/dev/null || exit 0; fi
stop_all
rm -f /tmp/.X11-unix/X${PVC_DISPLAY#:} /tmp/.X${PVC_DISPLAY#:}-lock
export DISPLAY=$PVC_DISPLAY
export XDG_RUNTIME_DIR=$PVC_RUN/xdg-runtime; mkdir -p "$XDG_RUNTIME_DIR"; chmod 700 "$XDG_RUNTIME_DIR"
export XDG_CONFIG_HOME=$PVC_RUN/xdg-config XDG_DATA_HOME=$PVC_RUN/xdg-data XDG_CACHE_HOME=$PVC_RUN/xdg-cache
export LC_ALL=C.UTF-8 LANG=C.UTF-8
# X
Xvfb $DISPLAY -screen 0 1366x768x24 -nolisten tcp >"$ROOT/logs/xvfb.log" 2>&1 & echo $! >"$(pidf xvfb)"
for i in $(seq 50); do xdpyinfo -display $DISPLAY >/dev/null 2>&1 && break; sleep 0.1; done
# private session bus
eval "$(dbus-launch --sh-syntax)"; echo $DBUS_SESSION_BUS_ADDRESS >"$PVC_RUN/dbus.addr"; echo $DBUS_SESSION_BUS_PID >"$(pidf dbus)"
export DBUS_SESSION_BUS_ADDRESS
# AT-SPI bus launcher + registry (a11y bus launched on demand by at-spi-bus-launcher)
/usr/libexec/at-spi-bus-launcher --launch-immediately >"$ROOT/logs/at-spi-bus-launcher.log" 2>&1 & echo $! >"$(pidf atspi-launcher)"
sleep 0.7
/usr/libexec/at-spi2-registryd --use-gnome-session >"$ROOT/logs/at-spi2-registryd.log" 2>&1 & echo $! >"$(pidf atspi-registryd)"
export GTK_MODULES= NO_AT_BRIDGE=0 ACCESSIBILITY_ENABLED=1
gsettings set org.gnome.desktop.interface toolkit-accessibility true >/dev/null 2>&1 || true
if [ "$PVC_IM" = fcitx ]; then
  cp "$ROOT/config/fcitx5-profile" "$XDG_CONFIG_HOME/fcitx5/profile"
  cp "$ROOT/config/fcitx5-config" "$XDG_CONFIG_HOME/fcitx5/config"
  mkdir -p "$XDG_CONFIG_HOME/fcitx5/conf"; cp "$ROOT"/config/conf/*.conf "$XDG_CONFIG_HOME/fcitx5/conf/"
  export XMODIFIERS=@im=fcitx GTK_IM_MODULE=fcitx QT_IM_MODULE=fcitx
  fcitx5 --disable=wayland,waylandim,kimpanel,virtualkeyboard -D >"$ROOT/logs/fcitx5.log" 2>&1 & echo $! >"$(pidf fcitx5)"
  # (fcitx5 -D = daemon flag absent in 5.1.7? we background it ourselves)
  sleep 1.5
fi
echo "env_up: DISPLAY=$DISPLAY DBUS=$DBUS_SESSION_BUS_ADDRESS XMODIFIERS=${XMODIFIERS:-}"
