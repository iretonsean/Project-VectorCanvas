#!/usr/bin/env python3
"""Owner A/B test: PVC agent vs control agent, judged by an evaluator agent.

Run from the repository root:
    python3 pilot/owner-test/run.py [T1 T2 ...] [--model MODEL] [--effort LEVEL] [--events]

For each test, three headless Claude Code sessions run, each in its own new
folder outside the repository:
  1. PVC agent: gets the three PVC agent-context files and the validator and
     compiler, and writes app.pvc.
  2. Control agent: gets only the description, picks any language, and says
     which one and why.
  3. Evaluator: gets the description, the fixed checklist and both results
     (code and screenshots) under the neutral labels A and B, never the token
     counts, and marks each checklist item pass or fail.

Token counts come from Claude Code's own report for each agent session
(modelUsage), never from the agents.

Output: one Markdown file per test in tests-run/ at the repository root,
named <UTC time>_<test id>_<slug>.md, with a folder of the same name beside
it (screenshots, every file the agents wrote, raw Claude Code transcripts).
tests-run/INDEX.md gets one row per test.

--events prints one JSON object per line (used by the pvc-test mod); without
it, progress is printed as plain text. Standard library only; tiktoken is
used for code-size counts when it is installed.
"""
import argparse
import datetime as dt
import hashlib
import json
import os
import platform
import random
import re
import shutil
import subprocess
import tempfile
import uuid
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
OUT = REPO / "tests-run"
CONTEXT = REPO / "pilot" / "agent-context"
CONTEXT_FILES = ["00_card.md", "01_std_reference.md", "02_dialect_en.md"]
TOOLS = ["Bash", "Read", "Write", "Edit", "Glob", "Grep"]
EVAL_TOOLS = ["Read", "Write", "Glob", "Grep"]
AGENT_TIMEOUT_S = 30 * 60
# Variables that tie a child session to the session that started it.
PARENT_VARS = [
    "CLAUDECODE", "CLAUDE_CODE_SESSION_ID", "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_MESSAGING_SOCKET", "CLAUDE_PID", "CLAUDE_CODE_SSE_PORT",
    "CLAUDE_CODE_ENTRYPOINT", "CLAUDE_CODE_SESSION_ATTENDED", "CLAUDE_AFTER_LAST_COMPACT",
]
TOOL_BINARIES = {"pvc-validate", "pvc-compile", "pvc-validate.exe", "pvc-compile.exe"}
NOT_CODE = {"RESULT.json", "NOTE.txt", "validator.txt", "app.pvca", "screenshot.png"}
CODE_LANG = {".pvc": "text", ".html": "html", ".htm": "html", ".css": "css", ".js": "javascript",
             ".mjs": "javascript", ".jsx": "jsx", ".ts": "typescript", ".tsx": "tsx", ".py": "python",
             ".json": "json", ".vue": "vue", ".svelte": "svelte", ".md": "markdown"}

PVC_PROMPT = """You are building a small app in PVC-IR, a user-interface language.
The files 00_card.md, 01_std_reference.md and 02_dialect_en.md in this folder teach you the language. Read them first. Use only PVC-IR.

App to build:
{description}

Write the program to app.pvc. Check it with `./pvc-validate app.pvc` and fix every error until it exits 0. Then run `./pvc-compile app.pvc app.pvca`. Work only inside this folder.
When you are done, write RESULT.json with this content: {{"language": "PVC-IR", "entry": "app.pvc", "notes": "<one sentence>"}}"""

CONTROL_PROMPT = """Build this app. Choose the language or framework you think is best for it.

App to build:
{description}

Work only inside this folder. You may check your work in any way that is available here.
If you choose a web technology, make the app open from a local file (for example index.html) with no build step and no network access.
When you are done, write RESULT.json with this content: {{"language": "<language or framework you chose>", "why": "<one sentence>", "entry": "<main file, relative path>", "run": "<how to open or run the app>"}}"""

EVAL_PROMPT = """You are the evaluator in a test that compares two implementations of the same app.
Read BRIEF.md. It holds the app description and the checklist. The two implementations are in the folders A and B. Each folder holds the source files, RESULT.json from its author, and screenshot.png when a screenshot could be made. NOTE.txt in a folder says when no screenshot exists.

Judge each checklist item for A and for B separately. Decide from the source code and, where present, the screenshot. Trace the code to decide behaviour (button presses, state, validation). Judge only what the checklist item says. Do not judge style, code quality or language choice. When you cannot decide an item from the evidence, mark it false and say why.

Write VERDICT.json with exactly this shape:
{"A": {"items": [{"id": "<item id>", "pass": true, "why": "<one sentence>"}]}, "B": {"items": [...]}}
Include every checklist item for both A and B, in checklist order. Write nothing else."""


# ---------------------------------------------------------------- helpers

class Out:
    def __init__(self, events):
        self.events = events

    def emit(self, event, **kw):
        if self.events:
            print(json.dumps({"event": event, **kw}), flush=True)
        else:
            text = kw.get("text") or " ".join(f"{k}={v}" for k, v in kw.items())
            print(f"[{event}] {text}", flush=True)


def run(argv, cwd, timeout=600):
    return subprocess.run(argv, cwd=cwd, capture_output=True, text=True, timeout=timeout)


def sha256_file(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def sha256_text(t):
    return hashlib.sha256(t.encode()).hexdigest()


def slug(s):
    return re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-")


def fmt(n):
    return "—" if n is None else f"{n:,}"


def child_env():
    env = dict(os.environ)
    for k in PARENT_VARS:
        env.pop(k, None)
    return env


_ENC = None


def count_tokens(text):
    """o200k_base count, the measure the README and G2 use; None when tiktoken is not installed."""
    global _ENC
    if _ENC is None:
        try:
            import tiktoken
            _ENC = tiktoken.get_encoding("o200k_base")
        except Exception:
            _ENC = False
    return len(_ENC.encode(text)) if _ENC else None


# ---------------------------------------------------------------- machine and inputs

def ram_gb():
    try:
        if platform.system() == "Darwin":
            return round(int(run(["sysctl", "-n", "hw.memsize"], REPO, 10).stdout) / 2**30, 1)
        return round(os.sysconf("SC_PAGE_SIZE") * os.sysconf("SC_PHYS_PAGES") / 2**30, 1)
    except Exception:
        return None


def cpu_name():
    try:
        if platform.system() == "Darwin":
            return run(["sysctl", "-n", "machdep.cpu.brand_string"], REPO, 10).stdout.strip()
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.startswith("model name"):
                return line.split(":", 1)[1].strip()
    except Exception:
        pass
    return platform.processor() or platform.machine()


def machine_info():
    hashes = {f"pilot/agent-context/{f}": sha256_file(CONTEXT / f) for f in CONTEXT_FILES}
    hashes["pilot/owner-test/run.py"] = sha256_file(__file__)
    hashes["pilot/owner-test/tests.json"] = sha256_file(HERE / "tests.json")
    return {
        "os": platform.platform(), "cpu": cpu_name(), "cpus": os.cpu_count(), "ram_gb": ram_gb(),
        "python": platform.python_version(),
        "claude_version": run(["claude", "--version"], REPO, 60).stdout.strip(),
        "git_commit": run(["git", "rev-parse", "HEAD"], REPO, 30).stdout.strip(),
        "git_dirty": bool(run(["git", "status", "--porcelain", "--", ".", ":!tests-run"], REPO, 30).stdout.strip()),
        "branch": run(["git", "rev-parse", "--abbrev-ref", "HEAD"], REPO, 30).stdout.strip(),
        "hashes": hashes,
    }


def build_tools(out):
    out.emit("build", text="Building pvc-validate, pvc-compile and pvc_shot (cargo, release)")
    for argv in (
        ["cargo", "build", "--release", "-p", "pvc-ir", "--bins"],
        ["cargo", "build", "--release", "-p", "pvc-runtime", "--features", "cpu", "--example", "pvc_shot"],
    ):
        r = run(argv, REPO, timeout=1800)
        if r.returncode != 0:
            raise SystemExit(f"build failed: {' '.join(argv)}\n{r.stderr[-3000:]}")
    exe = ".exe" if os.name == "nt" else ""
    rel = REPO / "target" / "release"
    tools = {"validate": rel / f"pvc-validate{exe}", "compile": rel / f"pvc-compile{exe}",
             "shot": rel / "examples" / f"pvc_shot{exe}"}
    for p in tools.values():
        if not p.exists():
            raise SystemExit(f"missing tool after build: {p}")
    return tools


def cli_flags():
    """The optional flags this Claude Code version has."""
    help_text = run(["claude", "--help"], REPO, timeout=60).stdout
    return [f for f in ("--restricted", "--strict-mcp-config", "--effort") if f in help_text]


# ---------------------------------------------------------------- one agent session

def claude(prompt, cwd, model, effort, tools, iso, budget):
    argv = ["claude", "-p", prompt, "--output-format", "stream-json", "--verbose",
            "--session-id", str(uuid.uuid4()),
            "--permission-mode", "acceptEdits",
            "--allowedTools", *tools,
            "--disallowedTools", "WebFetch", "WebSearch", "Task", "Agent"]
    if "--restricted" in iso:
        argv += ["--restricted", "--tools", ",".join(tools)]
    if "--strict-mcp-config" in iso:
        argv += ["--strict-mcp-config"]
    if effort and "--effort" in iso:
        argv += ["--effort", effort]
    if model:
        argv += ["--model", model]
    if budget:
        argv += ["--max-budget-usd", str(budget)]
    session = {"prompt": prompt, "flags": [a for a in argv[3:]]}
    try:
        r = subprocess.run(argv, cwd=cwd, capture_output=True, text=True,
                           timeout=AGENT_TIMEOUT_S, env=child_env())
        stream, err, code = r.stdout, r.stderr, r.returncode
    except subprocess.TimeoutExpired as e:
        stream = e.stdout.decode() if isinstance(e.stdout, bytes) else (e.stdout or "")
        err, code = f"timed out after {AGENT_TIMEOUT_S} s", None
    events = []
    for line in stream.splitlines():
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    session["stream"] = stream
    session["trace"] = trace_of(events)
    result = next((e for e in reversed(events) if e.get("type") == "result"), None)
    if result is None:
        session.update(ok=False, error=(err or "").strip()[-2000:] or f"no result (exit code {code})")
        return session
    session.update(usage_of(result))
    session["ok"] = not result.get("is_error", False)
    session["final_message"] = result.get("result")
    if not session["ok"]:
        session["error"] = f"{result.get('subtype')}: {result.get('result') or result.get('terminal_reason')}"
    return session


def usage_of(result):
    """Sum Claude Code's per-model usage for the whole session."""
    t = {"input": 0, "output": 0, "cache_read": 0, "cache_write": 0, "cost_usd": 0.0}
    models = []
    for name, m in (result.get("modelUsage") or {}).items():
        models.append(name)
        t["input"] += m.get("inputTokens", 0)
        t["output"] += m.get("outputTokens", 0)
        t["cache_read"] += m.get("cacheReadInputTokens", 0)
        t["cache_write"] += m.get("cacheCreationInputTokens", 0)
        t["cost_usd"] += m.get("costUSD", 0.0)
    t["total"] = t["input"] + t["output"] + t["cache_read"] + t["cache_write"]
    t["models"] = models
    t["turns"] = result.get("num_turns")
    t["duration_s"] = round((result.get("duration_ms") or 0) / 1000)
    return t


def _text(content):
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(b.get("text", "") for b in content if isinstance(b, dict))
    return ""


def trace_of(events):
    """Tool calls in order, with each result's text, from a stream-json transcript."""
    calls, by_id = [], {}
    for e in events:
        msg = e.get("message") if isinstance(e.get("message"), dict) else {}
        content = msg.get("content") if isinstance(msg.get("content"), list) else []
        for b in content:
            if b.get("type") == "tool_use":
                c = {"tool": b.get("name"), "input": b.get("input") or {}, "result": "", "is_error": False}
                calls.append(c)
                by_id[b.get("id")] = c
            elif b.get("type") == "tool_result" and b.get("tool_use_id") in by_id:
                c = by_id[b["tool_use_id"]]
                c["result"] = _text(b.get("content"))
                c["is_error"] = bool(b.get("is_error"))
    return calls


def checks_of(trace, arm):
    """Fix attempts: the check commands an agent ran and how each ended."""
    runs = []
    for c in trace:
        if c["tool"] != "Bash":
            continue
        cmd = str(c["input"].get("command", ""))
        if arm == "pvc" and "pvc-validate" not in cmd:
            continue
        failed = c["is_error"] or "FAIL errors=" in c["result"]
        codes = sorted(set(re.findall(r"^E (PVC-E-[A-Z0-9-]+)", c["result"], re.M)))
        runs.append({"command": cmd[:200], "failed": failed, "codes": codes})
    return runs


# ---------------------------------------------------------------- screenshots and files

def find_browser():
    for n in ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser", "chrome", "msedge"]:
        p = shutil.which(n)
        if p:
            return p
    for p in ("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
              "/Applications/Chromium.app/Contents/MacOS/Chromium",
              "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
              "/opt/pw-browsers/chromium"):
        if Path(p).exists():
            return p
    return None


def read_result_json(folder):
    try:
        return json.loads((folder / "RESULT.json").read_text())
    except Exception:
        return {}


def screenshot_control(work, dest, browser):
    res = read_result_json(work)
    entry = work / res["entry"] if res.get("entry") else None
    if not entry or not entry.is_file():
        entry = next((work / c for c in ("index.html", "app.html") if (work / c).is_file()), None)
    if not entry or entry.suffix.lower() not in (".html", ".htm"):
        return f"No screenshot: the entry is not an HTML file ({res.get('entry')!r})."
    if not browser:
        return "No screenshot: no Chrome, Chromium or Edge was found on this machine."
    with tempfile.TemporaryDirectory() as prof:
        root = ["--no-sandbox"] if hasattr(os, "geteuid") and os.geteuid() == 0 else []
        r = run([browser, "--headless=new", "--disable-gpu", "--no-first-run", *root, f"--user-data-dir={prof}",
                 "--window-size=1280,800", f"--screenshot={dest / 'screenshot.png'}",
                 entry.resolve().as_uri()], work, timeout=120)
    if not (dest / "screenshot.png").exists():
        return f"No screenshot: the browser failed ({r.stderr.strip()[-300:]})."
    return None


def screenshot_pvc(work, dest, tools):
    if not (work / "app.pvc").exists():
        return "No screenshot: the agent wrote no app.pvc."
    v = run([str(tools["validate"]), "app.pvc"], work, timeout=60)
    (dest / "validator.txt").write_text(v.stdout + v.stderr)
    if v.returncode != 0:
        return "No screenshot: app.pvc does not pass the validator (see validator.txt)."
    c = run([str(tools["compile"]), "app.pvc", "app.pvca"], work, timeout=60)
    if c.returncode != 0:
        return f"No screenshot: compile failed ({c.stderr.strip()[-300:]})."
    s = run([str(tools["shot"]), "app.pvca", str(dest / "screenshot.png")], work, timeout=120)
    if s.returncode == 3:
        return ("No screenshot: the PVC runtime cannot draw this app yet (" + s.stderr.strip()[-200:] +
                "). Judged from the source code only.")
    if s.returncode != 0:
        return f"No screenshot: render failed ({s.stderr.strip()[-300:]})."
    return None


def copy_results(work, dest):
    dest.mkdir(parents=True, exist_ok=True)
    for p in sorted(work.rglob("*")):
        rel = p.relative_to(work)
        if any(part in ("node_modules", ".git") for part in rel.parts):
            continue
        if p.is_file() and p.name not in TOOL_BINARIES and p.name not in CONTEXT_FILES and p.stat().st_size < 2_000_000:
            q = dest / rel
            q.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(p, q)


def code_files(folder):
    """The code an agent produced: text files it wrote, minus the harness's own files."""
    files = []
    for p in sorted(folder.rglob("*")):
        if not p.is_file() or p.name in NOT_CODE or p.suffix.lower() in (".png", ".jpg", ".jpeg", ".gif", ".ico"):
            continue
        try:
            text = p.read_text()
        except (UnicodeDecodeError, OSError):
            continue
        files.append({"path": str(p.relative_to(folder)), "text": text, "lines": text.count("\n") + 1,
                      "bytes": len(text.encode()), "tokens": count_tokens(text),
                      "lang": CODE_LANG.get(p.suffix.lower(), "")})
    return files


# ---------------------------------------------------------------- one test

def run_test(t, args, tools, iso, browser, machine, out):
    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")
    name = f"{stamp}_{t['id']}_{slug(t['title'])}"
    tdir = OUT / name
    raw = tdir / "raw"
    raw.mkdir(parents=True)

    # 1. PVC agent
    pvc_work = Path(tempfile.mkdtemp(prefix=f"pvc-test-{t['id']}-pvc-"))
    for f in CONTEXT_FILES:
        shutil.copy2(CONTEXT / f, pvc_work / f)
    exe = ".exe" if os.name == "nt" else ""
    shutil.copy2(tools["validate"], pvc_work / f"pvc-validate{exe}")
    shutil.copy2(tools["compile"], pvc_work / f"pvc-compile{exe}")
    out.emit("agent_start", test=t["id"], arm="pvc", text=f"{t['id']} PVC agent started")
    pvc = claude(PVC_PROMPT.format(description=t["description"]), pvc_work, args.model, args.effort, TOOLS, iso, args.budget)
    out.emit("agent_done", test=t["id"], arm="pvc", ok=pvc["ok"], total=pvc.get("total"),
             text=f"{t['id']} PVC agent done: {fmt(pvc.get('total'))} tokens")

    # 2. Control agent
    ctl_work = Path(tempfile.mkdtemp(prefix=f"pvc-test-{t['id']}-control-"))
    out.emit("agent_start", test=t["id"], arm="control", text=f"{t['id']} control agent started")
    ctl = claude(CONTROL_PROMPT.format(description=t["description"]), ctl_work, args.model, args.effort, TOOLS, iso, args.budget)
    out.emit("agent_done", test=t["id"], arm="control", ok=ctl["ok"], total=ctl.get("total"),
             text=f"{t['id']} control agent done: {fmt(ctl.get('total'))} tokens")

    # Outputs and screenshots
    pvc_dest, ctl_dest = tdir / "pvc", tdir / "control"
    copy_results(pvc_work, pvc_dest)
    copy_results(ctl_work, ctl_dest)
    pvc_note = screenshot_pvc(pvc_work, pvc_dest, tools)
    ctl_note = screenshot_control(ctl_work, ctl_dest, browser)
    for dest, note in ((pvc_dest, pvc_note), (ctl_dest, ctl_note)):
        if note:
            (dest / "NOTE.txt").write_text(note + "\n")

    # 3. Evaluator: blind to tokens; A/B order random and recorded
    ev_work = Path(tempfile.mkdtemp(prefix=f"pvc-test-{t['id']}-eval-"))
    order = ["pvc", "control"]
    random.shuffle(order)
    labels = {"A": order[0], "B": order[1]}
    for lab, arm in labels.items():
        shutil.copytree(pvc_dest if arm == "pvc" else ctl_dest, ev_work / lab)
    items = "\n".join(f"- **{c['id']}**: {c['text']}" for c in t["checklist"])
    brief = f"# {t['title']}\n\n## App description\n\n{t['description']}\n\n## Checklist\n\n{items}\n"
    (ev_work / "BRIEF.md").write_text(brief)
    out.emit("agent_start", test=t["id"], arm="evaluator", text=f"{t['id']} evaluator started")
    ev = claude(EVAL_PROMPT, ev_work, args.eval_model or args.model, args.effort, EVAL_TOOLS, iso, args.budget)
    try:
        verdict = json.loads((ev_work / "VERDICT.json").read_text())
    except Exception:
        verdict = None
    out.emit("agent_done", test=t["id"], arm="evaluator", ok=verdict is not None,
             text=f"{t['id']} evaluator done")

    for nm, s in (("pvc", pvc), ("control", ctl), ("evaluator", ev)):
        (raw / f"{nm}.stream.jsonl").write_text(s.get("stream", ""))
    (raw / "BRIEF.md").write_text(brief)
    (raw / "VERDICT.json").write_text(json.dumps({"labels": labels, "verdict": verdict}, indent=2))

    def score(arm):
        if verdict is None:
            return None
        lab = next(k for k, v in labels.items() if v == arm)
        its = (verdict.get(lab) or {}).get("items", [])
        return {"passed": sum(1 for i in its if i.get("pass") is True), "of": len(t["checklist"]), "items": its}

    hashes = dict(machine["hashes"])
    hashes[f"tests.json → {t['id']} (canonical JSON)"] = sha256_text(json.dumps(t, sort_keys=True))
    r = {
        "name": name, "stamp": stamp, "test": t, "labels": labels,
        "pvc": pvc, "control": ctl, "evaluator": ev,
        "pvc_note": pvc_note, "control_note": ctl_note,
        "pvc_code": code_files(pvc_dest), "control_code": code_files(ctl_dest),
        "pvc_checks": checks_of(pvc["trace"], "pvc"), "control_checks": checks_of(ctl["trace"], "control"),
        "control_result": read_result_json(ctl_dest),
        "pvc_score": score("pvc"), "control_score": score("control"),
        "args": {"model": args.model, "eval_model": args.eval_model, "effort": args.effort, "budget": args.budget},
        "machine": dict(machine, hashes=hashes), "iso": iso, "browser": browser,
    }
    (OUT / f"{name}.md").write_text(render_markdown(r))
    append_index(r)
    for d in (pvc_work, ctl_work, ev_work):
        shutil.rmtree(d, ignore_errors=True)

    ps, cs = r["pvc_score"], r["control_score"]
    out.emit("test_done", test=t["id"], file=f"tests-run/{name}.md", result={
        "id": t["id"], "title": t["title"], "level": t["level"],
        "control_language": r["control_result"].get("language"), "pvc_rendered": pvc_note is None,
        "evaluator_ok": verdict is not None, "pvc_total": pvc.get("total"), "control_total": ctl.get("total"),
        "pvc_passed": ps["passed"] if ps else None, "control_passed": cs["passed"] if cs else None,
        "of": len(t["checklist"]), "pvc_ok": pvc["ok"], "control_ok": ctl["ok"]},
        text=f"{t['id']} done: tests-run/{name}.md")
    return r


# ---------------------------------------------------------------- the result file

def per_item(total, score):
    if not total or not score or not score["passed"]:
        return None
    return round(total / score["passed"])


def ratio(a, b):
    return f"{a / b:.2f}" if isinstance(a, (int, float)) and isinstance(b, (int, float)) and a and b else "—"


def verdict_line(r):
    p, c = r["pvc"].get("total"), r["control"].get("total")
    ps, cs = r["pvc_score"], r["control_score"]
    parts = []
    if p and c:
        if p < c:
            parts.append(f"PVC used **{(c - p) / c:.0%} fewer** tokens than the control ({fmt(p)} vs {fmt(c)})")
        elif p > c:
            parts.append(f"PVC used **{(p - c) / c:.0%} more** tokens than the control ({fmt(p)} vs {fmt(c)})")
        else:
            parts.append(f"both agents used {fmt(p)} tokens")
    else:
        parts.append("no token comparison is possible (an agent session failed)")
    if ps and cs:
        if ps["passed"] == cs["passed"]:
            parts.append(f"both passed {ps['passed']}/{ps['of']} checklist items")
        else:
            better = "PVC" if ps["passed"] > cs["passed"] else "the control"
            parts.append(f"{better} passed more checklist items (PVC {ps['passed']}/{ps['of']}, "
                         f"control {cs['passed']}/{cs['of']})")
    else:
        parts.append("the checklist was not judged")
    if r["pvc_note"]:
        parts.append("the PVC app was not drawn")
    return "; ".join(parts) + "."


def cell(text):
    return str(text).replace("|", "/").replace("\n", " ")


def render_markdown(r):
    t, pvc, ctl, ev = r["test"], r["pvc"], r["control"], r["evaluator"]
    ps, cs = r["pvc_score"], r["control_score"]
    m, a, name = r["machine"], r["args"], r["name"]
    L = []
    w = L.append

    w(f"# {t['id']} {t['title']} — {r['stamp']} UTC")
    w("")
    w(f"**Verdict:** {verdict_line(r)}")
    w("")

    w("## Scope")
    w("")
    w(f"- **Level:** {t['level']}")
    w(f"- **Description sent to both agents:** {t['description']}")
    w(f"- **PVC app drawn by the runtime:** {'yes' if not r['pvc_note'] else 'no. ' + r['pvc_note']}")
    w("")

    w("## Agents and models")
    w("")
    w("| Agent | Role | Models used (Claude Code report) | Effort | Finished normally |")
    w("|---|---|---|---|---|")
    eff = a["effort"] or "Claude Code default"
    for nm, role, s in (("PVC", "Writes the app in PVC-IR, checked by the validator", pvc),
                        ("Control", "Writes the app in a language it picks", ctl),
                        ("Evaluator", "Marks both results against the checklist; never sees tokens", ev)):
        fin = "yes" if s.get("ok") else f"no: {cell(s.get('error', 'failed'))[:200]}"
        w(f"| {nm} | {role} | {', '.join(s.get('models') or []) or '—'} | {eff} | {fin} |")
    w("")
    w(f"Model requested: `{a['model'] or 'Claude Code default'}`; evaluator: "
      f"`{a['eval_model'] or a['model'] or 'Claude Code default'}`. "
      f"Budget cap per session: {('$' + str(a['budget'])) if a['budget'] else 'none'}.")
    w("")
    w(f"**Control's language:** {r['control_result'].get('language') or '—'}. "
      f"**Reason given:** {r['control_result'].get('why') or '—'}")
    w("")

    w("## Tokens, cost and time")
    w("")
    w("| | PVC | Control | PVC ÷ control |")
    w("|---|---|---|---|")
    for label, key in (("**Total tokens**", "total"), ("Input", "input"), ("Output", "output"),
                       ("Cache read", "cache_read"), ("Cache write", "cache_write"),
                       ("Agent turns", "turns"), ("Time (s)", "duration_s")):
        w(f"| {label} | {fmt(pvc.get(key))} | {fmt(ctl.get(key))} | {ratio(pvc.get(key), ctl.get(key))} |")
    pc, cc = pvc.get("cost_usd"), ctl.get("cost_usd")
    w(f"| Cost (USD) | {'—' if pc is None else f'${pc:.4f}'} | {'—' if cc is None else f'${cc:.4f}'} | {ratio(pc, cc)} |")
    pi, ci = per_item(pvc.get("total"), ps), per_item(ctl.get("total"), cs)
    w(f"| Tokens per checklist item passed | {fmt(pi)} | {fmt(ci)} | {ratio(pi, ci)} |")
    counted = all(f["tokens"] is not None for f in r["pvc_code"] + r["control_code"])
    pct = sum(f["tokens"] for f in r["pvc_code"]) if counted else None
    cct = sum(f["tokens"] for f in r["control_code"]) if counted else None
    w(f"| Final code size (o200k_base tokens) | {fmt(pct)} | {fmt(cct)} | {ratio(pct, cct)} |")
    pl, cl = sum(f["lines"] for f in r["pvc_code"]), sum(f["lines"] for f in r["control_code"])
    w(f"| Final code size (lines) | {fmt(pl)} | {fmt(cl)} | {ratio(pl, cl)} |")
    w("")
    w("Total tokens are Claude Code's own report for each session: input, output, cache read and cache write, summed. "
      "Both agents carry the same Claude Code system prompt. The PVC total also includes reading the three PVC "
      "context files. Final code size counts every text file the agent left, except RESULT.json. "
      f"The evaluator's tokens are not part of the comparison (evaluator: {fmt(ev.get('total'))} tokens, "
      f"${ev.get('cost_usd') or 0:.4f}).")
    if not counted:
        w("")
        w("Code size in tokens was not counted: install `tiktoken` (`pip install tiktoken`) and run again.")
    w("")

    w("## Checklist, item by item")
    w("")
    if ps and cs:
        w("| Item | Requirement | PVC | Control |")
        w("|---|---|---|---|")
        pmap = {i.get("id"): i for i in ps["items"]}
        cmap = {i.get("id"): i for i in cs["items"]}

        def mark(x):
            if not x:
                return "not judged"
            return f"{'✅ PASS' if x.get('pass') else '❌ FAIL'}: {cell(x.get('why', ''))}"
        for c in t["checklist"]:
            w(f"| {c['id']} | {cell(c['text'])} | {mark(pmap.get(c['id']))} | {mark(cmap.get(c['id']))} |")
        w(f"| **Total** | | **{ps['passed']}/{ps['of']}** | **{cs['passed']}/{cs['of']}** |")
    else:
        w("Not judged: the evaluator wrote no VERDICT.json." + (f" Error: {ev.get('error')}" if ev.get("error") else ""))
    w("")
    w(f"Blind labels for the evaluator: A = {r['labels']['A']}, B = {r['labels']['B']}.")
    w("")

    w("## Screenshots")
    w("")
    w("| PVC | Control |")
    w("|---|---|")

    def shot(arm, note):
        return cell(note) if note else f"![{arm} screenshot]({name}/{arm}/screenshot.png)"
    w(f"| {shot('pvc', r['pvc_note'])} | {shot('control', r['control_note'])} |")
    w("")

    w("## Fix attempts")
    w("")
    pch, cch = r["pvc_checks"], r["control_checks"]
    w(f"- **PVC:** ran the validator {len(pch)} time(s); {sum(1 for c in pch if c['failed'])} run(s) failed.")
    for i, c in enumerate(pch, 1):
        w(f"  {i}. {'FAIL' if c['failed'] else 'pass'}; error codes: {', '.join(c['codes']) or '—'}")
    w(f"- **Control:** ran {len(cch)} shell command(s) to build or check its work; "
      f"{sum(1 for c in cch if c['failed'])} ended in an error.")
    for i, c in enumerate(cch, 1):
        w(f"  {i}. {'error' if c['failed'] else 'ok'}: `{c['command'].replace('`', '')[:120]}`")
    w("")

    w("## Each agent's final message")
    w("")
    for nm, s in (("PVC", pvc), ("Control", ctl), ("Evaluator", ev)):
        w(f"**{nm}:**")
        w("")
        w("> " + (s.get("final_message") or "—").strip().replace("\n", "\n> "))
        w("")

    w("## Errors and failures")
    w("")
    errs = [(nm, s) for nm, s in (("PVC", pvc), ("Control", ctl), ("Evaluator", ev)) if not s.get("ok")]
    for nm, s in errs:
        w(f"- **{nm}:** {s.get('error', 'failed')}")
    if not errs:
        w("None. Every agent session finished normally.")
    w("")

    w("## Code produced")
    w("")
    for nm, files in (("PVC", r["pvc_code"]), ("Control", r["control_code"])):
        w(f"### {nm}")
        w("")
        if not files:
            w("No code files.")
            w("")
        for f in files:
            w(f"`{f['path']}`: {f['lines']} lines, {fmt(f['bytes'])} bytes, {fmt(f['tokens'])} tokens")
            w("")
            fence = "````" if "```" in f["text"] else "```"
            w(f"{fence}{f['lang']}")
            w(f["text"].rstrip("\n"))
            w(fence)
            w("")

    w("## Exact prompts")
    w("")
    for nm, s in (("PVC agent", pvc), ("Control agent", ctl), ("Evaluator", ev)):
        w(f"<details><summary>{nm}</summary>")
        w("")
        w("```text")
        w(s.get("prompt", ""))
        w("```")
        w("")
        w("Command-line flags: `" + " ".join(f for f in s.get("flags", []) if f != s.get("prompt")) + "`")
        w("")
        w("</details>")
        w("")

    w("## Versions, machine and inputs")
    w("")
    w(f"- **Claude Code:** {m['claude_version']}")
    w(f"- **Optional flags this version has:** {', '.join(r['iso']) or 'none'}")
    w(f"- **OS:** {m['os']}")
    w(f"- **CPU:** {m['cpu']} ({m['cpus']} logical CPUs). **RAM:** {m['ram_gb']} GB")
    w(f"- **Python:** {m['python']}. **Browser for screenshots:** {r['browser'] or 'none found'}")
    w(f"- **Repository:** commit `{m['git_commit']}` on `{m['branch']}`"
      f"{' (with uncommitted changes outside tests-run/)' if m['git_dirty'] else ''}")
    w("")
    w("| Input | SHA-256 |")
    w("|---|---|")
    for k, v in m["hashes"].items():
        w(f"| `{k}` | `{v}` |")
    w("")

    w("## Raw data")
    w("")
    w(f"- PVC session transcript: [{name}/raw/pvc.stream.jsonl]({name}/raw/pvc.stream.jsonl)")
    w(f"- Control session transcript: [{name}/raw/control.stream.jsonl]({name}/raw/control.stream.jsonl)")
    w(f"- Evaluator session transcript: [{name}/raw/evaluator.stream.jsonl]({name}/raw/evaluator.stream.jsonl)")
    w(f"- Evaluator brief and verdict: [{name}/raw/BRIEF.md]({name}/raw/BRIEF.md), "
      f"[{name}/raw/VERDICT.json]({name}/raw/VERDICT.json)")
    w(f"- Every file each agent left: [{name}/pvc/]({name}/pvc/), [{name}/control/]({name}/control/)")
    w("")

    w("## Limits")
    w("")
    w("- One run of one test with one model. This shows a direction, not a general result.")
    w("- The evaluator is a language model that reads code and screenshots. It is not the automated checker the G2 pilot specifies.")
    w("- Each agent ran in a new temporary folder outside the repository, with web tools off.")
    return "\n".join(L) + "\n"


def append_index(r):
    idx = OUT / "INDEX.md"
    if not idx.exists():
        idx.write_text(
            "# Test runs\n\nOne row per test run, oldest first. Each linked file holds the full detail.\n\n"
            "| When (UTC) | Test | Model | Effort | PVC tokens | Control tokens | PVC ÷ control "
            "| PVC checklist | Control checklist | Control language | File |\n"
            "|---|---|---|---|---|---|---|---|---|---|---|\n")
    p, c = r["pvc"].get("total"), r["control"].get("total")
    ps, cs = r["pvc_score"], r["control_score"]
    model = ", ".join(r["pvc"].get("models") or []) or (r["args"]["model"] or "default")
    pst = "%d/%d" % (ps["passed"], ps["of"]) if ps else "—"
    cst = "%d/%d" % (cs["passed"], cs["of"]) if cs else "—"
    with idx.open("a") as f:
        f.write(f"| {r['stamp']} | {r['test']['id']} {r['test']['title']} | {model} | "
                f"{r['args']['effort'] or 'default'} | {fmt(p)} | {fmt(c)} | {ratio(p, c)} | {pst} | {cst} | "
                f"{cell(r['control_result'].get('language') or '—')} | [{r['name']}.md]({r['name']}.md) |\n")


# ---------------------------------------------------------------- main

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("tests", nargs="*", help="test ids (default: all)")
    ap.add_argument("--model", help="model for the PVC and control agents (default: Claude Code default)")
    ap.add_argument("--eval-model", help="model for the evaluator (default: same as --model)")
    ap.add_argument("--effort", choices=["low", "medium", "high", "xhigh", "max"],
                    help="effort level for every agent (default: Claude Code default)")
    ap.add_argument("--budget", type=float, help="maximum USD per agent session")
    ap.add_argument("--events", action="store_true", help="print JSON lines for the pvc-test mod")
    args = ap.parse_args()
    out = Out(args.events)

    spec = json.loads((HERE / "tests.json").read_text())
    wanted = [x.upper() for x in args.tests] or [t["id"] for t in spec["tests"]]
    tests = [t for t in spec["tests"] if t["id"] in wanted]
    unknown = set(wanted) - {t["id"] for t in tests}
    if unknown:
        raise SystemExit(f"unknown test id(s): {', '.join(sorted(unknown))}")
    if shutil.which("claude") is None:
        raise SystemExit("the claude command was not found on PATH")

    OUT.mkdir(exist_ok=True)
    out.emit("start", tests=[{"id": t["id"], "title": t["title"], "level": t["level"]} for t in tests],
             run_dir=str(OUT), text=f"Results folder: {OUT}")
    tools = build_tools(out)
    iso = cli_flags()
    if args.effort and "--effort" not in iso:
        out.emit("log", text="This Claude Code version has no --effort flag; the effort level is not set.")
        args.effort = None
    browser = find_browser()
    machine = machine_info()
    files = []
    for t in tests:
        try:
            files.append(f"tests-run/{run_test(t, args, tools, iso, browser, machine, out)['name']}.md")
        except Exception as e:  # keep the other tests running
            out.emit("test_error", test=t["id"], error=str(e), text=f"{t['id']} failed: {e}")
    out.emit("done", report="tests-run/INDEX.md", files=files, text="Index: tests-run/INDEX.md")


if __name__ == "__main__":
    main()
