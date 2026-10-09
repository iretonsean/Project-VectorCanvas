#!/usr/bin/env python3
"""STUB fidelity checker for the DRY RUN only (dry-run specimen D00, not a pilot specimen). Regex checks, no browser."""
import sys, re
h = open(sys.argv[1]).read()
fails = []
if not re.search(r'<h1[^>]*>\s*Hello\s*</h1>', h, re.I): fails.append(("D00-1", "The page needs a level-1 heading with the text Hello."))
if not re.search(r'<button[^>]*>\s*Save\s*</button>', h, re.I): fails.append(("D00-2", "The page needs a button labelled Save."))
if not re.search(r'<title[^>]*>[^<]*Greeting', h, re.I): fails.append(("D00-3", "The page title (title element) must contain the word Greeting."))
for i, t in fails: print(f"FAIL\t{i}\t{t}")
sys.exit(1 if fails else 0)
