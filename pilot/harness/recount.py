#!/usr/bin/env python3
"""recount.py <transcript.jsonl> [more.jsonl ...] [--csv out.csv]

Recomputes every o200k_base count from the saved message text (tiktoken, same tokenizer the harness uses),
compares with the counts stored in the transcript, and prints the T_total accounting of PREREG §5.
Exit status 1 if any stored count differs from the recount, or the message/segment texts are inconsistent.
Needs: python3, tiktoken (o200k_base).  Does not need llama-server or the model.
"""
import json, sys, csv, argparse

SEG_JOIN = "\n\n"


def load(p):
    return [json.loads(l) for l in open(p, encoding="utf-8") if l.strip()]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="+")
    ap.add_argument("--csv")
    a = ap.parse_args()
    import tiktoken
    enc = tiktoken.get_encoding("o200k_base")
    cnt = lambda t: len(enc.encode(t, disallowed_special=()))
    rows, bad = [], 0
    for fn in a.files:
        recs = load(fn)
        hdr = recs[0]
        assert hdr["kind"] == "header", "first record must be the header"
        foot = next((r for r in recs if r["kind"] == "footer"), None)
        msgs = [r for r in recs if r["kind"] == "message"]
        mism = []
        # recount every segment and every message
        for m in msgs:
            if m["role"] != "assistant" and SEG_JOIN.join(s["text"] for s in m["segments"]) != m["text"]:
                mism.append((m["seq"], "message text != joined segments"))
            if cnt(m["text"]) != m["o200k_message"]:
                mism.append((m["seq"], f"message o200k stored {m['o200k_message']} recount {cnt(m['text'])}"))
            for s in m["segments"]:
                c = cnt(s["text"])
                if c != s["o200k"]:
                    mism.append((m["seq"], f"{s['category']} stored {s['o200k']} recount {c}"))
        # accounting per PREREG §5
        sysm = next(m for m in msgs if m["role"] == "system")
        reqm = next(m for m in msgs if m["role"] == "user" and m["turn"] == 1)
        ctx = cnt(sysm["segments"][0]["text"])
        req = cnt(reqm["segments"][0]["text"])
        asst = [m for m in msgs if m["role"] == "assistant"]
        n = len(asst)
        out = [sum(cnt(s["text"]) for s in m["segments"]) for m in asst]
        fb = [m for m in msgs if m["role"] == "user" and m["turn"] > 1]      # feedback sent before turn t (t>=2)
        tool = sum(cnt(s["text"]) for m in fb for s in m["segments"] if s["category"] == "tool")
        rep = sum(cnt(s["text"]) for m in fb for s in m["segments"] if s["category"] == "repair")
        fbt = [sum(cnt(s["text"]) for s in m["segments"]) for m in fb]       # per feedback message
        P = req + n * ctx + sum(out) + tool + rep
        P0 = P - n * ctx
        # H: full re-send of the conversation on every turn
        H = 0
        hist = 0
        for t in range(1, n + 1):
            H += ctx + req + hist + out[t - 1]
            hist += out[t - 1] + (fbt[t - 1] if t - 1 < len(fbt) else 0)
        H0 = H - n * ctx
        # secondary: model's own tokenizer (stored /tokenize counts + usage.completion_tokens), same accounting as P
        def mt(m, cat=None):
            return sum(s.get("model_tokens", 0) for s in m["segments"] if cat is None or s["category"] == cat)
        s_ctx, s_req = mt(sysm), mt(reqm)
        s_out = [(m.get("usage") or {}).get("completion_tokens", 0) for m in asst]
        s_fb = [mt(m) for m in fb]
        SP = s_req + n * s_ctx + sum(s_out) + sum(s_fb)
        SP0 = SP - n * s_ctx
        SH, hist = 0, 0
        for t in range(1, n + 1):
            SH += s_ctx + s_req + hist + s_out[t - 1]
            hist += s_out[t - 1] + (s_fb[t - 1] if t - 1 < len(s_fb) else 0)
        raw_usage = sum((m.get("usage") or {}).get("prompt_tokens", 0) + (m.get("usage") or {}).get("completion_tokens", 0) for m in asst)
        framing = sum(m["o200k_message"] - sum(s["o200k"] for s in m["segments"]) for m in msgs if m["role"] != "assistant")
        checks = [r for r in recs if r["kind"] == "check"]
        passed_turn = next((c["turn"] for c in checks if c["status"] == "pass"), None)
        row = dict(file=fn, run_id=hdr["run_id"], label=hdr["label"], arm=hdr["arm"], specimen=hdr["specimen"], stack=hdr["stack"],
                   outcome=foot["outcome"] if foot else "INCOMPLETE", turns=n, pass_turn=passed_turn,
                   request=req, context_per_turn=ctx, context_total=n * ctx, output=sum(out), tool=tool, repair=rep,
                   T_primary=P, T_primary_cached0=P0, T_history=H, T_history_cached0=H0, output_only=sum(out),
                   S_primary=SP, S_primary_cached0=SP0, S_history=SH, usage_raw_prompt_plus_completion=raw_usage,
                   framing_tokens_not_counted=framing, mismatches=len(mism))
        rows.append(row)
        print(f"== {hdr['run_id']}   [{hdr['label']}]   outcome={row['outcome']} turns={n} pass_turn={passed_turn}")
        print(f"   1 request tokens ............ {req}")
        print(f"   2 context tokens ............ {ctx} x {n} turns = {n*ctx}   (uncached)")
        print(f"   3 agent output tokens ....... {sum(out)}   per turn {out}")
        print(f"   4 tool tokens ............... {tool}")
        print(f"   5 repair-feedback tokens .... {rep}")
        print(f"   T_primary (P, uncached) ..... {P}")
        print(f"   T_primary cached-at-zero .... {P0}")
        print(f"   T_history (H, uncached) ..... {H}   cached-at-zero {H0}")
        print(f"   output-only ................. {sum(out)}")
        print(f"   secondary (model tokenizer, same accounting): S_primary {SP}  cached0 {SP0}  S_history {SH}; raw usage prompt+completion {raw_usage}")
        print(f"   framing tokens (joiner/BPE-boundary, reported, not in T): {framing}")
        if mism:
            bad += 1
            print(f"   !! {len(mism)} MISMATCH(ES):")
            for s_, why in mism[:10]:
                print(f"      seq {s_}: {why}")
        else:
            print("   recount matches every stored o200k count: OK")
    if a.csv:
        with open(a.csv, "w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=list(rows[0]))
            w.writeheader()
            w.writerows(rows)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
