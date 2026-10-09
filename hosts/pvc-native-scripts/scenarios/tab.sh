focus_win; click ${TAB_START:-name}; sleep 0.2
for i in $(seq ${N:-60}); do xdotool key Tab; sleep 0.05; done
