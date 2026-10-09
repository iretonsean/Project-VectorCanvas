# Session 3 plan: cut PVC's fixed cost per session

**Status:** approved by the owner, 2026-10-09; goal amended by the owner the same day (milestone M1, §1). Work has not started.
**Executed by:** one focused agent, step by step, with a commit and a measurement after each step.

## 1. Goal: milestone M1

**Owner decision, 2026-10-09:** cutting the token cost of building apps with AI models is the highest priority of the project. The first milestone of the language is a 50% cut.

**M1, exact definition:**
- **What is compared:** a PVC session against a control session for the same app, the same description, the same model and the same effort level (`pilot/owner-test/`).
- **What is counted:** *language tokens*: the session's total tokens minus the coding tool's own base prompt, which is the same for both agents. Measured as: total tokens − (model requests × base tokens per request). The base per request is measured with an empty session that uses the same tool, version and flags (step 2).
- **The bar:** the median, over T1–T6, of (PVC language tokens ÷ control language tokens) is **0.50 or less**. Each test runs at least 3 times per agent; the median of the 3 runs is used for each test.
- **Quality:** PVC's checklist pass rate is not lower than the control's (the G2 rule: no more than 5 percentage points lower).

**Also reported in every result, but not part of the bar (owner decision):**
- total session tokens, as the tool reports them;
- final code size in o200k_base tokens.

**How far away M1 is (estimate):** in the one Sonnet T3 run of §2, the first request cost about 6.8k tokens of tool base and prompt. With that as the base per request, PVC used about 15k language tokens and the control about 3k: PVC is about **5 times** the control, so M1 needs about a **10-times** cut on small apps. Larger apps are expected to be easier, because the control's code grows faster than PVC's (*estimate*). Step 2 replaces this estimate with a measurement.

This plan takes the first steps toward M1: it removes the fixed costs that PVC adds to every session. If M1 is not met at step 7, the result says by how much, and the owner decides the next work.

## 2. Measured starting point

One T3 (counter) session per agent, Sonnet, Claude Code 2.1.295, container host, 2026-10-09:

| | PVC | Control |
|---|---|---|
| Total tokens (Claude Code report) | 42,680 | 16,594 |
| Model requests | 4 | 2 |
| Final code size (o200k, from the earlier Haiku run) | 108 | 497 |

Request by request, PVC:

| Request | What the agent did | Tokens re-read (cache read + write) |
|---|---|---|
| 1 | Read the three context files | 4,775 + 2,046 |
| 2 | Wrote app.pvc and ran the validator | 6,821 + 4,038 |
| 3 | Ran the compiler | 10,859 + 409 |
| 4 | Wrote its final message | 11,268 + 168 |

The control wrote one file and finished in two requests. It ran no check.

**Where the cost comes from (in order of size):**
1. **Extra requests.** Each request re-reads Claude Code's own base prompt (about 5–7k tokens) and everything said before. PVC makes two more requests than the control: one to read the files and one to compile after validating.
2. **The context on every later request.** The three files (2,511 o200k tokens; about 4k tokens as Claude counts them) are re-read on each request after they are loaded.
3. **The code itself.** This is small, and PVC already wins here (108 against 497 tokens).

So the main lever is the number of requests, then the size of the context. The language syntax is the smallest lever.

## 3. Rules for this work

- The validator stays the source of truth. Nothing here weakens a check.
- The hard wall (no model in a shipped app) is unchanged.
- The agent context v2 (`pilot/agent-context/FROZEN.md`) stays as the historical record. Each new context version gets its own hash and commit.
- A change that saves tokens but lowers the checklist pass rate does not count as a saving.
- Every claim of a saving cites a run file in `tests-run/`.

## 4. Steps

Each step ends with a commit and a run of T1–T4. Steps 1 and 2 come first because the later numbers mean nothing without them.

### Step 1. Make the test fair

**Problem:** PVC is told to validate and compile; the control is told nothing about checking. PVC pays two requests for checks that the control skips.

**Change:**
- The harness compiles the PVC app itself. The agent is not asked to.
- Both prompts get the same sentence: "Check your work once with the tools in this folder, if there are any."
- The control folder gets a check tool of the same kind (an HTML validator if one is available on the machine; otherwise the sentence is the only change, and the report says so).

**Done when:** both prompts differ only in the language instructions, and `pilot/owner-test/README.md` states the rule.

### Step 2. Measure a baseline with variance

**Change:**
- Add the M1 measure to `pilot/owner-test/run.py`: count model requests per session, measure the tool's base tokens per request with an empty session (same tool, version and flags), and report language tokens beside total tokens and code tokens in every result file and in `tests-run/INDEX.md`.
- Run T1–T6 three times each, with Sonnet and the default effort level, and with the fair harness from step 1.

**Done when:** `tests-run/` holds 36 sessions (18 per agent) and `docs/session3/BASELINE.md` gives, per test and agent, the median and range of language tokens, total tokens and code tokens, and the M1 ratio. One run per test is not enough to compare.

### Step 3. One check, one request

**Change:** add `pvc check <file>`: validate, then compile when the file is valid, in one command with one short output. The diagnostic format does not change.

**Expected effect:** one fewer request when the agent compiles itself, and shorter tool output.

**Done when:** the PVC agent's usual path is "write, check once, finish", seen in the step-2 style runs.

### Step 4. Deliver the context the way a developer would

**Problem:** the agent spends a whole request reading three files.

**Change:** package the PVC context as a Claude Code skill (`SKILL.md` with the core card) that loads into the first request, with no separate read. Any agent with a skills mechanism uses the same text. Agents without one get the same text in their first message. This is also the first piece of the developer toolchain (phase 3 below).

**Expected effect:** one fewer request (about 7k tokens on a T3-sized app).

**Done when:** the PVC agent's first request already contains the core card, and the harness uses the skill, not loose files.

### Step 5. A smaller core card, the rest on demand

**Problem:** the full 2,511-token context is re-read on every request, but a small app uses a small part of it.

**Change:**
- Measure which parts of `01_std_reference.md` and `02_dialect_en.md` agents use, from the step-2 transcripts and the 15 pilot reference programs.
- A core card with the grammar, the five most used components and one full example. Target: at most 1,000 o200k tokens.
- The rest moves behind a lookup: `pvc explain <component>` and richer validator diagnostics, which already carry `fix=` hints. An agent pays for a component's reference only when it uses that component.

**Done when:** the core card is at most 1,000 tokens, the T1–T4 pass rate is not lower than the step-2 baseline, and the context is frozen as v3 with its hash.

### Step 6. Fewer tokens in the language itself (only if steps 3–5 are not enough)

**Change:** shorter defaults and spellings for the most common patterns found in step 5 (for example default gaps and padding, shorter names for common properties). Every change must keep the 15 pilot reference programs valid, or update them in the same commit.

**Done when:** the median code size of the pilot reference programs drops, and no validator test regresses.

### Step 7. Measure again and report

**Change:** repeat the step-2 runs with the final setup. Write `docs/session3/RESULT.md`: the median and range per test and agent, before and after, each step's share of the saving, and whether M1 (§1) is met, and if not, by how much.

**Done when:** RESULT.md exists, the README states only what was measured, and the analysis §8.1 gets a dated note. The confidence figure changes only if the result supports a change.

## 5. Size and cost (estimate)

- Steps 1, 3 and 4: small (harness and CLI changes).
- Step 5: medium (measurement, rewrite, re-freeze).
- Step 6: medium, and only if needed.
- Runs: two rounds of 24 sessions plus a check run after each step. With Sonnet on these small apps, one session costs about $0.01–0.05 (estimate from today's runs), so the measurement cost is small.

## 6. Risks

| Risk | What we do |
|---|---|
| A smaller card makes agents fail more, so they need more repair requests | Pass rate and request count are measured in every run; step 5 reverts parts of the card that raise either |
| Skill delivery works only in Claude Code | The same text is delivered in the first message for other agents; the plain-text card is the source and the skill wraps it |
| Claude Code's own base prompt dominates small sessions, whatever PVC does | Report it as its own number; it is equal for both agents, so the comparison stays fair |
| Results from three runs are still noisy | Report the medians and ranges; make no claim when the ranges overlap |
| The G2 pre-registration names context v2 | PREREG is still interim; the next session updates it to the final context before any pilot run |

## 7. Later phases (noted, not in this plan)

1. **Phase 2: runtime gaps.** Make the runtime draw what the validator accepts: arithmetic and decimal expressions, lists (`view`, `append`, `remove`), grid layout, screen-size rules, shapes, layers and slots, and link actions. After this, all six owner tests show a working PVC app. Today only surface, box, text and text input are drawn.
2. **Phase 3: developer toolchain.** One `pvc` command (check, compile, preview) for macOS, Windows and Linux, and a tool interface (for example MCP) so any cloud or local agent can call the validator. Steps 3 and 4 above are its first pieces. This also replaces the test signing key with a real signing path.
3. **Phase 4: language reference.** A complete v0 reference for developers and agents, written from the grammar note (`docs/session2/dev/dev-ir/GRAMMAR.md`) and the G4 specification, and a larger standard component library.

Phase order after this plan is the owner's decision.
