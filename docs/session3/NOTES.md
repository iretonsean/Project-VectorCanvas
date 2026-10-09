# Session 3 notes (2026-10-09)

What Session 3 learned that is not in another file. Every number is from one run on the cloud container (4 vCPU, no GPU, Claude Code 2.1.295). The raw files of these check runs were not kept, because they were pipeline checks, not planned measurements. Treat the numbers as *indicative*.

## 1. Check-run results (indicative)

| Run | Model, effort | PVC total tokens | Control total tokens | PVC ÷ control | Checklist PVC / control | Control language |
|---|---|---|---|---|---|---|
| T1 Hello + Save | Haiku, default | 59,176 | 34,609 | 1.71 | 3/3 / 3/3 | HTML |
| T3 Counter | Haiku, low | 47,286 | 27,233 | 1.74 | 5/5 / 5/5 | HTML, CSS, JavaScript |
| T3 Counter (per-request probe) | Sonnet, default | 42,680 | 16,594 | 2.57 | not judged | not recorded |

Final code size in the Haiku T3 run (o200k_base): PVC `app.pvc` 108 tokens, 13 lines; control `index.html` 497 tokens, 78 lines.

**Conclusion:** the code PVC writes is about 5 times smaller, but a whole PVC session costs 1.7–2.6 times more tokens on small apps. `docs/session3/PLAN.md` §2 breaks the Sonnet run down request by request and gives the causes: extra model requests first, the context re-read on each request second, the code size last.

## 2. Language and runtime findings

- In both check runs the PVC agent passed the validator on the first attempt. An agent can learn PVC from the frozen context alone.
- The runtime refuses arithmetic expressions. The T3 counter (`n = n + 1`) fails with `Unsupported("…: decimal/arithmetic expression")`. Before this run, the plan said the counter could be drawn. That was wrong; `pilot/owner-test/tests.json` now marks T3 as not drawn.
- The PVC prompt asked the agent to validate and compile; the control prompt asked for no check. The control ran no check at all. This makes the comparison unfair against PVC. `docs/session3/PLAN.md` step 1 fixes it.
- The local pilot models of Session 2 (Qwen3-4B, Qwen2.5-Coder-7B) were outside the repository and are gone. Their SHA-256 hashes stay in `pilot/PREREG.md`.

## 3. Tooling traps

| Trap | What happens | What to do |
|---|---|---|
| Headless `claude -p` started from inside a Claude Code session | In the cloud container the child took the parent's session ID, even with the parent's `CLAUDE*` variables removed | Always pass `--session-id <new uuid>` (done in `pilot/owner-test/run.py`) |
| Which usage field to trust | The top-level `usage` in the `-p` JSON result did not match the whole session (example: `input_tokens` 4 against 1,193 in `modelUsage`) | Sum `modelUsage` over all models (done in `run.py`) |
| `--output-format stream-json --verbose` | One assistant message with several tool calls appears as several events, each with the same `usage` | Count requests by distinct usage, not by events |
| `--restricted` | Turns off user, project and local settings files and code-running tools unless `--tools` names them; confines file tools to the working folder | Pass `--tools Bash,Read,…` with it; check that the flag exists (`claude --help`) before you use it, because older versions lack it |
| Headless Chrome as root | Refuses to start without `--no-sandbox` | Add the flag only when the user ID is 0 (done in `run.py`) |
| `tiktoken` | Not in the Python standard library | Optional; the result files say so when code size is not counted |
| Mod install | `/plugin install … --marketplace iretonsean/Project-VectorCanvas` reads the default branch only | Merge to `main` before you tell anyone to install a change |

## 4. Open items

- `hosts/pvc-web-host/build.sh` and `run_web.sh` still use `/workspace/...` paths for the Cargo target and the CPU lock.
- The `pvc-test` mod and `run.py` were run on Linux only. macOS and Windows are not tested.
- `mods/pvc-test/.claude-plugin/plugin.json` has no `author` field (a validator warning only).
- `pilot/PREREG.md` carries an amendment note for cloud-only arms. The arms are not named and the harness has no hosted-API adapter yet.
- The analysis confidence figure (~93%) was not changed in Session 3. No new measurement supports a change.
