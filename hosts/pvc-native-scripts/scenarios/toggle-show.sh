focus_win; move toggle; sleep 0.3
for i in $(seq ${N:-100}); do xdotool click 1; sleep 0.05; done
