# PREREG — G2 pilot pre-registration (Session 2, Deliverable B, thread B1)

**Status: DRAFT. Not yet committed. No real run has happened.** Every item written as `⟦PENDING …⟧` must be filled by a named owner and this file re-hashed *before* the first real run. The harness refuses a real run unless `--prereg-sha256` equals the sha256 of this file (`harness/pilot.py`).
**Author:** dev-pilot. **Files:** `/workspace/dev_work/dev-pilot/` (harness in `harness/`, intents in `intents/`).
**What this is:** the written-down test, fixed before any result exists, so the outcome cannot be rationalised afterwards (G2 §1).
**What this is not:** evidence. Nothing in this file is a measured result, apart from the one dry-run line in §14, which is labelled *not data*.

**Mandatory label (repeat it wherever a result from this pilot is quoted):**
> Both arms are small local models; no hosted/frontier arm was run; the result speaks only for that class.

**Host label for any wall-clock figure:** `host=container-xeon4` (4 vCPU Intel Xeon @ 2.10 GHz, 15 GB, no GPU), llama.cpp CPU build. Wall-clock time is **not** a G2 metric and is never used in the decision rule.

---

## 0. Question for the Supervisor (needs a ruling before the first real run)
**Does re-sent conversation history (prior agent outputs, prior tool and repair messages) count again on later turns in `T_total`, or only the items G2 §2 lists?**
*Ruling status: CONFIRMED by the Supervisor (relayed by the Orchestrator): re-sent history does NOT count again; primary P = G2 §2 literal; H reported only. Also confirmed: D-7 reading (§9.4 items 1 and 5: PVC wins a category only if the rule holds in every evaluable arm; the trigger fires if either arm fails it), `max_tokens 3072` as a labelled judgement, and the L01 five-strings sentence.* *Recommended default (adopted):* **No** — primary `T_total` = definition **P** of §5 (G2 §2 literally: context overhead re-counted every turn; everything else once). The full-resend variant **H** is reported alongside but is not decisive. Reason: P is what G2 §2 enumerates; H turns the metric into a billing proxy that grows with the number of turns and with agent verbosity, and P is the conservative choice for the PVC claim (P charges PVC's large context every turn while not crediting its shorter outputs through history re-send).

---

## 1. Question served

Q1 of analysis §8.1: does PVC's per-session context overhead eat the token savings? Metric and rule are G2 §1–§2. This pilot is G2 restricted to: 15 specimens, 2 small local arms, create-turn only (§5 "T_total"). The five-step session variant `T_session` (G2 §4.5) and the multi-language arm (G2 §4.6) are **not run** in this pilot.

## 2. Specimens and run order

**Specimens (D-3):** L01–L10 (category 1, layout-heavy; carries the falsification trigger) and G01–G05 (the lowest five IDs of category 4, logic-heavy). `F01` is **not** a pilot specimen; it is used only for the harness dry run.

**Interleaved order, deterministic rule:** alternate L and G specimens, starting with L, in ascending ID order; when the G specimens run out, the remaining L specimens follow in ascending order. Result (also printed by `python3 harness/pilot.py order`):

`L01, G01, L02, G02, L03, G03, L04, G04, L05, G05, L06, L07, L08, L09, L10`

**Run order inside the pilot:** arm-major (all of S4 first, then all of S7: one model load per arm). Within an arm: specimens in the order above; within a specimen, stacks in the order `PVC, B1, B2`. Every (arm, specimen, stack) is one *run* of at most 6 turns. Runs are independent: no state, memory, or example carries from one run to another (the llama-server prompt cache may reuse a common prefix; that changes speed only, not the counted tokens).

**If compute binds (D-3):** each arm's analysed set is the **completed prefix** of the order above for that arm, where a specimen is complete only when all three stacks (PVC, B1, B2) have finished for it. The prefix is cut by position in the order, never by result. The analysed set must not be smaller than **L01–L05 + G01–G05** (the first 10 positions); if an arm does not reach that, it is reported as *partially run* and the sentence in §9.4 for "not evaluable" is used. The analysed set and n are stated per arm beside every number. S7 is started only after S4 is complete or has been stopped by the Orchestrator.

**No selection after the fact.** No specimen, run, or turn is dropped, re-run or replaced because of what the model produced. The only permitted re-run is after an infrastructure failure (server crash, out-of-memory, harness exception, power loss) — the failed attempt's partial transcript is kept in a `_failed/` directory and listed in RESULT.md. An incomplete run restarts from turn 1 (the harness deletes it); there is no mid-run resume.

## 3. Arms (D-1)

| Arm | Model file | sha256 | Notes |
|---|---|---|---|
| **S4** | `/workspace/tools/models/Qwen3-4B-Q4_K_M.gguf` (Qwen3-4B, Q4_K_M) | `7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5` | thinking **off**: request field `chat_template_kwargs: {"enable_thinking": false}` |
| **S7** | `/workspace/tools/models/qwen2.5-coder-7b-instruct-q4_k_m.gguf` (Qwen2.5-Coder-7B-Instruct, Q4_K_M) | `509287f78cb4d4cf6b3843734733b914b2c158e43e22a7f4bf5e963800894d3c` | no thinking mode; no template kwargs sent |

The harness hashes each model file at arm start and aborts on mismatch.

**Runtime:** llama.cpp **b11514** (`/workspace/tools/llama-b11514/llama-server`, CPU). Server arguments, identical for both arms: `-c 32768 -t 4 --parallel 1 --jinja --seed 1234 -ngl 0` (context size 32768, 4 threads, one slot, chat template via jinja). Endpoint `/v1/chat/completions`, non-streaming.

**Sampling, identical for both arms and all stacks:** `temperature 0, top_k 1, top_p 1.0, min_p 0.0, repeat_penalty 1.0, seed 1234`, `cache_prompt true`. **Max output tokens per turn: `max_tokens = 3072`** (chosen so that a one-file React page is unlikely to be cut; a looping generation costs at most 3072 tokens per turn; with 6 turns, the 32768 context holds ~22k tokens of replies plus context, so a PVC context above ~10k tokens would need the context size raised — the harness then records `fail_server_rejected`). A reply cut by this cap (`finish_reason: length`) whose fenced block is not closed has no artifact, and is handled as `NO-ARTIFACT` (§7). The count of length-truncated turns per stack is reported, because a cap that bites harder on longer-output stacks favours the terser stack; this is a stated limit, not corrected.

**Determinism note:** at temperature 0 with a fixed seed the *sampling* is deterministic. The CI in §10 therefore reflects **variability across specimens only**, not sampling variability. Bit-exact reproduction of a generation is not claimed (CPU thread scheduling and prompt-cache state can change floating-point reduction order); every counted number is recomputable from the saved transcript, which is what the Supervisor checks.

**Required wording wherever results are shown:** the mandatory label above, plus the arm's model name, quantisation and llama.cpp build.

## 4. Stacks, contexts and tools

### 4.1 What goes to the model
Per turn the model receives the full chat so far (stateless API; the harness re-sends it):

1. **system message** = the stack's *context* (item 2 of §5). One segment, category `context`.
2. **first user message** = the *request*: the G2 §5 intent text for the specimen **plus** the specimen's fixture sentence(s) (§4.4). One segment, category `request`. Identical text for PVC, B1, B2.
3. then, for each further turn: the previous **assistant reply** (verbatim) and a **feedback user message** made of up to two segments: `tool` (tool output) and `repair` (standard repair text), joined by a blank line.

### 4.2 Stacks
- **PVC:** agent context from dev-ir `agent-context/FROZEN.md` (v2, committed 2224d28; v1 superseded and not used): system message = `00_card.md`, `01_std_reference.md`, `02_dialect_en.md` (absolute paths under `/workspace/dev_work/dev-ir/agent-context/`) joined by `"\n\n"`; **8354 bytes, 2511 `o200k_base` tokens (the PVC fixed overhead, charged on every turn), sha256 `d30503040f448a0f9921a93c404c32469c901da7430942838bde80ea685320d9`**. File hashes: card `79d42fae…c927`, std reference `d5a15160…7a87`, en dialect `a869c398…6315` (full in FROZEN.md). The harness re-computes the sent text's sha256 and o200k count from `harness/config.json` at start and **refuses to run if either differs from FROZEN** (checked: `pilot.py context-info --stack PVC` prints exactly these numbers). Output: one fenced block tagged `pvc` (terse PVC-IR; the harness does not sign it). Tool: `/workspace/cargo-target/debug/pvc-validate <file>` (text format, exit 0 = no errors / 1 = errors / 2 = unreadable); its stdout, including the final `OK …`/`FAIL …` line, is the tool output fed back. ⟦PENDING at freeze: sha256 of the `pvc-validate` binary actually used; `check-pvc` command line⟧
- **B1:** HTML + Tailwind CSS + vanilla JavaScript in **one file**. Context = `harness/contexts/B1.md` (output format and environment rules only; sent verbatim; o200k count in §12). Output: one fenced block tagged `html`.
- **B2:** React + TypeScript + Tailwind + **one fixed component library** (not agent-chosen; deviation from G2 §3.3, §13). Context = `harness/contexts/B2.md` with `{{LIBRARY}}` replaced by the name dev-baseline fixes. Output: one file `App.tsx` in one fenced block tagged `tsx` (`typescript`/`ts` accepted). ⟦PENDING dev-baseline: library name+version; the B1/B2 context texts are to be reconciled with how dev-baseline's checker builds the artifact, and the final text and hash entered in §12.⟧
- **Both baselines run on every specimen** (G2 §3.3). **Reported baseline `T_total` for a specimen = the lower passing `T_total` of B1 and B2** (per count variant, §5).

Context fairness rule: *everything in the system message is context overhead (item 2), for every stack, including the baselines' output-format instructions.* The intent and fixture are request (item 1). Nothing else is added. The baseline contexts say nothing about accessibility, while the PVC context necessarily teaches PVC's accessibility rules; this asymmetry is G2 §3.1 by design (the baseline pays for accessibility through repair turns) and is listed in §13.

### 4.3 Tools (G2 §4.3) — harness-mediated
After **every agent turn that contains an artifact**, the harness itself runs the stack's tools on the extracted file. Tool output is **fed back to the agent in the next user message only if another turn follows**, and counted as **tool tokens** (item 4) only when so sent. The agent does not call tools; tool output on a passing turn, or on turn 6, is never sent and not counted (a labelled simplification: it slightly understates tool use equally for all stacks).

| Stack | Tools run by the harness |
|---|---|
| PVC | `pvc-validate` (dev-ir) |
| B1 | `html-validate`, `axe-core` (dev-baseline wrappers) |
| B2 | `tsc` type-check, `axe-core` (dev-baseline wrappers). `html-validate` is not run for B2: the rendered DOM is React-generated; this is my reading of G2 §4.3 and is flagged in §13 |

Tool output is shown as `<tool name> output:` followed by its stdout, at most **40 lines, each cut at 300 characters**, then `... (N more lines omitted)`; empty stdout is shown as `(no findings)`. Multiple tools are joined by a blank line. No screenshots (G2 §4.3). ⟦PENDING dev-baseline/dev-ir: the exact command lines and compact output formats (`harness/adapters/ADAPTERS.md` is the contract I need from them); entered in §12.⟧

### 4.4 Request text: G2 intent + fixture (Orchestrator ruling; deviation D-F)
The request is the G2 §5 "Intent" cell **verbatim**, with only two edits (each repeats in §13): the doc-reference parentheticals `(the README card)` (L01) and `(A§2.9)` (G01) are removed because they are meaningless to the model. To it is appended, after one space, the **"Fixture" sentence(s) taken verbatim from dev-baseline's `checklists/<ID>.md`** (`/workspace/dev_work/dev-baseline/checklists/`), identically for PVC, B1 and B2, because several checklist items cannot be decided without facts the G2 intent omits (e.g. L01's exact strings, G04's valid promo code). Fixture text is counted as **request tokens**. This is a **labelled deviation from G2 §4.1** ("same intent text"); the pairing across stacks is unaffected because all three stacks receive the same request. Intent files: `intents/<ID>.txt` (generated by `harness/extract_intents.py`; file bytes = request text, no trailing newline).
Source of the fixtures: `/workspace/dev_work/dev-baseline/checklists/<ID>.md`, the blockquote under `## Fixture` (`checklists/INDEX.md` sha256 `074a7e39986ea25ccb41320b5ffd2dac3bed2ad584e9d29a984599c6b1b036fd`; the per-file hashes are in that INDEX).
**Exception, L01 (flagged to the Supervisor):** dev-baseline's L01 fixture is not an appendable sentence ("None beyond the README English request …"); the G2 L01 intent has no strings, and the README request itself is phrased in PVC's vocabulary (`Canvas { … }`, "'connect_user' state function"), against G2 §4.1. For L01 only, I append the five strings the fixture lists, in plain words: `The avatar shows the initials "JD", the name is "Jane Doe", the job title is "Product Designer", the bio is "Building the future of spatial computing via GPU pipelines." and the button label is "Connect".` Applied identically to PVC, B1, B2. (Accepted by the Supervisor.)

| ID | request file `intents/<ID>.txt` sha256 | o200k_base tokens |
|---|---|---|
| L01 | `85316c2ee0f9af87bf53074047c5986b5193dbb1498d509b18d162e7c7a0401d` | 68 |
| L02 | `dacc048508a7654c4ed072b594447a41d463600a04fc3dd7e4af75d6714da3d3` | 72 |
| L03 | `0e9f9c9b4bc0c93890fc1d42a43f76fbbeb98046a7ffd483f3b376a0392b4d0b` | 72 |
| L04 | `021b09c4d49a82e8954f46d4f4aaeaef18509a9fad71580209ecba6c1a440ed6` | 73 |
| L05 | `e74449f49b6e180c02626e965b06c84d9737f77f6d9d6840d8197e157c75f4f2` | 76 |
| L06 | `44b40cd079d80c588b9188a56c23cecaf01a104872515d8094c50e2b40a985f1` | 75 |
| L07 | `3f97cdc81043be797e91f22eaf02381965c58416683b745136cba35f3618ad27` | 103 |
| L08 | `9d81ffa014a23e24de1177f89af0c6c419edf8be65acfccfdf677e183116284d` | 85 |
| L09 | `461fcd0cbf6d469d51529a2893742d982de8347e2f279a0d667b812f64a96223` | 99 |
| L10 | `ff5d4d8edcd4435783ec55a11d09fd2a92e49dc162c9aaec1b05400b6ca1b444` | 82 |
| G01 | `b5e8bcf22893ed0a7dba989c2fb4c72c3ed1fbd23e8d1e69171b5daa46803cec` | 150 |
| G02 | `955cca71268ab8bcf9a4da109b5cefedccaec0729a2a812b2fa6e33ffeee5184` | 100 |
| G03 | `19b669e466494639a8bccb8826d5b9bd8a25eda76643656a895fe9086f19f6f7` | 139 |
| G04 | `973658af79f82ca37cdaeba1fb3ad43f939171576596fad6f282d9e7eea43da6` | 222 |
| G05 | `d6666d41af1aef002597c3962fab23e1a6ad94099f12332711bd9a74b33d5efe` | 163 |

(Request token counts are `tiktoken` o200k counts of the file bytes; they are request tokens, item 1, counted once per run.)

Intent texts were drafted by the project's PM and the fixtures by a Developer, not by outside authors (G2 §4.1 and §7.3 are **unmet**; deviation V9).

## 5. Exact accounting of `T_total` (G2 §2)

Counting tokenizer: **`o200k_base`**, `tiktoken` 0.14.0, `encode(text, disallowed_special=())` on the **exact message text** (UTF-8, as saved in the transcript). A literal string in the text that looks like a special token (e.g. `<|im_start|>`) is counted as ordinary text. **Chat-template special tokens that llama-server adds around messages (role markers, `<think></think>` stubs, BOS/EOS) are not part of any message text and are excluded from every o200k count**; they appear only in the raw server `usage` (secondary) and are reported as "template overhead" per turn.

For one run with `n` agent turns (n ≤ 6), with `ctx` = o200k(system message), `req` = o200k(request), `out_t` = o200k(assistant reply `t`) (plus o200k of any `reasoning_content` the server returns; with thinking off this is expected to be empty and its presence is reported), `fb_t` = o200k(feedback message sent after turn `t`, t < n) split into `tool_t` and `repair_t`:

| G2 §2 item | Counted as | When counted |
|---|---|---|
| 1 request | `req` | once (turn 1): the human sends it once |
| 2 context overhead | `ctx` | **on every turn** (n times), uncached |
| 3 agent output | `Σ out_t` | each turn, as returned (code + commentary + any reasoning field) |
| 4 tool tokens | `Σ tool_t` | in the feedback message in which it is sent |
| 5 repair-feedback tokens | `Σ repair_t` | same |

**Primary definition P (proposed; needs the Supervisor's ruling, §0):**
`T_total(P) = req + n·ctx + Σ out_t + Σ tool_t + Σ repair_t`
i.e. **only the items G2 §2 lists are counted, each when it is first produced, except context overhead, which G2 says is counted on every turn it is sent. Re-sent conversation history (earlier assistant replies, tool and repair messages, and the request itself) is NOT counted again.**

**Variant H (reported, never used for the decision rule unless the Supervisor rules otherwise):** every turn is billed as a fresh uncached request, i.e. the full conversation so far is counted again.
`T_total(H) = Σ_{t=1..n} [ ctx + req + Σ_{j<t}(out_j + tool_j + repair_j) ] + Σ_t out_t`.

**Cached-at-zero variants (G2 §2):** `P0 = P − n·ctx`, `H0 = H − n·ctx` (the context overhead counted as zero on every turn, the best case if a provider caches it). The truth for a real provider lies between P (uncached) and P0. In H0 only the context is zeroed, history is still counted.

**Output-only:** `Σ out_t`.

A run that has not passed after **6 agent turns** is a **failure**: its `T_total` is recorded as tokens spent (same formulas with n=6; no feedback follows turn 6), it is excluded from the medians, it is counted in the pass rate (G2 §2). The same applies to `fail_server_rejected` (the server returned HTTP 400, e.g. prompt + max_tokens > 32768) — recorded as a failure with the tokens spent.

**Framing:** the system message and the request are single segments. The feedback message is two segments joined by `"\n\n"`; that joiner and BPE boundary effects between segments are not charged to any item; `recount.py` prints the difference between the whole-message count and the sum of segment counts ("framing tokens") so it can be seen (expected ≈ 1 token per feedback message).

**Per-category range (README format, G2 §1):** the median **relative** paired difference `(T_PVC − T_base)/T_base` with its 95% CI and min–max across specimens, **per arm**, never "up to X%".

### 5.1 Secondary count (PLAN V10; replaces vendor billing)
There is no vendor billing for local models. The secondary count is the **model's own tokenizer** through llama-server, with the **same accounting as P** (and H):
- agent output = `usage.completion_tokens` of each response;
- every input item (context, request, tool, repair) = the number of tokens from `POST /tokenize` (`add_special=false`, `parse_special=false`) on that segment's exact text. (`usage` alone cannot be split by G2 category: it reports one prompt total per turn.)
- Additionally the raw `usage.prompt_tokens + usage.completion_tokens`, summed over turns, is saved (it contains the template tokens and, being a full-prompt count each turn, corresponds to the H accounting).
**Direction rule (G2 §2):** for each (arm, category), if the sign of the median paired difference under the secondary count differs from its sign under the primary count (zero counts as its own sign), that category/arm is reported **inconclusive**, whatever the primary CI says. This is reported next to every primary result.

## 6. Turn loop and failure (G2 §2: turn cap 6)

```
messages = [system(context), user(request)]
for turn in 1..6:
    reply = model(messages)                      # temp 0, max_tokens as §3
    artifact = last complete fenced block whose tag is accepted for the stack (§4.2),
               else last complete untagged block, else none        # deterministic
    if none:  failed = [NO-ARTIFACT]
    else:     run the stack's tools on the artifact (outputs kept for feedback);
              run the fidelity checker (§8); exit 0 → PASS, stop
    if turn == 6: FAIL, stop
    send feedback = [tool output segment (if any tool ran)] + [repair segment]
```
`pass@1` = pass on turn 1; `pass@6` = pass within 6 turns. A reply with no extractable artifact is a failed turn (item `NO-ARTIFACT`; no tools run, no tool segment).

## 7. Repair feedback (G2 §4.4) — standard text, verbatim, identical across stacks
After a failed check the agent receives **only** the standardised text of the failing checklist items, no hints, no improvisation. The checker supplies `<ITEM-ID>` and `<standard text>` (the same strings for the same checklist item in every stack — dev-baseline's `checklists/`). Template, exactly (newline-separated, no trailing newline):

```
These checklist items failed:
- <ITEM-ID>: <standard text>
- <ITEM-ID>: <standard text>
Fix them and reply with the complete corrected result in the same format.
```
Two pseudo-items, identical for every stack:
```
- BUILD: The result could not be built or loaded, so no checklist item could be checked.
- NO-ARTIFACT: No complete code block in the required format was found in your reply.
```
`BUILD` is used when the checker exits 2 (PVC: validator error-severity diagnostics, the runtime would refuse to load it; B1: the page cannot load; B2: the esbuild bundle or page load fails). A TypeScript type error alone is **not** `BUILD` for B2 (the code still runs); it reaches the agent only through the `tsc` tool output. These definitions are my proposal and must be matched by the checkers (§12).

## 8. Fidelity (G2 §3.1–§3.2, D-5)
- Pass = **every A item of the specimen's checklist and the universal items U1–U5** (WCAG 2.2 AA thresholds, from general knowledge as G2 says) pass in the stack's checker: dev-baseline's `check-baseline` (Chromium 141, offline) for B1/B2, `check-pvc` (dev-ir validator + dev-render headless runtime) for PVC. ⟦PENDING: dev-baseline's report on `checker_validation.md` (baseline checkers) and the PVC checker validation status⟧
- **H (human) items are "not judged" (D-5)** (L02, L03, L05 each contain one). Pass is decided **on A items only**, and every table says so.
- **An LLM is never the judge.** No model call occurs in any checker.
- The checklist specs are fixed before any run: dev-baseline `checklists/INDEX.md` sha256 `074a7e39986ea25ccb41320b5ffd2dac3bed2ad584e9d29a984599c6b1b036fd` (lists each checklist file and each executable spec module by hash; if dev-baseline regenerates INDEX before the first run, this hash must be updated).
- Known weakness, stated in advance: checkers were written by dev-baseline from G2 alone; the PVC checker is a different implementation from the baseline checker. A checker bug is possible in either direction; the pass/fail validation examples are in dev-baseline's `checker_validation.md`.

## 9. Analysis (computed by code written before the runs; no analysis choices after seeing data)

### 9.1 Quantities per (arm, specimen)
For each stack the run is **passing** if it passed within 6 turns. For each count variant V ∈ {P, P0, H, H0, output-only, secondary-P} (all computed by `recount.py`):
- `T_PVC(V)` — PVC's `T_total` if PVC passed;
- `T_base(V)` — the **lower** `T_total(V)` among the *passing* baselines (B1, B2); exists if at least one baseline passed;
- the specimen forms a **valid pair** iff both exist; paired difference `Δ = T_PVC − T_base` (negative favours PVC).

### 9.2 Unpaired failures (stated before the run)
- **Primary analysis uses valid pairs only** (complete cases), consistent with G2's exclusion of failures from medians. The counts of the three unpaired kinds — *PVC failed, a baseline passed*; *PVC passed, both baselines failed*; *all three failed* — are printed beside every statistic. Asymmetric failures are not hidden: they enter the decision through the **pass-rate condition** (§9.4), which is how G2 §1 treats fidelity.
- **Sensitivity (reported, never decisive):** the same statistics over all specimens where a failed run is entered at its tokens-spent (a lower bound on the cost of a correct result; a failure's true cost is unbounded).
- If an arm has **no valid pair** in a category, the CI is undefined and the G2 §1 rule is **not satisfied** for that arm/category (the rule needs a CI that excludes zero).

### 9.3 Confidence interval (G2 §1: bootstrap 95%)
Paired **percentile bootstrap over specimens**: for each (arm, category, variant), resample the valid pairs with replacement, same size, **10 000** resamples; statistic = median of Δ (mean of the two middle values when n is even); 95% CI = the 250th and 9 750th of the 10 000 sorted resampled statistics (1-based). One `random.Random(20261008)` stream, consumed in the fixed order: arm S4, S7; category layout, logic; variant P, P0, H, H0, output-only, secondary-P, each followed by the relative-difference statistic. Pure standard-library Python. Because sampling is deterministic (§3), the CI describes specimen-to-specimen variability only. With few pairs the bootstrap distribution of a median is coarse and can only exclude zero if (almost) all Δ share a sign; this is accepted and reported with n.
Categories: **layout-heavy = L01–L10 analysed set; logic-heavy = G01–G05 (five of the ten category-4 specimens)**.

### 9.4 Decision rule and falsification trigger — quoted verbatim from G2 §1

> **Decision rule (fixed before any run, so results cannot be rationalised afterwards):**
> - PVC may claim fewer tokens **for a category** only if, across all agent arms (§4.2), the median paired difference in total tokens favours PVC with a bootstrap 95% confidence interval that excludes zero, **and** PVC's checklist pass rate is no more than 5 percentage points below the baseline's.
> - The README may state a headline number only as a **per-category range with its confidence interval and the arm it applies to**, never as "up to X%".
> - **Falsification trigger:** if PVC does not win the **layout-heavy** category, the category where the architecture should help most, with the context overhead included (§2 item 4), the token thesis is treated as **falsified for v0**. The finding is escalated to the owner before any further build work. Renderer quality does not rescue it.

**Operationalisation (fixed here):**
1. **Applied to each arm individually (D-7).** "Across all agent arms" is read conservatively: PVC *wins* a category only if the rule is satisfied in **every** evaluable arm.
2. **"Median paired difference favours PVC with a CI that excludes zero"** = primary variant **P** (context uncached), valid pairs, median Δ < 0 **and** the 95% CI upper bound < 0.
3. **"Pass rate"** = pass@6 over the arm's analysed specimens of the category (turn cap 6). PVC's rate = passing PVC runs / specimens. The **baseline's rate** = specimens on which **at least one of B1, B2 passed** / specimens (consistent with "reported baseline = the lower passing of B1/B2", G2 §3.3). Condition met iff `rate_PVC ≥ rate_baseline − 5 percentage points`. B1 and B2 pass rates are also reported separately; pass@1 too.
4. **Secondary-count direction check** (§5.1) is applied to each (arm, category); a disagreement makes that result *inconclusive* and the sentence says so; an inconclusive layout result is **not** counted as a win.
5. **Falsification trigger (D-8, applied as written):** it fires if the layout-heavy category is not *won* per item 1 in at least one evaluable arm — including when n is small or underpowered, when the CI touches zero, when there is no valid pair, or when the result is inconclusive. An arm is **evaluable** iff its analysed set contains at least L01–L05 and G01–G05 complete (§2). If no arm is evaluable the trigger is "not evaluated" and the report says why.
6. Category 4 gets the same rule; the trigger does not apply to it (A§1.4's prior for it is "near zero or negative").
7. Definition-dependence: if the rule's outcome under H or H0 differs from P for any (arm, category), RESULT.md flags it "definition-sensitive". The decision uses P unless the Supervisor rules otherwise (§0).

**Exact sentence if the trigger fires** (placeholders filled by the analysis code; nothing else is changed):
> **G2 §1 falsification trigger FIRED for v0.** In the layout-heavy category (specimens `<list>`, n = `<N>`; `<K>` valid PVC-vs-baseline pairs for S4 and `<K'>` for S7), with the context overhead counted uncached, PVC did not win for arm(s) `<arm list>`: `<per-arm: median paired difference X tokens, 95% bootstrap CI [lo, hi] / pass rate PVC a% vs baseline b% / inconclusive under secondary count / no valid pair>`. Per G2 §1 the token thesis is treated as falsified for v0 and the finding is escalated to the owner before any further build work. Both arms are small local models (Qwen3-4B Q4_K_M, thinking off; Qwen2.5-Coder-7B-Instruct Q4_K_M); no hosted/frontier arm was run; the result speaks only for that class.

**If it does not fire:** *"G2 §1 falsification trigger did not fire: in the layout-heavy category (n = `<N>`, arms S4 and S7) PVC met the decision rule in both arms (S4: median paired difference X, 95% CI [lo, hi], K pairs; S7: …), context overhead uncached. This speaks only for small local models; no hosted/frontier arm was run."* If an arm is not evaluable the sentence names it ("S7 not evaluated: only n specimens completed") and says the trigger was evaluated on the other arm only.

### 9.5 Reported (all required by G2 §2, per category, per arm, per stack)
`T_total` per run (all variants); median and 90th percentile (nearest-rank on passing runs); pass@1 and pass@6; output-only tokens; cached-at-zero variants; the per-category range with CI (§5); the decision-rule outcome per arm; the falsification statement; fixed overhead tokens (PVC context, B1, B2 contexts) as one number each; template overhead from `usage`; counts of length-truncated turns, `NO-ARTIFACT` turns, and unpaired failures. Not reported (not run): `T_session`, multi-language arm, H-item fidelity.

## 10. Reproducibility and audit
Every turn is saved verbatim in `transcript.jsonl` (header, one message record per chat message with exact text and per-segment o200k and model-token counts, the server's `usage` and `timings`, timestamps, the artifact path and checker/tool outputs, footer). The header carries the model sha256, llama-server argv, sampling, the sha256 of the context and request text, and the sha256 of the harness (a hash over every harness source file). `harness/recount.py <transcript.jsonl>` recomputes every count from the saved text and exits non-zero on any mismatch. `harness/tests/test_harness.py` tests the loop, extraction, accounting and the real-run guard with a fake server (no LLM).

**CPU lock (Orchestrator amendment): exclusive for the whole run.** A real pilot run (`pilot.py run`, one arm) takes `flock -x` on `/workspace/orchestrator_work/pm/CPU_LOCK` once, before the llama-server starts, and holds it through server start, every model request, every `/tokenize` call, every tool/checker subprocess and server stop; the harness refuses a real run in any other lock mode. Consequence: while an arm is running, every other job that takes the lock (builds, browsers, measurements) waits, for the length of the arm. Dry runs and tests use the earlier shared per-call mode (`--lock shared-per-call`, same semantics as `flock -s`), recorded as `cpu_lock_mode` in each transcript header. Pilot runs themselves are not timing measurements.

## 11. Compute plan (*estimate*, from F18 throughput; not a schedule)
Upper bound of 2 arms × 15 specimens × 3 stacks × 6 turns = 540 model turns. Generation speed measured by `llama-bench` (FINDINGS F18): S4 9.4 tok/s, S7 5.5 tok/s with the machine otherwise idle. Wall time is dominated by output length; with the exclusive lock (§10) nothing else should share the CPU during a real run, so the starvation seen in the dry-run preparation is not expected, but an arm may block other Developers for hours (*estimate*: S4 ≈ 270 turns). **Observed during harness preparation (not data):** while other Developers were compiling under shared locks (load average 15–17), a llama-server generation slowed from ~8 tok/s to 0.05–2 tok/s (`dryrun_aborted_F01_starved/`, `_server/` log); thread oversubscription hurts a 4-thread llama.cpp badly. **Token counts do not depend on speed**, but the pilot should run in a quiet CPU window. **Memory (observed with `ps`, S4 server at `-c 32768`):** RSS ≈ 9.7 GB (model ≈ 2.5 GB + f16 KV cache for 32 768 tokens ≈ 4.8 GB + buffers) of the 15 GB container; other jobs that need several GB at the same time can push it into swapless OOM. Lowering `-c` would cut this but must stay above the PVC context plus ~22k tokens of replies; fixed at 32768 here unless the frozen PVC context makes a smaller value safe.

## 12. Inputs whose hashes must be filled before the run ⟦ALL PENDING⟧
| Input | Owner | Hash / value |
|---|---|---|
| PVC agent context `FROZEN.md` v2 (system message sha256 + o200k count) | dev-ir | **filled (§4.2)**: `d3050304…20d9`, 2511 tokens; enforced by the harness |
| `harness/contexts/B1.md` sha256 / o200k | dev-pilot (reconciled with dev-baseline) | draft: `530f9d70087de982da4bd23e986de5178f23958261fbdb5449774d7dd9ee5942`, 120 o200k tokens (measured with `pilot.py context-info`); ⟦final after dev-baseline reconciliation⟧ |
| `harness/contexts/B2.md` sha256 / o200k, library name+version | dev-pilot + dev-baseline | ⟦PENDING⟧ |
| `checklists/INDEX.md` hash | dev-baseline | `074a7e39…b036fd` (filled, §8) |
| tool + checker command lines and output formats | dev-ir, dev-baseline | `pvc-validate`: filled (§4.2). ⟦PENDING: `check-pvc`, `check-baseline`, html-validate, tsc, axe wrappers; pvc-validate binary hash at freeze⟧ |
| intents table (§4.4) | dev-pilot from dev-baseline fixtures | filled (§4.4) |
| `harness/` sha256 (`harness_sha256` in each transcript header) | dev-pilot | ⟦computed at freeze⟧ |
| analysis code `harness/analyse.py` (§9), tested on synthetic data by `harness/tests/test_analyse.py` (passes) | dev-pilot | covered by `harness_sha256`; ⟦final hash at freeze⟧ |
| Supervisor ruling on §0, D-7 reading, max_tokens, L01 sentence | Supervisor | CONFIRMED |

## 13. Deviation list
From PLAN §5: **V7** both arms are local small models (Qwen3-4B Q4_K_M, Qwen2.5-Coder-7B-Instruct Q4_K_M); G2 §4.2's frontier and mid-tier hosted arms F and M are not run; G2 §4.2's "at least three arms" is not met. **V8** H items not judged (D-5). **V9** intent texts drafted by the PM, not outsiders (G2 §4.1/§7.3 unmet); in addition the fixtures were written by dev-baseline. **V10** secondary count is llama.cpp's tokenizer, via `usage` and `/tokenize`, not vendor billing.
Added here:
- **D-F** request = G2 intent + dev-baseline fixture sentence(s) (§4.4), deviation from G2 §4.1 "same intent text" (Orchestrator ruling). L01 uses my five-strings sentence instead of dev-baseline's non-appendable fixture block. Doc-reference parentheticals `(the README card)` and `(A§2.9)` removed.
- **D-B2** one fixed component library for B2, not agent-chosen (G2 §3.3); output restricted to a single `App.tsx` file.
- **D-T** tools are run by the harness, not called by the agent; tool output counted only when sent to the agent (§4.3). B2 gets `tsc` + `axe-core`, not `html-validate` (G2 §4.3 reading).
- **D-S** G2 §4.2 says arm S "running on a Tier-1-class laptop"; here it runs on the container's 4-vCPU Xeon (host label above). S7 (7.6 B params) is within the G2 "≤ 8B" class; the Supervisor brief's "≤4B" requirement is met by S4 alone.
- **D-C** category 4 is represented by G01–G05 only; category 1 by L01–L10; categories 2, 3, 5, 6, `T_session`, and languages are out of this pilot (D-3).
- **D-U** a draft pseudo-item `BUILD` / `NO-ARTIFACT` repair text (§7) is not in G2.
- Baseline contexts do not mention accessibility; the PVC context does (it is the language's own rules). Any difference in repair turns caused by that is part of what G2 §3.1 charges the baseline, and is not corrected.
- Qwen3-4B with temperature 0 may repeat itself; Qwen's own guidance for sampling differs. Fixed anyway, as G2 requires a single configuration (no per-arm tuning of PVC).

## 14. Dry run (plumbing only)
`DRY-RUN — not data.` One dry-run specimen `D00` (a one-sentence page; **not** a pilot specimen), arm S4, stack B1, with stub tools and a stub regex checker (`harness/adapters/stub_*.py`; **not** html-validate, **not** dev-baseline's checker). Output in `dryrun/` (never in a results directory; `recount.py`/`analyse.py` refuse to treat `DRY-RUN` transcripts as data). It ran real Qwen3-4B through llama-server b11514 for 2 turns (turn 1 failed the stub's third item, standard repair text was fed back, turn 2 passed). It proved: server start/stop, model sha256 check, message construction, o200k and `/tokenize` counting, fenced-block extraction, tool → check → feedback, transcript writing, recount agreement (`dryrun/recount_DRYRUN.txt`). No number from it enters any result. An earlier dry-run attempt on F01 was aborted because of CPU starvation (§11) and is kept in `dryrun_aborted_F01_starved/`, also not data.

---
*Hash of this file: computed after the ⟦PENDING⟧ items are filled; announced to the Supervisor as "PREREG ready: <path> sha256 <hash>".*
