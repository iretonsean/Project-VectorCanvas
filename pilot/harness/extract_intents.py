#!/usr/bin/env python3
"""Extract the 15 pilot intents (+F01, dry-run only) from G2 §5, verbatim, into ../intents/<ID>.txt.
Only edits applied (listed in EDITS, repeated in PREREG): drop doc-reference parentheticals, and for L01 append the strings the checklist needs."""
import re, sys, hashlib, pathlib
G2 = pathlib.Path("/workspace/Project-VectorCanvas/docs/G2_Benchmark_Corpus.md")
OUT = pathlib.Path(__file__).resolve().parent.parent / "intents"
PILOT = [f"L{i:02d}" for i in range(1, 11)] + [f"G{i:02d}" for i in range(1, 6)]
DRYRUN_ONLY = ["F01"]
EDITS = {
    "L01": [(' (the README card)', '')],
    "G01": [(' (A§2.9)', '')],
}
CHECKLISTS = pathlib.Path("/workspace/dev_work/dev-baseline/checklists")


def fixture_text(sid):
    """Orchestrator ruling: the blockquote under '## Fixture' in dev-baseline's checklists/<ID>.md, verbatim, appended to the G2 intent
    (same for PVC, B1, B2).  Exception L01 (see L01_OVERRIDE)."""
    p = CHECKLISTS / f"{sid}.md"
    if not p.exists():
        return None
    txt = p.read_text()
    if "## Fixture" not in txt:
        return None
    sec = txt.split("## Fixture", 1)[1].split("\n## ", 1)[0]
    q = [l[2:] if l.startswith("> ") else l[1:] for l in sec.splitlines() if l.startswith(">")]
    return " ".join(q).strip() or None


# L01's fixture block in dev-baseline's file is meta-text ("None beyond the README English request ...") that cannot be appended to a request
# (it would tell the model nothing and the G2 intent does not contain the strings).  Override: the five strings the fixture itself lists.
L01_OVERRIDE = ('The avatar shows the initials "JD", the name is "Jane Doe", the job title is "Product Designer", '
                'the bio is "Building the future of spatial computing via GPU pipelines." and the button label is "Connect".')


rows = {}
for line in G2.read_text().splitlines():
    m = re.match(r"\| (\w\d\d)(?: ?\[[A-Z]\])* \| (.*?) \| (B\d) \| (.*) \|$", line)
    if m:
        rows[m.group(1)] = m.group(2)
for sid in PILOT + DRYRUN_ONLY:
    t = rows[sid]
    for a, b in EDITS.get(sid, []):
        assert a in t, (sid, a)
        t = t.replace(a, b)
    if sid in PILOT:
        fx = L01_OVERRIDE if sid == "L01" else fixture_text(sid)
        if fx is None:
            print(f"{sid}: NO FIXTURE YET (checklist missing or no 'Fixture' line) -- intent NOT written")
            continue
        t = t + " " + fx
    p = OUT / f"{sid}.txt"
    p.write_text(t)  # no trailing newline: the file bytes are the request text
    print(sid, hashlib.sha256(t.encode()).hexdigest()[:12], t)
