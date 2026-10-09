# F01 accessibility: dump -> Tab x5 + Space (checkbox) -> dump; then make email invalid -> error exposed -> dump
focus_win; settle
/usr/bin/python3.12 $ROOT/scripts/atspi_listen.py $OUT/atspi_events.jsonl 14 > $OUT/atspi_listen.log 2>&1 &
LISTEN=$!; sleep 1.5
dump a11y_0_initial
for i in 1 2 3 4 5; do xdotool key Tab; sleep 0.15; done
xdotool key space; settle
state a11y_1_checkbox
dump a11y_1_after_checkbox_space
shot a11y_1
# error message: focus email by click, type an invalid address
click email; settle 0.2
xdotool type --delay 80 "bad"; settle
state a11y_2_invalid_email
dump a11y_2_invalid_email
shot a11y_2
sleep 0.5
wait $LISTEN 2>/dev/null
