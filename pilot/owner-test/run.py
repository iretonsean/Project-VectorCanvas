#!/usr/bin/env python3
"""Owner A/B test: PVC agent vs control agent, judged by an evaluator agent.

Run from the repository root:
    python3 pilot/owner-test/run.py [T1 T2 ...] [--model MODEL] [--events]

For each test, three headless Claude Code sessions run, each in its own new
folder outside the repository:
  1. PVC agent: gets the three PVC agent-context files and the validator and
     compiler, and writes app.pvc.
  2. Control agent: gets only the description, picks any language, and says
     which one and why.
  3. Evaluator: gets the description, the fixed checklist and both results
     (code and screenshots) under the neutral labels A and B, never the token
     counts, and marks each checklist item pass or fail.

Token counts come from Claude Code's own JSON report for each agent session
(modelUsage), never from the agents. Results go to
pilot/owner-test/runs/<UTC time>/ : REPORT.md, results.json and every file
the agents wrote (binaries left out).

--events prints one JSON object per line (used by the pvc-test mod); without
it, progress is printed as plain text. Standard library only.
"""
import argparse
import datetime as dt
import json
import os
import random
import shutil
import subprocess
import sys
import tempfile
import uuid
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
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
SKIP_COPY = {"pvc-validate", "pvc-compile", "pvc-validate.exe", "pvc-compile.exe"}

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
{{"A": {{"items": [{{"id": "<item id>", "pass": true, "why": "<one sentence>"}}]}}, "B": {{"items": [...]}}}}
Include every checklist item for both A and B, in checklist order. Write nothing else."""


class Out:
    def __init__(self, events):
        self.events = events

    def emit(self, event, **kw):
        if self.events:
            print(json.dumps({"event": event, **kw}), flush=True)
        else:
            text = kw.get("text") or " ".join(f"{k}={v}" for k, v in kw.items())
            print(f"[{event}] {text}", flush=True)


def child_env():
    env = dict(os.environ)
    for k in PARENT_VARS:
        env.pop(k, None)
    return env


def run(argv, cwd, timeout=600):
    return subprocess.run(argv, cwd=cwd, capture_output=True, text=True, timeout=timeout)


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
    tools = {
        "validate": rel / f"pvc-validate{exe}",
        "compile": rel / f"pvc-compile{exe}",
        "shot": rel / "examples" / f"pvc_shot{exe}",
    }
    for p in tools.values():
        if not p.exists():
            raise SystemExit(f"missing tool after build: {p}")
    return tools


def isolation_flags():
    """Flags that keep user settings, CLAUDE.md-style config and MCP servers out, if this CLI has them."""
    help_text = run(["claude", "--help"], REPO, timeout=60).stdout
    return [f for f in ("--restricted", "--strict-mcp-config") if f in help_text]


def claude(prompt, cwd, model, tools, iso, budget):
    argv = ["claude", "-p", prompt, "--output-format", "json",
            "--session-id", str(uuid.uuid4()),
            "--permission-mode", "acceptEdits",
            "--allowedTools", *tools,
            "--disallowedTools", "WebFetch", "WebSearch", "Task", "Agent"]
    if "--restricted" in iso:
        argv += ["--restricted", "--tools", ",".join(tools)]
    if "--strict-mcp-config" in iso:
        argv += ["--strict-mcp-config"]
    if model:
        argv += ["--model", model]
    if budget:
        argv += ["--max-budget-usd", str(budget)]
    try:
        r = subprocess.run(argv, cwd=cwd, capture_output=True, text=True,
                           timeout=AGENT_TIMEOUT_S, env=child_env())
    except subprocess.TimeoutExpired:
        return {"ok": False, "error": f"timed out after {AGENT_TIMEOUT_S} s"}
    try:
        data = json.loads(r.stdout)
    except json.JSONDecodeError:
        return {"ok": False, "error": (r.stderr or r.stdout)[-2000:], "exit": r.returncode}
    return {"ok": not data.get("is_error", False), "raw": data, **usage_of(data)}


def usage_of(data):
    """Sum Claude Code's per-model usage for the whole session."""
    t = {"input": 0, "output": 0, "cache_read": 0, "cache_write": 0, "cost_usd": 0.0}
    models = []
    for name, m in (data.get("modelUsage") or {}).items():
        models.append(name)
        t["input"] += m.get("inputTokens", 0)
        t["output"] += m.get("outputTokens", 0)
        t["cache_read"] += m.get("cacheReadInputTokens", 0)
        t["cache_write"] += m.get("cacheCreationInputTokens", 0)
        t["cost_usd"] += m.get("costUSD", 0.0)
    t["total"] = t["input"] + t["output"] + t["cache_read"] + t["cache_write"]
    t["uncached_in_plus_out"] = t["input"] + t["cache_write"] + t["output"]
    t["models"] = models
    t["turns"] = data.get("num_turns")
    t["duration_s"] = round((data.get("duration_ms") or 0) / 1000)
    return t


def find_browser():
    names = ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser", "chrome", "msedge"]
    for n in names:
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


def screenshot_control(work, dest, browser):
    try:
        res = json.loads((work / "RESULT.json").read_text())
        entry = work / res.get("entry", "")
    except Exception:
        res, entry = {}, None
    if not entry or not entry.is_file():
        for cand in ("index.html", "app.html"):
            if (work / cand).is_file():
                entry = work / cand
                break
    if not entry or not entry.is_file() or entry.suffix.lower() not in (".html", ".htm"):
        return f"No screenshot: the entry is not an HTML file ({res.get('entry')!r})."
    if not browser:
        return "No screenshot: no Chrome, Chromium or Edge was found on this machine."
    with tempfile.TemporaryDirectory() as prof:
        root = ["--no-sandbox"] if hasattr(os, "geteuid") and os.geteuid() == 0 else []
        r = run([browser, "--headless=new", "--disable-gpu", "--no-first-run", *root, f"--user-data-dir={prof}",
                 "--window-size=1280,800", f"--screenshot={dest / 'screenshot.png'}",
                 entry.resolve().as_uri()], work, timeout=120)
    if not (dest / "screenshot.png").exists():
        return f"No screenshot: the browser failed ({r.stderr[-300:]})."
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
        return f"No screenshot: compile failed ({c.stderr[-300:]})."
    s = run([str(tools["shot"]), "app.pvca", str(dest / "screenshot.png")], work, timeout=120)
    if s.returncode == 3:
        return ("No screenshot: the PVC runtime cannot draw this app yet (" + s.stderr.strip()[-200:] +
                "). Judge from the source code only.")
    if s.returncode != 0:
        return f"No screenshot: render failed ({s.stderr[-300:]})."
    return None


def copy_results(work, dest):
    dest.mkdir(parents=True, exist_ok=True)
    for p in work.rglob("*"):
        rel = p.relative_to(work)
        if any(part in ("node_modules", ".git") for part in rel.parts):
            continue
        if p.is_file() and p.name not in SKIP_COPY and p.name not in CONTEXT_FILES and p.stat().st_size < 2_000_000:
            q = dest / rel
            q.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(p, q)


def run_test(t, args, tools, iso, browser, run_dir, out):
    tdir = run_dir / t["id"]
    tdir.mkdir(parents=True)
    result = {"id": t["id"], "title": t["title"], "level": t["level"]}

    # 1. PVC agent
    pvc_work = Path(tempfile.mkdtemp(prefix=f"pvc-test-{t['id']}-pvc-"))
    for f in CONTEXT_FILES:
        shutil.copy2(CONTEXT / f, pvc_work / f)
    exe = ".exe" if os.name == "nt" else ""
    shutil.copy2(tools["validate"], pvc_work / f"pvc-validate{exe}")
    shutil.copy2(tools["compile"], pvc_work / f"pvc-compile{exe}")
    out.emit("agent_start", test=t["id"], arm="pvc", text=f"{t['id']} PVC agent started")
    pvc = claude(PVC_PROMPT.format(description=t["description"]), pvc_work, args.model, TOOLS, iso, args.budget)
    out.emit("agent_done", test=t["id"], arm="pvc", ok=pvc["ok"], total=pvc.get("total"),
             cost_usd=pvc.get("cost_usd"), text=f"{t['id']} PVC agent done: {pvc.get('total')} tokens")

    # 2. Control agent
    ctl_work = Path(tempfile.mkdtemp(prefix=f"pvc-test-{t['id']}-control-"))
    out.emit("agent_start", test=t["id"], arm="control", text=f"{t['id']} control agent started")
    ctl = claude(CONTROL_PROMPT.format(description=t["description"]), ctl_work, args.model, TOOLS, iso, args.budget)
    out.emit("agent_done", test=t["id"], arm="control", ok=ctl["ok"], total=ctl.get("total"),
             cost_usd=ctl.get("cost_usd"), text=f"{t['id']} control agent done: {ctl.get('total')} tokens")

    # Collect outputs and screenshots
    pvc_dest, ctl_dest = tdir / "pvc", tdir / "control"
    copy_results(pvc_work, pvc_dest)
    copy_results(ctl_work, ctl_dest)
    pvc_note = screenshot_pvc(pvc_work, pvc_dest, tools)
    ctl_note = screenshot_control(ctl_work, ctl_dest, browser)
    for dest, note in ((pvc_dest, pvc_note), (ctl_dest, ctl_note)):
        if note:
            (dest / "NOTE.txt").write_text(note + "\n")
    for name, r in (("pvc", pvc), ("control", ctl)):
        (tdir / f"{name}.claude.json").write_text(json.dumps(r.get("raw", r), indent=2))
    try:
        ctl_lang = json.loads((ctl_dest / "RESULT.json").read_text()).get("language")
    except Exception:
        ctl_lang = None

    # 3. Evaluator, blind to tokens; A/B order random and recorded
    ev_work = Path(tempfile.mkdtemp(prefix=f"pvc-test-{t['id']}-eval-"))
    order = ["pvc", "control"]
    random.shuffle(order)
    label = {"A": order[0], "B": order[1]}
    for lab, arm in label.items():
        shutil.copytree(pvc_dest if arm == "pvc" else ctl_dest, ev_work / lab)
    items = "\n".join(f"- **{c['id']}**: {c['text']}" for c in t["checklist"])
    (ev_work / "BRIEF.md").write_text(f"# {t['title']}\n\n## App description\n\n{t['description']}\n\n## Checklist\n\n{items}\n")
    out.emit("agent_start", test=t["id"], arm="evaluator", text=f"{t['id']} evaluator started")
    ev = claude(EVAL_PROMPT, ev_work, args.eval_model or args.model, EVAL_TOOLS, iso, args.budget)
    verdict = None
    try:
        verdict = json.loads((ev_work / "VERDICT.json").read_text())
    except Exception:
        pass
    (tdir / "evaluator.claude.json").write_text(json.dumps(ev.get("raw", ev), indent=2))
    if verdict is not None:
        (tdir / "VERDICT.json").write_text(json.dumps({"labels": label, "verdict": verdict}, indent=2))

    def score(arm):
        lab = next(k for k, v in label.items() if v == arm)
        its = (verdict or {}).get(lab, {}).get("items", [])
        return {"passed": sum(1 for i in its if i.get("pass") is True), "of": len(t["checklist"]), "items": its}

    result.update({
        "pvc": {k: v for k, v in pvc.items() if k != "raw"},
        "control": {k: v for k, v in ctl.items() if k != "raw"},
        "control_language": ctl_lang,
        "pvc_rendered": pvc_note is None,
        "pvc_note": pvc_note,
        "control_note": ctl_note,
        "evaluator_ok": verdict is not None,
        "evaluator_tokens": ev.get("total"),
        "labels": label,
        "pvc_score": score("pvc") if verdict else None,
        "control_score": score("control") if verdict else None,
    })
    summary = {k: result[k] for k in ("id", "title", "level", "control_language", "pvc_rendered", "evaluator_ok")}
    summary.update({
        "pvc_total": pvc.get("total"), "control_total": ctl.get("total"),
        "pvc_passed": (result["pvc_score"] or {}).get("passed"),
        "control_passed": (result["control_score"] or {}).get("passed"),
        "of": len(t["checklist"]),
        "pvc_ok": pvc["ok"], "control_ok": ctl["ok"]})
    out.emit("test_done", test=t["id"], result=summary, text=f"{t['id']} done")
    for d in (pvc_work, ctl_work, ev_work):
        shutil.rmtree(d, ignore_errors=True)
    return result


def fmt(n):
    return "—" if n is None else f"{n:,}"


def write_report(results, run_dir, meta):
    lines = [
        f"# Owner A/B test — {meta['started']} UTC", "",
        f"Model: `{meta['model'] or 'Claude Code default'}`. Evaluator model: `{meta['eval_model'] or meta['model'] or 'Claude Code default'}`. "
        f"Claude Code {meta['claude_version']}. Isolation flags: `{' '.join(meta['isolation']) or 'none (older CLI)'}`.", "",
        "Token counts are Claude Code's own report for each agent session (all four kinds summed: input, output, cache read, cache write). "
        "Both agents carry the same Claude Code system prompt, so the difference between them comes from the task and the language. "
        "The PVC count includes reading the three PVC context files.", "",
        "| Test | Level | PVC tokens | Control tokens | PVC ÷ control | Control language | PVC checklist | Control checklist | PVC drawn? |",
        "|---|---|---|---|---|---|---|---|---|",
    ]
    for r in results:
        p, c = r["pvc"].get("total"), r["control"].get("total")
        ratio = f"{p / c:.2f}" if p and c else "—"
        ps, cs = r["pvc_score"], r["control_score"]
        pst = "%d/%d" % (ps["passed"], ps["of"]) if ps else "not judged"
        cst = "%d/%d" % (cs["passed"], cs["of"]) if cs else "not judged"
        drawn = "yes" if r["pvc_rendered"] else "no"
        lang = r["control_language"] or "—"
        lines.append(f"| {r['id']} {r['title']} | {r['level']} | {fmt(p)} | {fmt(c)} | {ratio} | {lang} | {pst} | {cst} | {drawn} |")
    lines += ["", "## Detail", ""]
    for r in results:
        lines += [f"### {r['id']} {r['title']} (level {r['level']})", ""]
        for arm in ("pvc", "control"):
            u = r[arm]
            if not u.get("ok"):
                lines.append(f"- **{arm}**: the agent session failed: {u.get('error', 'is_error set')}")
                continue
            lines.append(
                f"- **{arm}**: {fmt(u.get('total'))} tokens (input {fmt(u.get('input'))}, output {fmt(u.get('output'))}, "
                f"cache read {fmt(u.get('cache_read'))}, cache write {fmt(u.get('cache_write'))}), "
                f"{u.get('turns')} turns, {u.get('duration_s')} s, ${u.get('cost_usd', 0):.4f}")
        if r["pvc_note"]:
            lines.append(f"- PVC screenshot: {r['pvc_note']}")
        if r["control_note"]:
            lines.append(f"- Control screenshot: {r['control_note']}")
        for arm, key in (("PVC", "pvc_score"), ("Control", "control_score")):
            sc = r[key]
            if not sc:
                lines.append(f"- {arm} checklist: not judged (the evaluator wrote no VERDICT.json)")
                continue
            for it in sc["items"]:
                lines.append(f"  - {arm} {it.get('id')}: {'PASS' if it.get('pass') else 'FAIL'} — {it.get('why', '')}")
        lines.append("")
    lines += [
        "## Limits", "",
        "- One run per test with one model. This shows a direction, not a general result.",
        "- The evaluator is a language model reading code and screenshots. It is not the automated checker the G2 pilot specifies.",
        "- Where the PVC runtime cannot draw an app yet (lists, grids, screen-size rules), PVC is judged from source code only.",
        "- Each agent ran in a new temporary folder outside the repository, with web tools off.",
    ]
    (run_dir / "REPORT.md").write_text("\n".join(lines) + "\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("tests", nargs="*", help="test ids (default: all)")
    ap.add_argument("--model", help="model for the PVC and control agents (default: Claude Code default)")
    ap.add_argument("--eval-model", help="model for the evaluator (default: same as --model)")
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

    started = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")
    run_dir = HERE / "runs" / started
    run_dir.mkdir(parents=True)
    out.emit("start", tests=[{"id": t["id"], "title": t["title"], "level": t["level"]} for t in tests],
             run_dir=str(run_dir), text=f"Results folder: {run_dir}")
    tools = build_tools(out)
    iso = isolation_flags()
    browser = find_browser()
    meta = {"started": started, "model": args.model, "eval_model": args.eval_model, "isolation": iso,
            "claude_version": run(["claude", "--version"], REPO, timeout=60).stdout.strip(),
            "browser": browser}
    results = []
    for t in tests:
        try:
            results.append(run_test(t, args, tools, iso, browser, run_dir, out))
        except Exception as e:  # keep the other tests running
            out.emit("test_error", test=t["id"], error=str(e), text=f"{t['id']} failed: {e}")
        (run_dir / "results.json").write_text(json.dumps({"meta": meta, "results": results}, indent=2))
        write_report(results, run_dir, meta)
    out.emit("done", report=str(run_dir / "REPORT.md"), text=f"Report: {run_dir / 'REPORT.md'}")


if __name__ == "__main__":
    main()
