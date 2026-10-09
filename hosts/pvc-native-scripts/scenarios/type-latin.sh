focus_win; click name; sleep 0.2
for r in $(seq ${ROUNDS:-5}); do xdotool type --delay 50 "Ada Lovelace Byron 1"; sleep 0.2; xdotool key ctrl+a; xdotool key Delete; sleep 0.2; done
