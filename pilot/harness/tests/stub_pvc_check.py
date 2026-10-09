#!/usr/bin/env python3
import sys
t = open(sys.argv[1]).read()
if "button" in t: sys.exit(0)
print("FAIL\tL00-A1\tThe button is missing.\nFAIL\tU1\tEvery interactive element needs an accessible name."); sys.exit(1)
