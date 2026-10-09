# shared helpers for scenarios (sourced). Expects: OUT (output dir), DISPLAY exported, pvc-native running.
ROOT=/workspace/dev_work/dev-native
rect() { # rect NAME -> "x y w h"
  python3 - "$OUT/rects.json" "$1" <<'PY'
import json,sys
r=json.load(open(sys.argv[1]))[sys.argv[2]]
print(*r)
PY
}
center() { read -r x y w h < <(rect "$1"); python3 -c "print(int($x+$w/2), int($y+$h/2))"; }
move() { xdotool mousemove $(center "$1"); }
click() { move "$1"; sleep ${GAP:-0.05}; xdotool click 1; }
winid() { xdotool search --name '^PVC$' | head -1; }
focus_win() { xdotool windowfocus "$(winid)"; sleep 0.2; }
shot() { xwd -root -display "$DISPLAY" | convert xwd:- "$OUT/$1.png"; }
dump() { /usr/bin/python3.12 "$ROOT/scripts/atspi_dump.py" --brief --out "$OUT/$1.json" > "$OUT/$1.txt" 2>"$OUT/$1.err"; }
state() { cp "$OUT/state.json" "$OUT/$1.state.json" 2>/dev/null; }
settle() { sleep ${1:-0.4}; }
