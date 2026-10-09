#!/usr/bin/env python3
"""SYNTHETIC data only (not data). Checks analyse.py mechanics: prefix, pairing, bootstrap, rule, statements, dry-run refusal."""
import csv, subprocess, sys, tempfile, pathlib, json, random
H = pathlib.Path(__file__).resolve().parent.parent
ORDER = ["L01","G01","L02","G02","L03","G03","L04","G04","L05","G05","L06","L07","L08","L09","L10"]
def make(path, pvc_factor, n_specs=15, label="SYNTHETIC TEST", s7=True):
    rng = random.Random(1)
    cols = "label arm specimen stack outcome turns pass_turn T_primary T_primary_cached0 T_history T_history_cached0 output_only S_primary".split()
    w = csv.DictWriter(open(path, "w", newline=""), fieldnames=cols); w.writeheader()
    for arm in (["S4", "S7"] if s7 else ["S4"]):
        for s in ORDER[:n_specs]:
            base = 1000 + rng.randrange(200)
            for stk in ("PVC", "B1", "B2"):
                t = int(base * (pvc_factor if stk == "PVC" else (1.0 if stk == "B1" else 1.3)))
                w.writerow(dict(label=label, arm=arm, specimen=s, stack=stk, outcome="pass", turns=1, pass_turn=1, T_primary=t, T_primary_cached0=t-50,
                                T_history=t, T_history_cached0=t-50, output_only=t//2, S_primary=int(t*1.1)))
tmp = pathlib.Path(tempfile.mkdtemp())
make(tmp / "a.csv", 0.5)
r = subprocess.run([sys.executable, str(H / "analyse.py"), str(tmp / "a.csv"), "--json", str(tmp / "a.json")], capture_output=True, text=True)
assert r.returncode == 0, r.stderr
j = json.loads((tmp / "a.json").read_text())
assert j["S4"]["analysed"] == ORDER and j["S4"]["evaluable"]
assert j["S4"]["cats"]["layout"]["rule"]["WIN"] and j["falsification"]["fired"] is False, j["falsification"]
make(tmp / "b.csv", 1.5)
subprocess.run([sys.executable, str(H / "analyse.py"), str(tmp / "b.csv"), "--json", str(tmp / "b.json")], capture_output=True, text=True, check=True)
j = json.loads((tmp / "b.json").read_text())
assert j["falsification"]["fired"] is True and "FIRED" in j["falsification"]["statement"] and "small local models" in j["falsification"]["statement"]
make(tmp / "c.csv", 0.5, n_specs=8, s7=False)       # prefix of 8 < 10 -> not evaluable
subprocess.run([sys.executable, str(H / "analyse.py"), str(tmp / "c.csv"), "--json", str(tmp / "c.json")], capture_output=True, text=True, check=True)
j = json.loads((tmp / "c.json").read_text())
assert j["falsification"]["fired"] is None and j["S4"]["analysed"] == ORDER[:8]
make(tmp / "d.csv", 0.5, label="DRY-RUN — not data")
assert subprocess.run([sys.executable, str(H / "analyse.py"), str(tmp / "d.csv")], capture_output=True, text=True).returncode != 0
print("ANALYSE TESTS PASSED")
