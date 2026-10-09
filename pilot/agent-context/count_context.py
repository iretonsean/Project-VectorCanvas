#!/usr/bin/env python3
"""Recount the PVC agent context exactly as the harness builds it (files joined with a blank line)."""
import hashlib, pathlib, tiktoken
d = pathlib.Path(__file__).resolve().parent
files = ["00_card.md", "01_std_reference.md", "02_dialect_en.md"]
msg = "\n\n".join((d / f).read_text() for f in files)
enc = tiktoken.get_encoding("o200k_base")
print("bytes", len(msg.encode()), "sha256", hashlib.sha256(msg.encode()).hexdigest(), "o200k_base tokens", len(enc.encode(msg)))
