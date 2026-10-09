#!/usr/bin/env python3
"""A7 golden comparison. Bound and procedure are fixed in ../TOLERANCE.md (do not edit bounds here).
usage: golden_compare.py <dir with <state>.<path>.png> <out_dir>
The reference is <state>.tier0c-vello_cpu.png; every other <state>.<path>.png is compared to it."""
import sys, os, json, glob, hashlib, datetime
import numpy as np
from PIL import Image
import flip_evaluator as flip

BOUND = {"mean": 0.005, "p99.9": 0.05, "max": 0.25}   # TOLERANCE.md §4
STATES = ["L01_idle", "F01_empty", "F01_error", "F01_zh_preedit"]
REF = "tier0c-vello_cpu"

def load(p):
    im = Image.open(p).convert("RGBA")
    a = np.asarray(im)
    return a

def magma_save(emap, path):
    flip_img = flip.utils.CHWtoHWC(flip.utils.index2color(np.round(emap * 255.0), flip.utils.get_magma_map())) if hasattr(flip, "utils") else None
    Image.fromarray((np.clip(flip_img, 0, 1) * 255).astype(np.uint8)).save(path)

def compare(ref_p, test_p, out_prefix):
    ref, tst = load(ref_p), load(test_p)
    if ref.shape != tst.shape:
        return {"error": f"size mismatch {ref.shape} vs {tst.shape}", "pass": False}
    assert (ref[..., 3] == 255).all() and (tst[..., 3] == 255).all(), "images must be opaque"
    r = ref[..., :3].astype(np.float32) / 255.0
    t = tst[..., :3].astype(np.float32) / 255.0
    emap, mean, params = flip.evaluate(r, t, "LDR", inputsRGB=True, applyMagma=False)
    emap = np.asarray(emap).reshape(r.shape[0], r.shape[1])
    mean_f = float(mean)
    p999 = float(np.percentile(emap, 99.9))
    mx = float(emap.max())
    d = np.abs(ref[..., :3].astype(np.int16) - tst[..., :3].astype(np.int16))
    res = {
        "mean": mean_f, "p99.9": p999, "max": mx,
        "pixels_not_identical": int((d.max(axis=2) > 0).sum()),
        "max_channel_abs_diff": int(d.max()),
        "pixels_total": int(emap.size),
    }
    res["pass"] = bool(mean_f <= BOUND["mean"] and p999 <= BOUND["p99.9"] and mx <= BOUND["max"])
    # diff images
    mag = np.clip(emap, 0, 1)
    # simple magma-like map via matplotlib-free ramp (black -> purple -> orange -> pale yellow)
    stops = np.array([[0, 0, 4], [80, 18, 123], [182, 54, 121], [251, 136, 97], [252, 253, 191]], dtype=np.float32)
    x = mag * (len(stops) - 1)
    i0 = np.clip(np.floor(x).astype(int), 0, len(stops) - 2)
    f = (x - i0)[..., None]
    col = stops[i0] * (1 - f) + stops[i0 + 1] * f
    Image.fromarray(col.astype(np.uint8)).save(out_prefix + ".flip.png")
    Image.fromarray(np.clip(d * 8, 0, 255).astype(np.uint8)).save(out_prefix + ".absdiff_x8.png")
    res["flip_params"] = {k: (v if isinstance(v, (int, float, str)) else str(v)) for k, v in params.items()}
    return res

def main():
    src, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    results = {}
    paths = set()
    for p in glob.glob(os.path.join(src, "*.png")):
        b = os.path.basename(p)[:-4]
        if b.endswith(".flip") or b.endswith(".absdiff_x8"):
            continue
        s, _, path = b.partition(".")
        if path and path != REF:
            paths.add(path)
    for s in STATES:
        ref_p = os.path.join(src, f"{s}.{REF}.png")
        for path in sorted(paths):
            tp = os.path.join(src, f"{s}.{path}.png")
            if os.path.exists(ref_p) and os.path.exists(tp):
                results[f"{s}|{path}"] = compare(ref_p, tp, os.path.join(out, f"{s}.{path}"))
    meta = {"date": datetime.datetime.utcnow().isoformat() + "Z", "bound": BOUND, "flip_evaluator": "1.7",
            "loadavg": open("/proc/loadavg").read().strip(),
            "tolerance_sha256": hashlib.sha256(open(os.path.join(os.path.dirname(__file__), "..", "TOLERANCE.md"), "rb").read()).hexdigest()}
    json.dump({"meta": meta, "results": results}, open(os.path.join(out, "results.json"), "w"), indent=1)
    lines = ["| state | path | mean | p99.9 | max | px not identical | max ch diff | verdict |", "|---|---|---|---|---|---|---|---|"]
    for k, r in results.items():
        s, path = k.split("|")
        if "error" in r:
            lines.append(f"| {s} | {path} | - | - | - | - | - | FAIL ({r['error']}) |")
        else:
            lines.append(f"| {s} | {path} | {r['mean']:.6f} | {r['p99.9']:.5f} | {r['max']:.4f} | {r['pixels_not_identical']} | {r['max_channel_abs_diff']} | {'PASS' if r['pass'] else 'FAIL'} |")
    open(os.path.join(out, "results.md"), "w").write("\n".join(lines) + "\n")
    print("\n".join(lines))

if __name__ == "__main__":
    main()
