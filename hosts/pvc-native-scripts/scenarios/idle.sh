focus_win; settle 1.0
echo "frames_at_start=$(($(wc -l < "$OUT/trace.csv")-1))" > "$OUT/idle.txt"
sleep ${IDLE_SECS:-5}
echo "frames_at_end=$(($(wc -l < "$OUT/trace.csv")-1))" >> "$OUT/idle.txt"
