# System IME (fcitx5 pinyin via XIM): click name field, type n i h a o, capture preedit, select candidate 1 (space), capture commit
focus_win; settle
click name; settle 0.3
xdotool key ctrl+space; settle 0.4   # fcitx5 trigger key: keyboard-us -> pinyin
for c in n i h a o; do xdotool key $c; sleep 0.2; done
settle 0.5
state ime_1_preedit
shot ime_1_preedit
dump ime_1_preedit
xdotool key space; settle 0.5
state ime_2_commit
shot ime_2_commit
dump ime_2_commit
