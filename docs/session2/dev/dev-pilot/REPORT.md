# dev-pilot REPORT — B1 (PREREG), B3 (harness + recount), dry run  [prep wave; no real pilot run]

Directory: `/workspace/dev_work/dev-pilot/`. PREREG hash at time of writing: see final message / `sha256sum PREREG.md` (updated after Supervisor confirmations) (re-hash if the file is edited).

## 1. Summary
- `PREREG.md` is drafted. It covers specimens/order, arms, stacks, exact T_total accounting (P primary, H variant, cached-at-zero, output-only, secondary), repair templates verbatim, tools, fidelity, CI method, G2 §1 decision rule and falsification trigger quoted verbatim with the exact fired/not-fired sentences, and a deviation list. Fixtures from dev-baseline are included in the request text (Orchestrator ruling).
- It still has `⟦PENDING⟧` items owned by others (§12 of PREREG): dev-ir `FROZEN.md` (blocks the run by design), the B2 library, the tool/checker command lines, and a final harness hash at freeze.
- Harness (`harness/pilot.py`), `recount.py`, `analyse.py` (B5 analysis code, written and tested on synthetic data before any run) and tests are done.
- One real-model dry run (Qwen3-4B, B1, stub tools, dry-run specimen D00) proves the plumbing. It is labelled DRY-RUN and kept out of any results directory.
- Supervisor ruling on history re-send: Orchestrator relayed "agreed, primary = NO (P), H reported". The Supervisor later confirmed it, the D-7 reading, max_tokens 3072 and the L01 sentence; PREREG updated.

## 2. Done criteria
| Criterion (brief) | Status | Evidence |
|---|---|---|
| B1 PREREG drafted and announced | **met (draft)**; announced with the final message to the Supervisor | `PREREG.md` |
| PREREG blocks on frozen PVC context | met | `harness/config.json` has `PLACEHOLDER`; the harness refuses real runs (tested) |
| Harness: start/stop llama-server per arm, build messages, extract, tools, fidelity, feedback, stop at pass/turn 6 | met (real-model run with stub tools; full logic also tested with a fake server) | `dryrun/`, `harness/tests/test_harness.py` |
| Saves every turn verbatim to JSONL with counts, usage, timings, timestamps, model sha256, harness hash | met | `dryrun/S4/D00__B1/transcript.jsonl` |
| `recount.py <transcript>` recomputes counts and prints per-item totals | met | `dryrun/recount_DRYRUN.txt`; tamper test in `test_harness.py` |
| Model runs under the CPU lock | met, **updated per Orchestrator amendment**: real runs hold `flock -x` for the whole arm (`--lock exclusive-run`, mandatory for non-dry runs); dry runs use shared per-call. Tested: lock excludes other holders, no self-deadlock, real run refuses shared mode | `harness/pilot.py` `SharedLock`, `tests/test_harness.py` |
| Dry run, labelled | met | `dryrun/`; header label `DRY-RUN — not data` |
| B5 (run, analysis) | not started (correctly, waits for "go") | — |

## 3. Checked vs inferred
**Checked (I ran it; file given):**
- `python3 harness/tests/test_harness.py` → `ALL HARNESS TESTS PASSED`. It covers: interleaved order; fence extraction rules; tool truncation; a 3-turn loop on a fake llama-server (NO-ARTIFACT, then fail, then pass) with stub PVC tools; the exact repair text; the independently recomputed T_primary, P0 and H; tamper detection by recount.py (exit 1); and refusal of a real run without `--prereg-sha256`. This is a fake server, **no LLM**.
- `python3 harness/tests/test_analyse.py` → `ANALYSE TESTS PASSED`: synthetic CSVs only. Checks the completed-prefix rule, valid pairs, the bootstrap, the decision rule, the fired / not-fired / not-evaluated statements, and the refusal of DRY-RUN data.
- Real dry run: `dryrun/S4/D00__B1/transcript.jsonl`: Qwen3-4B-Q4_K_M (sha256 verified by the harness), llama-server b11514, 2 turns, turn 1 failed the stub check, turn 2 passed. `dryrun/recount_DRYRUN.txt`: recount matches every stored o200k count; framing tokens 0.
  - T_primary 518 = request 15 + context 120×2 + output 211 + tool 14 + repair 38; P0 278; H 638; secondary (model tokenizer) 540. *These are plumbing numbers, not data.*
  - Server `usage`: `completion_tokens` 114/115 against o200k output counts 105/106. `prompt_tokens` includes cached tokens (turn 2: 330 total, `cached_tokens` 149). `timings.cache_n` is present.
  - Thinking-off worked: `reasoning_content` empty on both turns.
- Intent files: 15 files extracted from G2 §5 + dev-baseline fixtures; sha256 and o200k counts are in PREREG §4.4 (from `harness/extract_intents.py`).
- `harness/contexts/B1.md` = 120 o200k tokens (`pilot.py context-info`).
- Observed with `ps`: S4 llama-server RSS ≈ 9.7 GB at `-c 32768`. Observed from the server log: generation fell to 0.05–2 tok/s while others compiled (load average 15–17), which killed the first dry-run attempt (`dryrun_aborted_F01_starved/`).

**Inferred / not checked:**
- Compute plan (540 turns upper bound; throughput from FINDINGS F18) is an *estimate*.
- Claim that P is the more conservative accounting for the PVC claim (H adds the larger baseline history) is reasoning, not run.
- Bit-exact repeatability of generations across prompt-cache states was **not tested** (PREREG does not claim it).
- `max_tokens 3072` is a judgement; its effect on baselines vs PVC is unknown until the real transcripts exist.
- Nothing about the real tools: `pvc-validate`, `check-pvc`, `check-baseline`, html-validate, tsc, axe wrappers did not exist yet, so the harness was tested against stubs and the adapter contract only (`harness/adapters/ADAPTERS.md`).

## 4. Measurements
No pilot measurements. Dry-run numbers above are `DRY-RUN — not data`, `host=container-xeon4`, llama.cpp CPU, 4 threads; they are not timing results. Dry-run wall time (about 55 s per ~110-token turn, machine under load average ≈ 10) is not a measurement. No CPU-lock-exclusive measurement was made.

## 5. Deviations and labelled gaps
- Full list in PREREG §13 (V7–V10 plus D-F, D-B2, D-T, D-S, D-C, D-U).
- **L01 fixture:** dev-baseline's L01 fixture block is meta-text and not appendable; I use the five strings it lists in one plain sentence (Orchestrator: agreed). Alternatives for the PM: use the README request, but that is phrased in PVC vocabulary and breaks G2 §4.1.
- **Lock:** real pilot runs hold the CPU lock exclusively for the whole arm (Orchestrator amendment); this blocks every other lock user for the duration of an arm. The dry run in `dryrun/` ran before the amendment in shared per-call mode (`cpu_lock_mode` is not in its header).
- **Dry run used a stub checker and dry-run specimen D00** (not F01), because the first attempt on F01 starved under CPU load. D00's intent was hand-written (not from G2) and lives in `intents/D00.txt`.
- **Secondary count** uses `/tokenize` plus `usage.completion_tokens`; `usage` alone cannot be split by G2 category. This extends PLAN V10.
- The dry-run header's harness hash (`1cae12d5…`) is for the pre-amendment harness; the current harness hash is `b0796718…` and it changes with any later edit. The final hash is taken at freeze.

## 6. Dependencies
No Cargo additions. Tooling: system python 3.11.16, `tiktoken` 0.14.0 (already installed; no new installs by me, no venv needed: stdlib only). llama.cpp b11514 and the two GGUF models are the Orchestrator's installs (hashes in PREREG §3). No apt/npm/pip installs; no `target/` dir of mine exists.

## 7. Inconsistencies flagged
1. G2 §5 L01's intent does not contain the strings that its own checklist item "exact strings present" needs (the intent only says "the README card"). dev-baseline's fixture for L01 is not appendable as written (see above).
2. G2 §2 is silent on re-sent history (the ruling is relayed: P primary). G2 §2 says "provider-reported billed usage" but no vendor exists here (V10).
3. G2 §4.3 "HTML validation and TypeScript compile errors for B2" is ambiguous (html-validate for B2 or not). I chose tsc + axe for B2 and html-validate + axe for B1 (PREREG §4.3).
4. G2 §1 "across all agent arms" vs D-7 "each arm individually": I operationalised it as "wins only if the rule is satisfied in every evaluable arm", so the trigger fires if either arm fails. If the Supervisor intended "fires only if both fail", PREREG §9.4 item 1 and 5 must change *before* the run.
5. G2 §4.2 says arm S runs on a Tier-1-class laptop; here it is the container Xeon (D-S).
6. Resource: S4 server at `-c 32768` needs ≈ 9.7 GB RSS of a 15 GB container shared with builds.

## 8. Open issues (blocking the first real run)
- dev-ir `FROZEN.md` (file list/order, hashes, system-message o200k count, fence tag) and the working `pvc-validate` / `check-pvc` CLIs matching `ADAPTERS.md` (exit 0 / 1 / 2, `FAIL<TAB>id<TAB>text` lines).
- dev-baseline: B2 component library name and version; final `contexts/B1.md` / `B2.md` reconciliation with the checker's build; wrapper command lines for html-validate / tsc / axe-core and `check-baseline` that conform to `ADAPTERS.md`. Fill `harness/config.json`.
- Supervisor confirmation of PREREG §0 and of the D-7 reading in §7.4 above.
- Then: freeze (final harness hash into PREREG), commit by the Orchestrator, and `pilot.py run … --prereg-sha256 <hash>` after "go". A quiet CPU window is needed.
- Not done: repeatability test of cached vs uncached generation; `T_session`; languages (out of scope per D-3).

## Files
`PREREG.md`; `harness/{pilot.py,recount.py,analyse.py,extract_intents.py,config.json,config.dryrun.json,contexts/,adapters/,tests/}`; `intents/*.txt`; `dryrun/`, `dryrun_aborted_F01_starved/` (both labelled not data).
