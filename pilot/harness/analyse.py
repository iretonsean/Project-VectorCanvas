#!/usr/bin/env python3
"""analyse.py — PREREG §9 analysis. Pure standard library.  (B5 code, written before any real run.)

    python3 recount.py runs/*/*/transcript.jsonl --csv per_run.csv      # exits 1 on any count mismatch
    python3 analyse.py per_run.csv --out RESULT_tables.md --json result.json

Input: the per-run CSV written by recount.py (columns arm, specimen, stack, outcome, turns, pass_turn, T_* ...).
It performs: analysed-set (completed prefix per arm), valid pairs, paired percentile bootstrap (10 000, seed 20261008),
decision rule per arm (D-7), secondary-direction check, falsification sentence.  It states no interpretation beyond PREREG §9.
"""
import argparse, csv, json, random, statistics, sys, collections

SEED, B = 20261008, 10000
ORDER = ["L01", "G01", "L02", "G02", "L03", "G03", "L04", "G04", "L05", "G05", "L06", "L07", "L08", "L09", "L10"]
MIN_SET = ORDER[:10]
ARMS = ["S4", "S7"]
CATS = {"layout": lambda s: s.startswith("L"), "logic": lambda s: s.startswith("G")}
VARIANTS = [("P", "T_primary"), ("P0", "T_primary_cached0"), ("H", "T_history"), ("H0", "T_history_cached0"),
            ("output-only", "output_only"), ("secondary-P", "S_primary")]
ARM_NAME = {"S4": "Qwen3-4B Q4_K_M (thinking off)", "S7": "Qwen2.5-Coder-7B-Instruct Q4_K_M"}
LABEL = "Both arms are small local models; no hosted/frontier arm was run; the result speaks only for that class."


def med(xs):
    return statistics.median(xs)


def boot(rng, xs):
    n = len(xs)
    stats = sorted(med([xs[rng.randrange(n)] for _ in range(n)]) for _ in range(B))
    return stats[249], stats[9749]          # 250th and 9750th, 1-based


def p90(xs):
    s = sorted(xs)
    import math
    return s[max(0, math.ceil(0.9 * len(s)) - 1)]


def load(path):
    rows = list(csv.DictReader(open(path)))
    assert rows, "empty CSV"
    labs = {r["label"] for r in rows}
    if any("DRY-RUN" in l for l in labs):
        sys.exit("REFUSED: CSV contains DRY-RUN transcripts; dry runs are not data")
    return rows


def sgn(x):
    return (x > 0) - (x < 0)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("csv"); ap.add_argument("--out"); ap.add_argument("--json")
    a = ap.parse_args()
    rows = load(a.csv)
    by = collections.defaultdict(dict)       # (arm, spec) -> stack -> row
    for r in rows:
        if r["outcome"] != "INCOMPLETE":
            by[(r["arm"], r["specimen"])][r["stack"]] = r
    rng = random.Random(SEED)
    res, md = {}, []
    md.append(f"**{LABEL}**\n")
    for arm in ARMS:
        # completed prefix of the pre-registered order
        analysed = []
        for s in ORDER:
            if len(by.get((arm, s), {})) == 3:
                analysed.append(s)
            else:
                break
        evaluable = analysed[:10] == MIN_SET if len(analysed) >= 10 else False
        res[arm] = {"model": ARM_NAME[arm], "analysed": analysed, "evaluable": evaluable, "cats": {}}
        md.append(f"## Arm {arm} — {ARM_NAME[arm]}\nAnalysed set (completed prefix, n={len(analysed)}): {', '.join(analysed) or 'none'}; evaluable (≥ L01–L05+G01–G05): **{evaluable}**\n")
        for cat, pred in CATS.items():
            specs = [s for s in analysed if pred(s)]
            c = {"specimens": specs, "n": len(specs)}
            passed = lambda s, stk: by[(arm, s)][stk]["outcome"] == "pass"
            c["pass@6"] = {k: sum(passed(s, k) for s in specs) for k in ("PVC", "B1", "B2")}
            c["pass@1"] = {k: sum(by[(arm, s)][k]["pass_turn"] == "1" for s in specs) for k in ("PVC", "B1", "B2")}
            c["pass@6"]["baseline_any"] = sum(passed(s, "B1") or passed(s, "B2") for s in specs)
            c["unpaired"] = {"pvc_failed_baseline_passed": 0, "pvc_passed_both_baselines_failed": 0, "all_failed": 0}
            for s in specs:
                pv, bl = passed(s, "PVC"), passed(s, "B1") or passed(s, "B2")
                if not pv and bl: c["unpaired"]["pvc_failed_baseline_passed"] += 1
                if pv and not bl: c["unpaired"]["pvc_passed_both_baselines_failed"] += 1
                if not pv and not bl: c["unpaired"]["all_failed"] += 1
            c["variants"] = {}
            for vname, col in VARIANTS:
                d, rel, pairs = [], [], []
                for s in specs:
                    if not passed(s, "PVC"):
                        continue
                    bases = [float(by[(arm, s)][k][col]) for k in ("B1", "B2") if passed(s, k)]
                    if not bases:
                        continue
                    tb, tp = min(bases), float(by[(arm, s)]["PVC"][col])
                    d.append(tp - tb); rel.append((tp - tb) / tb if tb else float("nan")); pairs.append(s)
                v = {"valid_pairs": len(d), "pairs": pairs}
                if d:
                    v["median_diff"] = med(d); v["ci_diff"] = boot(rng, d)
                    v["median_rel"] = med(rel); v["ci_rel"] = boot(rng, rel); v["range_rel"] = (min(rel), max(rel))
                c["variants"][vname] = v
            for stk in ("PVC", "B1", "B2"):
                t = [float(by[(arm, s)][stk]["T_primary"]) for s in specs if passed(s, stk)]
                c.setdefault("stack_T_primary", {})[stk] = {"n_pass": len(t), "median": med(t) if t else None, "p90": p90(t) if t else None}
            # decision rule (PREREG §9.4)
            P, S = c["variants"]["P"], c["variants"]["secondary-P"]
            ok_tokens = P["valid_pairs"] > 0 and P["median_diff"] < 0 and P["ci_diff"][1] < 0
            n = max(len(specs), 1)
            rate_p, rate_b = c["pass@6"]["PVC"] / n, c["pass@6"]["baseline_any"] / n
            ok_pass = rate_p >= rate_b - 0.05
            inconclusive = S["valid_pairs"] == 0 or P["valid_pairs"] == 0 or sgn(S.get("median_diff", 0)) != sgn(P.get("median_diff", 0))
            c["rule"] = {"tokens_favour_pvc_ci_excludes_zero": ok_tokens, "pass_rate_within_5pp": ok_pass, "rate_pvc": rate_p, "rate_baseline_any": rate_b,
                         "secondary_direction_disagrees_or_undefined": inconclusive, "WIN": bool(ok_tokens and ok_pass and not inconclusive)}
            res[arm]["cats"][cat] = c
            md.append(f"### {arm} / {cat} (specimens {', '.join(specs) or 'none'}; n={len(specs)})\n")
            md.append(f"pass@6 PVC {c['pass@6']['PVC']}/{len(specs)}, B1 {c['pass@6']['B1']}, B2 {c['pass@6']['B2']}, baseline-any {c['pass@6']['baseline_any']}; pass@1 PVC {c['pass@1']['PVC']}, B1 {c['pass@1']['B1']}, B2 {c['pass@1']['B2']}. Unpaired failures: {c['unpaired']}.\n")
            md.append("| variant | valid pairs | median Δ tokens (PVC − baseline) | 95% CI | median relative Δ | 95% CI | min–max relative |\n|---|---|---|---|---|---|---|")
            for vname, _ in VARIANTS:
                v = c["variants"][vname]
                if v["valid_pairs"]:
                    md.append(f"| {vname} | {v['valid_pairs']} | {v['median_diff']:.1f} | [{v['ci_diff'][0]:.1f}, {v['ci_diff'][1]:.1f}] | {100*v['median_rel']:.1f}% | [{100*v['ci_rel'][0]:.1f}%, {100*v['ci_rel'][1]:.1f}%] | {100*v['range_rel'][0]:.1f}% … {100*v['range_rel'][1]:.1f}% |")
                else:
                    md.append(f"| {vname} | 0 | undefined | undefined | | | |")
            md.append(f"\nDecision rule (P, uncached): tokens favour PVC with CI excluding zero = {ok_tokens}; pass-rate within 5 pp = {ok_pass} (PVC {rate_p:.0%}, baseline-any {rate_b:.0%}); secondary-direction check disagrees/undefined = {inconclusive} → **{'PVC WINS' if c['rule']['WIN'] else 'PVC does not win'}** for this category/arm.\n")
    # falsification trigger (PREREG §9.4 item 5)
    ev = [arm for arm in ARMS if res[arm]["evaluable"]]
    lay = {arm: res[arm]["cats"]["layout"] for arm in ev}
    if not ev:
        stmt = "G2 §1 falsification trigger NOT EVALUATED: no arm completed L01–L05 + G01–G05. " + LABEL
        fired = None
    else:
        losers = [arm for arm in ev if not lay[arm]["rule"]["WIN"]]
        fired = bool(losers)
        def desc(arm):
            c = lay[arm]; P = c["variants"]["P"]; r = c["rule"]
            if not P["valid_pairs"]: return f"{arm}: no valid pair"
            t = f"{arm}: median paired difference {P['median_diff']:.1f} tokens, 95% bootstrap CI [{P['ci_diff'][0]:.1f}, {P['ci_diff'][1]:.1f}], {P['valid_pairs']} valid pairs; pass rate PVC {r['rate_pvc']:.0%} vs baseline {r['rate_baseline_any']:.0%}"
            return t + ("; inconclusive under the secondary count" if r["secondary_direction_disagrees_or_undefined"] else "")
        ns = ", ".join(f"{arm}: n={lay[arm]['n']} ({', '.join(lay[arm]['specimens'])})" for arm in ev)
        notev = [arm for arm in ARMS if arm not in ev]
        tail = (" " + ", ".join(f"{x} not evaluated (fewer than L01–L05+G01–G05 complete)" for x in notev) + ".") if notev else ""
        if fired:
            stmt = (f"G2 §1 falsification trigger FIRED for v0. In the layout-heavy category ({ns}), with the context overhead counted uncached, PVC did not win for arm(s) {', '.join(losers)}: "
                    + "; ".join(desc(x) for x in losers) + ". Per G2 §1 the token thesis is treated as falsified for v0 and the finding is escalated to the owner before any further build work." + tail + " " + LABEL)
        else:
            stmt = (f"G2 §1 falsification trigger did not fire: in the layout-heavy category ({ns}) PVC met the decision rule in every evaluable arm ("
                    + "; ".join(desc(x) for x in ev) + "), context overhead uncached." + tail + " " + LABEL)
    res["falsification"] = {"fired": fired, "statement": stmt}
    md.append("## Falsification statement\n\n" + stmt + "\n")
    if a.out: open(a.out, "w").write("\n".join(md))
    if a.json: json.dump(res, open(a.json, "w"), indent=1, default=list)
    print("\n".join(md))


if __name__ == "__main__":
    main()
