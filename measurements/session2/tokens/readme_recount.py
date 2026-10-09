# Recount of the README example blocks at main@5fdd412 with o200k_base (Orchestrator, Session 2).
import re, subprocess, tiktoken
e = tiktoken.get_encoding('o200k_base')
t = subprocess.run(['git', 'show', '5fdd412:README.md'], capture_output=True, text=True, check=True).stdout
for lang, b in re.findall(r'```(html|text)\n(.*?)```', t, re.S):
    print(f"{lang:5s} {len(e.encode(b)):4d} tokens (block body incl. trailing newline) | {len(e.encode(b.rstrip(chr(10)))):4d} without | first line: {b.splitlines()[0][:40]!r}")
print('tiktoken', tiktoken.__version__)
