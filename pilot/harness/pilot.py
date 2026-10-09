#!/usr/bin/env python3
"""G2 pilot harness (dev-pilot, B3).  Python 3 stdlib + tiktoken only.

Commands
  order                         print the pre-registered run order
  context-info --stack S        print the exact system message + o200k count for a stack
  run --arm A [--specimens ..] [--stacks ..] [--config C] [--out DIR] [--dry-run | --prereg-sha256 H]

Hard rules implemented here (see ../PREREG.md):
  * the whole `run` holds the CPU lock EXCLUSIVELY (real runs); dry runs/tests hold it SHARED per call
  * every chat message is saved verbatim, with per-segment o200k counts, to transcript.jsonl
  * real (non-dry) runs are refused unless the PREREG.md hash matches --prereg-sha256 and the config has no PLACEHOLDER
"""
import argparse, contextlib, fcntl, hashlib, json, os, pathlib, re, shutil, subprocess, sys, time, urllib.request, urllib.error

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent
PILOT_L = [f"L{i:02d}" for i in range(1, 11)]
PILOT_G = [f"G{i:02d}" for i in range(1, 6)]
STACK_ORDER = ["PVC", "B1", "B2"]
ARM_ORDER = ["S4", "S7"]
SEG_JOIN = "\n\n"

# ---- standard texts (verbatim in PREREG) ---------------------------------------------------
REPAIR_HEAD = "These checklist items failed:"
REPAIR_TAIL = "Fix them and reply with the complete corrected result in the same format."
PSEUDO = {
    "BUILD": "The result could not be built or loaded, so no checklist item could be checked.",
    "NO-ARTIFACT": "No complete code block in the required format was found in your reply.",
}
TOOL_MAX_LINES = 40
TOOL_MAX_LINE_CHARS = 300


def interleaved_order():
    """Alternate L and G specimens starting with L; when G runs out, the remaining L follow."""
    out = []
    for i in range(max(len(PILOT_L), len(PILOT_G))):
        if i < len(PILOT_L):
            out.append(PILOT_L[i])
        if i < len(PILOT_G):
            out.append(PILOT_G[i])
    return out


def now():
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime()) + f".{time.time_ns() % 10**9 // 10**6:03d}Z"


def sha256_file(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def sha256_text(t):
    return hashlib.sha256(t.encode("utf-8")).hexdigest()


_enc = None


def o200k(text):
    """o200k_base count of the exact text. Literal strings that look like special tokens are counted as ordinary text."""
    global _enc
    if _enc is None:
        import tiktoken
        _enc = tiktoken.get_encoding("o200k_base")
    return len(_enc.encode(text, disallowed_special=()))


def harness_hash():
    h = hashlib.sha256()
    files = sorted(p for p in HERE.rglob("*") if p.is_file() and "__pycache__" not in p.parts and p.suffix in (".py", ".json", ".md", ".sh"))
    for p in files:
        h.update(str(p.relative_to(HERE)).encode() + b"\0" + sha256_file(p).encode() + b"\n")
    return h.hexdigest()


# ---- CPU lock -------------------------------------------------------------------------------
class SharedLock:
    """CPU-lock wrapper (name kept for history).
    mode 'exclusive-run' (pilot runs; Orchestrator amendment): `flock -x` on the CPU lock file is taken ONCE and held for the whole `run`
        command (server start, every request, every tool/checker call, server stop); per-call `held()` is then a no-op, because a second
        flock on a new file descriptor from this same process would deadlock against our own exclusive lock.
    mode 'shared-per-call' (dry runs, tests): LOCK_SH around each request / tokenize / tool call, same semantics as `flock -s`."""

    def __init__(self, path, mode):
        assert mode in ("exclusive-run", "shared-per-call")
        self.path, self.mode = path, mode
        self.wait_s = 0.0
        self.fd = None

    def acquire_run(self):
        if self.mode == "exclusive-run":
            self.fd = os.open(self.path, os.O_RDONLY)
            t0 = time.time()
            fcntl.flock(self.fd, fcntl.LOCK_EX)
            self.wait_s += time.time() - t0

    def release_run(self):
        if self.fd is not None:
            fcntl.flock(self.fd, fcntl.LOCK_UN)
            os.close(self.fd)
            self.fd = None

    @contextlib.contextmanager
    def held(self):
        if self.mode == "exclusive-run":
            assert self.fd is not None, "exclusive run lock not held"
            yield
            return
        fd = os.open(self.path, os.O_RDONLY)
        t0 = time.time()
        try:
            fcntl.flock(fd, fcntl.LOCK_SH)
            self.wait_s += time.time() - t0
            yield
        finally:
            fcntl.flock(fd, fcntl.LOCK_UN)
            os.close(fd)


# ---- llama-server ---------------------------------------------------------------------------
class Server:
    def __init__(self, cfg, arm, lock, logdir):
        self.cfg, self.arm, self.lock = cfg, arm, lock
        a = cfg["arms"][arm]
        self.port = a["port"]
        self.base = f"http://127.0.0.1:{self.port}"
        self.proc = None
        self.logdir = logdir

    def start(self):
        a = self.cfg["arms"][self.arm]
        model = a["model"]
        want = a["sha256"]
        got = sha256_file(model)
        if got != want:
            raise SystemExit(f"model sha256 mismatch for {self.arm}: {got} != {want}")
        self.model_sha256 = got
        args = [self.cfg["llama_server"], "-m", model, "--host", "127.0.0.1", "--port", str(self.port)] + self.cfg["server_args"]
        self.logdir.mkdir(parents=True, exist_ok=True)
        self.logf = open(self.logdir / f"llama-server-{self.arm}.log", "ab")
        with self.lock.held():
            self.proc = subprocess.Popen(args, stdout=self.logf, stderr=subprocess.STDOUT)
            for _ in range(600):
                if self.proc.poll() is not None:
                    raise SystemExit("llama-server exited during start; see log")
                try:
                    with urllib.request.urlopen(self.base + "/health", timeout=2) as r:
                        if r.status == 200:
                            break
                except Exception:
                    time.sleep(1)
            else:
                raise SystemExit("llama-server did not become healthy")
        self.argv = args
        try:
            with urllib.request.urlopen(self.base + "/props", timeout=10) as r:
                self.props = json.loads(r.read())
        except Exception as e:  # recorded, not fatal
            self.props = {"error": str(e)}

    def stop(self):
        if self.proc and self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(60)
            except subprocess.TimeoutExpired:
                self.proc.kill()
        if getattr(self, "logf", None):
            self.logf.close()

    def post(self, path, body, timeout):
        req = urllib.request.Request(self.base + path, data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return json.loads(r.read())

    def tokenize_count(self, text):
        with self.lock.held():
            r = self.post("/tokenize", {"content": text, "add_special": False, "parse_special": False}, 120)
        return len(r["tokens"])

    def chat(self, messages):
        s = self.cfg["sampling"]
        a = self.cfg["arms"][self.arm]
        body = {"model": "pilot", "messages": messages, "stream": False,
                "temperature": s["temperature"], "top_k": s["top_k"], "top_p": s["top_p"], "min_p": s["min_p"],
                "repeat_penalty": s["repeat_penalty"], "seed": s["seed"], "max_tokens": s["max_tokens"],
                "cache_prompt": s["cache_prompt"]}
        if a.get("chat_template_kwargs"):
            body["chat_template_kwargs"] = a["chat_template_kwargs"]
        with self.lock.held():
            t0 = time.time()
            r = self.post("/v1/chat/completions", body, s["request_timeout_s"])
            wall = time.time() - t0
        return r, wall, body


# ---- stack handling ---------------------------------------------------------------------------
def context_text(cfg, stack):
    st = cfg["stacks"][stack]
    parts = []
    for f in st["context_files"]:
        p = pathlib.Path(f)
        if not p.is_absolute():
            p = HERE / p
        parts.append(p.read_text())
    txt = "\n\n".join(parts) if len(parts) > 1 else parts[0]
    for k, v in cfg.get("context_substitutions", {}).items():
        txt = txt.replace(k, v)
    return txt


def extract_artifact(text, tags):
    """Last complete fenced block whose info string is in `tags`; else last complete fenced block with an empty info string; else None."""
    blocks = [(m.group(1).strip().lower(), m.group(2)) for m in re.finditer(r"(?ms)^[ \t]{0,3}```([^\n`]*)\n(.*?)^[ \t]{0,3}```[ \t]*$", text)]
    tagged = [b for b in blocks if b[0] in tags]
    if tagged:
        return tagged[-1][1]
    bare = [b for b in blocks if b[0] == ""]
    return bare[-1][1] if bare else None


def trunc_tool(text):
    lines = [(l if len(l) <= TOOL_MAX_LINE_CHARS else l[:TOOL_MAX_LINE_CHARS] + "...") for l in text.rstrip("\n").split("\n")] if text.strip() else []
    if len(lines) > TOOL_MAX_LINES:
        extra = len(lines) - TOOL_MAX_LINES
        lines = lines[:TOOL_MAX_LINES] + [f"... ({extra} more lines omitted)"]
    return "\n".join(lines)


def run_cmd(argv, lock, timeout):
    t0 = time.time()
    with lock.held():
        try:
            p = subprocess.run(argv, capture_output=True, text=True, timeout=timeout)
            return p.returncode, p.stdout, p.stderr, time.time() - t0
        except subprocess.TimeoutExpired as e:
            return 124, (e.stdout or b"").decode() if isinstance(e.stdout, bytes) else (e.stdout or ""), "timeout", time.time() - t0


def fill(argv, **kw):
    return [a.format(**kw) for a in argv]


def parse_check(rc, out):
    """Checker contract (adapters/ADAPTERS.md): exit 0 pass, 1 checklist failure, 2 build/load failure.
    stdout lines 'FAIL<TAB>item-id<TAB>standard text'."""
    if rc == 0:
        return "pass", []
    if rc == 2:
        return "build_error", [("BUILD", PSEUDO["BUILD"])]
    if rc == 1:
        items = []
        for l in out.splitlines():
            if l.startswith("FAIL\t"):
                parts = l.split("\t", 2)
                if len(parts) == 3:
                    items.append((parts[1], parts[2]))
        if not items:
            raise RuntimeError("checker exit 1 without FAIL lines")
        return "fail", items
    raise RuntimeError(f"checker exit code {rc}")


def repair_text(items):
    return REPAIR_HEAD + "\n" + "\n".join(f"- {i}: {t}" for i, t in items) + "\n" + REPAIR_TAIL


# ---- one run ------------------------------------------------------------------------------------
def run_one(cfg, server, lock, arm, spec, stack, outdir, dry, hh):
    st = cfg["stacks"][stack]
    rdir = outdir / arm / f"{spec}__{stack}"
    tpath = rdir / "transcript.jsonl"
    if tpath.exists() and '"kind": "footer"' in tpath.read_text():
        print(f"skip (complete): {arm} {spec} {stack}")
        return
    if rdir.exists():
        shutil.rmtree(rdir)  # incomplete run restarts from scratch (PREREG: no partial resume)
    rdir.mkdir(parents=True)
    f = open(tpath, "a", buffering=1)
    seq = [0]

    def emit(rec):
        rec["seq"] = seq[0]
        seq[0] += 1
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")

    intent = (ROOT / "intents" / f"{spec}.txt").read_text()
    ctx = context_text(cfg, stack)
    run_id = f"{arm}__{spec}__{stack}"
    emit({"kind": "header", "run_id": run_id, "label": "DRY-RUN — not data" if dry else "PILOT RUN", "arm": arm, "specimen": spec,
          "stack": stack, "cpu_lock_mode": lock.mode, "started": now(), "model_file": cfg["arms"][arm]["model"], "model_sha256": server.model_sha256,
          "llama_server_argv": server.argv, "llama_build": cfg["llama_build"], "sampling": cfg["sampling"],
          "chat_template_kwargs": cfg["arms"][arm].get("chat_template_kwargs"), "max_turns": cfg["max_turns"],
          "harness_sha256": hh, "host": "container-xeon4 (4 vCPU Xeon 2.10GHz, CPU only), llama.cpp CPU, 4 threads",
          "loadavg_start": open("/proc/loadavg").read().strip(), "tokenizer": "o200k_base (tiktoken)",
          "context_sha256": sha256_text(ctx), "intent_sha256": sha256_text(intent), "server_props": server.props})

    def seg_record(segs):
        out = []
        for cat, text in segs:
            out.append({"category": cat, "text": text, "o200k": o200k(text), "model_tokens": server.tokenize_count(text)})
        return out

    def message_record(role, segs, turn, extra=None):
        segr = seg_record(segs)
        text = SEG_JOIN.join(t for _, t in segs)
        rec = {"kind": "message", "role": role, "turn": turn, "text": text, "o200k_message": o200k(text), "segments": segr}
        if extra:
            rec.update(extra)
        emit(rec)

    messages = [{"role": "system", "content": ctx}, {"role": "user", "content": intent}]
    message_record("system", [("context", ctx)], 1)
    message_record("user", [("request", intent)], 1)
    outcome, turns_used = "fail", 0
    for turn in range(1, cfg["max_turns"] + 1):
        turns_used = turn
        wait0 = lock.wait_s
        la = open("/proc/loadavg").read().strip()
        t_start = now()
        try:
            resp, wall, body = server.chat(messages)
        except urllib.error.HTTPError as e:
            if e.code == 400:  # request rejected by the server (e.g. prompt + max_tokens exceed the context size): counted as a failed run
                emit({"kind": "error", "turn": turn, "http_status": 400, "body": e.read().decode(errors="replace")[:2000], "ts": now()})
                outcome = "fail_server_rejected"
                break
            raise
        ch = resp["choices"][0]
        msg = ch["message"]
        content = msg.get("content") or ""
        reasoning = msg.get("reasoning_content") or ""
        r_segs = [("output", content)] + ([("output_reasoning", reasoning)] if reasoning else [])
        segr = [{"category": c, "text": t, "o200k": o200k(t)} for c, t in r_segs]
        emit({"kind": "message", "role": "assistant", "turn": turn, "text": content, "reasoning_text": reasoning,
              "o200k_message": o200k(content), "segments": segr, "finish_reason": ch.get("finish_reason"),
              "usage": resp.get("usage"), "timings": resp.get("timings"), "request_wall_s": round(wall, 3),
              "lock_wait_s": round(lock.wait_s - wait0, 3), "loadavg_at_turn": la, "ts_start": t_start, "ts_end": now(),
              "request_body_sampling": {k: v for k, v in body.items() if k != "messages"}})
        messages.append({"role": "assistant", "content": content})
        # --- extract, tools, check
        art = extract_artifact(content, st["fence_tags"])
        chk = {"kind": "check", "turn": turn, "finish_reason": ch.get("finish_reason")}
        tool_texts = []
        if art is None:
            status, items = "no_artifact", [("NO-ARTIFACT", PSEUDO["NO-ARTIFACT"])]
            chk.update(artifact_sha256=None)
        else:
            adir = rdir / f"turn{turn}"
            adir.mkdir()
            apath = adir / st["artifact_name"]
            apath.write_text(art)
            chk.update(artifact_sha256=sha256_text(art), artifact_path=str(apath.relative_to(outdir)))
            tools = []
            for tl in st["tools"]:
                rc, out, err, w = run_cmd(fill(tl["argv"], artifact=str(apath), dir=str(adir), id=spec), lock, tl.get("timeout_s", 600))
                shown = trunc_tool(out)
                tools.append({"name": tl["name"], "exit": rc, "stdout": out, "stderr": err, "wall_s": round(w, 3), "shown": shown})
                tool_texts.append(f"{tl['name']} output:\n{shown if shown else '(no findings)'}")
            chk["tools"] = tools
            rc, out, err, w = run_cmd(fill(st["check"]["argv"], artifact=str(apath), dir=str(adir), id=spec), lock, st["check"].get("timeout_s", 900))
            chk.update(check_exit=rc, check_stdout=out, check_stderr=err, check_wall_s=round(w, 3))
            status, items = parse_check(rc, out)
        chk.update(status=status, failed=[{"id": i, "text": t} for i, t in items])
        emit(chk)
        if status == "pass":
            outcome = "pass"
            break
        if turn == cfg["max_turns"]:
            break
        segs = []
        if tool_texts:
            segs.append(("tool", SEG_JOIN.join(tool_texts)))
        segs.append(("repair", repair_text(items)))
        message_record("user", segs, turn + 1)
        messages.append({"role": "user", "content": SEG_JOIN.join(t for _, t in segs)})
    emit({"kind": "footer", "outcome": outcome, "turns_used": turns_used, "ended": now(), "lock_wait_total_s": round(lock.wait_s, 3)})
    f.close()
    print(f"done: {run_id} outcome={outcome} turns={turns_used}")


# ---- main ---------------------------------------------------------------------------------------
def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("order")
    ci = sub.add_parser("context-info")
    ci.add_argument("--stack", required=True)
    ci.add_argument("--config", default=str(HERE / "config.json"))
    r = sub.add_parser("run")
    r.add_argument("--arm", required=True, choices=ARM_ORDER)
    r.add_argument("--specimens", nargs="*")
    r.add_argument("--stacks", nargs="*")
    r.add_argument("--config", default=str(HERE / "config.json"))
    r.add_argument("--out", required=True)
    r.add_argument("--dry-run", action="store_true")
    r.add_argument("--prereg-sha256")
    r.add_argument("--lock", choices=["exclusive-run", "shared-per-call"], help="default: exclusive-run for real runs, shared-per-call for --dry-run")
    a = ap.parse_args()

    if a.cmd == "order":
        for arm in ARM_ORDER:
            for s in interleaved_order():
                for st in STACK_ORDER:
                    print(arm, s, st)
        return
    cfg = json.loads(pathlib.Path(a.config).read_text())
    if a.cmd == "context-info":
        t = context_text(cfg, a.stack)
        print(t)
        print(f"---\nbytes={len(t.encode())} o200k_base={o200k(t)} sha256={sha256_text(t)}", file=sys.stderr)
        return
    if not a.dry_run:
        raw = pathlib.Path(a.config).read_text()
        if "PLACEHOLDER" in raw or any("{{" in context_text(cfg, k) for k in cfg["stacks"]):
            raise SystemExit("REFUSED: config/context still has PLACEHOLDER (FROZEN hash, library name, ...)")
        pre = ROOT / "PREREG.md"
        if not a.prereg_sha256 or sha256_file(pre) != a.prereg_sha256:
            raise SystemExit("REFUSED: real run needs --prereg-sha256 equal to the sha256 of PREREG.md")
    for k, st in cfg["stacks"].items():       # frozen-context check: sent bytes must equal what FROZEN.md says
        if "context_frozen_sha256" in st and not str(st["context_frozen_sha256"]).startswith("PLACEHOLDER"):
            t = context_text(cfg, k)
            if sha256_text(t) != st["context_frozen_sha256"] or o200k(t) != st["context_frozen_o200k"]:
                raise SystemExit(f"REFUSED: {k} context differs from FROZEN (sha256 {sha256_text(t)}, o200k {o200k(t)})")
    lock = SharedLock(cfg["cpu_lock"], a.lock or ("shared-per-call" if a.dry_run else "exclusive-run"))
    if not a.dry_run and lock.mode != "exclusive-run":
        raise SystemExit("REFUSED: real pilot runs take the CPU lock exclusively (Orchestrator amendment)")
    outdir = pathlib.Path(a.out)
    outdir.mkdir(parents=True, exist_ok=True)
    specs = a.specimens or interleaved_order()
    stacks = a.stacks or STACK_ORDER
    hh = harness_hash()
    server = Server(cfg, a.arm, lock, outdir / "_server")
    lock.acquire_run()
    try:
        server.start()
        for sp in specs:
            for stk in stacks:
                run_one(cfg, server, lock, a.arm, sp, stk, outdir, a.dry_run, hh)
    finally:
        server.stop()
        lock.release_run()


if __name__ == "__main__":
    main()
