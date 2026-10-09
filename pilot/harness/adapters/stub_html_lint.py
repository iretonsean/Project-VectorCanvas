#!/usr/bin/env python3
"""STUB tool for the DRY RUN only. Not html-validate. Reports unclosed tags and inputs without label/aria-label."""
import sys, re
from html.parser import HTMLParser
VOID = {"meta","link","br","hr","img","input","source","area","base","col","embed","param","track","wbr"}
class P(HTMLParser):
    def __init__(s):
        super().__init__(); s.stack=[]; s.out=[]; s.labels=set(); s.inputs=[]
    def handle_starttag(s, t, a):
        a=dict(a)
        if t=="label" and a.get("for"): s.labels.add(a["for"])
        if t=="input": s.inputs.append(a)
        if t not in VOID: s.stack.append((t, s.getpos()[0]))
    def handle_endtag(s, t):
        if t in VOID: return
        if s.stack and s.stack[-1][0]==t: s.stack.pop()
        else: s.out.append(f"line {s.getpos()[0]}: unexpected </{t}>")
p=P(); p.feed(open(sys.argv[1]).read())
for t,l in p.stack: p.out.append(f"line {l}: <{t}> is never closed")
for a in p.inputs:
    if a.get("type")!="hidden" and not (a.get("aria-label") or a.get("id") in p.labels or a.get("aria-labelledby")):
        p.out.append(f"input type={a.get('type','text')}: no accessible label")
print("\n".join(p.out))
