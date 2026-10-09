# L01-style pointer-move: alternate between two points (placeholder: toggle<->name rect centres) N times at 50 ms gap
focus_win
read -r ax ay < <(center ${PTR_IN:-toggle}); read -r bx by < <(center ${PTR_OUT:-name})
xdotool mousemove $bx $by; sleep 0.3
for i in $(seq ${N:-200}); do xdotool mousemove $ax $ay; sleep 0.05; xdotool mousemove $bx $by; sleep 0.05; done
