#!/usr/bin/env python3
"""STUB fidelity checker for the DRY RUN only (specimen F01, which is NOT a pilot specimen). Regex checks, no browser.
It is NOT dev-baseline's checker and proves plumbing only."""
import sys, re
h = open(sys.argv[1]).read()
fails = []
def need(cond, i, t):
    if not cond: fails.append((i, t))
need(re.search(r'type=["\']?email', h, re.I), "F01-A2", "The email field is missing or is not an email input.")
need(re.search(r'type=["\']?password', h, re.I), "F01-A3", "The password field is missing.")
need(re.search(r'(aria-label|<button)[^>]*(show|hide)|(show|hide)[^<]*</button>', h, re.I), "F01-A3b", "The show/hide control for the password is missing.")
need(re.search(r'type=["\']?checkbox', h, re.I), "F01-A4", "The accept-terms checkbox is missing.")
need(re.search(r'disabled', h, re.I), "F01-A1", "The submit button is not disabled until the form is valid.")
if fails:
    for i, t in fails: print(f"FAIL\t{i}\t{t}")
    sys.exit(1)
sys.exit(0)
