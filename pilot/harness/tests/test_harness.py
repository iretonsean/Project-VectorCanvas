#!/usr/bin/env python3
"""Offline harness test with a fake server and stub PVC tools. No LLM, no network. Run: python3 test_harness.py"""
import json, os, pathlib, subprocess, sys, tempfile, hashlib, shutil
T = pathlib.Path(__file__).resolve().parent
H = T.parent
sys.path.insert(0, str(H))
import pilot, tiktoken
enc = tiktoken.get_encoding("o200k_base")
c = lambda s: len(enc.encode(s, disallowed_special=()))

# unit tests
assert pilot.interleaved_order() == ["L01","G01","L02","G02","L03","G03","L04","G04","L05","G05","L06","L07","L08","L09","L10"]
assert pilot.extract_artifact("a\n```pvc\nX\n```\nb\n```pvc\nY\n```\n", ["pvc"]) == "Y\n"
assert pilot.extract_artifact("```\nZ\n```", ["pvc"]) == "Z\n"
assert pilot.extract_artifact("```pvc\nunclosed", ["pvc"]) is None
assert pilot.extract_artifact("```python\nq\n```", ["pvc"]) is None
big = "\n".join(f"line{i}" for i in range(50))
assert pilot.trunc_tool(big).endswith("... (10 more lines omitted)")
assert pilot.trunc_tool("") == ""

tmp = pathlib.Path(tempfile.mkdtemp())
model = tmp / "m.gguf"; model.write_bytes(b"fake")
shim = tmp / "llama-server"
shim.write_text(f"#!/bin/sh\nexec python3 {T}/fake_llama_server.py \"$@\"\n"); shim.chmod(0o755)
(tmp / "ctx.md").write_text("PVC CONTEXT: write pvc.")
cfg = json.loads((H / "config.dryrun.json").read_text())
cfg["llama_server"] = str(shim); cfg["server_args"] = []
cfg["arms"]["S4"].update(model=str(model), sha256=hashlib.sha256(b"fake").hexdigest(), port=18099)
cfg["stacks"] = {"PVC": {"context_files": [str(tmp / "ctx.md")], "fence_tags": ["pvc"], "artifact_name": "a.pvc",
                         "tools": [{"name": "pvc-validate", "argv": ["python3", str(T / "stub_pvc_validate.py"), "{artifact}"]}],
                         "check": {"argv": ["python3", str(T / "stub_pvc_check.py"), "{artifact}"]}}}
cfgp = tmp / "cfg.json"; cfgp.write_text(json.dumps(cfg))
out = tmp / "out"
r = subprocess.run([sys.executable, str(H / "pilot.py"), "run", "--arm", "S4", "--specimens", "L03", "--stacks", "PVC",
                    "--config", str(cfgp), "--out", str(out), "--dry-run"], capture_output=True, text=True)
print(r.stdout, r.stderr)
assert r.returncode == 0 and "outcome=pass turns=3" in r.stdout
tp = out / "S4" / "L03__PVC" / "transcript.jsonl"
recs = [json.loads(l) for l in tp.read_text().splitlines()]
assert recs[0]["label"] == "DRY-RUN — not data"
ck = [x for x in recs if x["kind"] == "check"]
assert [x["status"] for x in ck] == ["no_artifact", "fail", "pass"], [x["status"] for x in ck]
# expected accounting computed independently
ctx = "PVC CONTEXT: write pvc."; req = (H.parent / "intents" / "L03.txt").read_text()
outs = [c(t) for t in ["Here you go, no code block.", "Second try.\n```pvc\nsurface { text \"Hi\" }\n```\n", "Third try.\n```pvc\nsurface { text \"Hi\" button \"Go\" }\n```\n"]]
msgs = [x for x in recs if x["kind"] == "message" and x["role"] == "user" and x["turn"] > 1]
assert msgs[0]["text"] == pilot.REPAIR_HEAD + "\n- NO-ARTIFACT: " + pilot.PSEUDO["NO-ARTIFACT"] + "\n" + pilot.REPAIR_TAIL
assert "pvc-validate output:\nW PVC-W-STUB" in msgs[1]["text"] and "- L00-A1: The button is missing." in msgs[1]["text"]
tool = c("pvc-validate output:\nW PVC-W-STUB 1:1 no interactive element")
rep = [c(pilot.repair_text([("NO-ARTIFACT", pilot.PSEUDO["NO-ARTIFACT"])])), c(pilot.repair_text([("L00-A1", "The button is missing."), ("U1", "Every interactive element needs an accessible name.")]))]
expP = c(req) + 3 * c(ctx) + sum(outs) + tool + sum(rep)
rc = subprocess.run([sys.executable, str(H / "recount.py"), str(tp), "--csv", str(tmp / "r.csv")], capture_output=True, text=True)
print(rc.stdout)
assert rc.returncode == 0, rc.stdout
import csv
row = next(csv.DictReader(open(tmp / "r.csv")))
assert int(row["T_primary"]) == expP, (row["T_primary"], expP)
assert int(row["T_primary_cached0"]) == expP - 3 * c(ctx)
fb = [rep[0], tool + rep[1]]
expH = 0; hist = 0
for t in range(3):
    expH += c(ctx) + c(req) + hist + outs[t]; hist += outs[t] + (fb[t] if t < 2 else 0)
assert int(row["T_history"]) == expH, (row["T_history"], expH)
# tamper test: recount must flag an edited message
bad = tmp / "bad.jsonl"
lines = tp.read_text().splitlines()
d = json.loads(lines[2]); d["text"] += " extra"; lines[2] = json.dumps(d, ensure_ascii=False); bad.write_text("\n".join(lines) + "\n")
assert subprocess.run([sys.executable, str(H / "recount.py"), str(bad)], capture_output=True, text=True).returncode == 1
# real-run guard
g = subprocess.run([sys.executable, str(H / "pilot.py"), "run", "--arm", "S4", "--config", str(cfgp), "--out", str(tmp / "x")], capture_output=True, text=True)
assert g.returncode != 0 and "REFUSED" in (g.stdout + g.stderr)
# exclusive-run lock mode: held for the whole run, excludes other holders, released afterwards
import fcntl
lockf = tmp / "LOCK"; lockf.write_text("")
L = pilot.SharedLock(str(lockf), "exclusive-run"); L.acquire_run()
fd2 = os.open(str(lockf), os.O_RDONLY)
try:
    fcntl.flock(fd2, fcntl.LOCK_SH | fcntl.LOCK_NB); raise AssertionError("shared lock must be refused while exclusive is held")
except BlockingIOError:
    pass
with L.held():
    pass                      # no-op, no deadlock
L.release_run()
fcntl.flock(fd2, fcntl.LOCK_SH | fcntl.LOCK_NB); fcntl.flock(fd2, fcntl.LOCK_UN); os.close(fd2)
cfg["cpu_lock"] = str(lockf); cfgp.write_text(json.dumps(cfg))
r2 = subprocess.run([sys.executable, str(H / "pilot.py"), "run", "--arm", "S4", "--specimens", "L03", "--stacks", "PVC", "--config", str(cfgp),
                     "--out", str(tmp / "out2"), "--dry-run", "--lock", "exclusive-run"], capture_output=True, text=True)
assert r2.returncode == 0 and "outcome=pass turns=3" in r2.stdout, r2.stdout + r2.stderr
assert json.loads((tmp / "out2/S4/L03__PVC/transcript.jsonl").read_text().splitlines()[0])["cpu_lock_mode"] == "exclusive-run"
open(tmp / "PREREG.md", "w").close()
g2 = subprocess.run([sys.executable, str(H / "pilot.py"), "run", "--arm", "S4", "--config", str(cfgp), "--out", str(tmp / "y"), "--lock", "shared-per-call", "--prereg-sha256", pilot.sha256_file(H.parent / "PREREG.md")], capture_output=True, text=True)
assert g2.returncode != 0 and "REFUSED" in (g2.stdout + g2.stderr)
shutil.rmtree(tmp)
print("ALL HARNESS TESTS PASSED")
