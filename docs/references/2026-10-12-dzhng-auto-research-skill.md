# Reference: dzhng's "auto-research" skill, and what it can do for PVC

Researched 2026-10-12. Read-only. The skill is not installed and was not run. Nothing was copied into this repository.
Source: https://github.com/dzhng/skills, file `skills/engineering/auto-research/SKILL.md`, read at commit `d513228`
(latest commit 2026-10-05, measured with the GitHub API). Also read: the repo `README.md` and `LICENSE`.
Related notes: [explore-unknowns skill](2026-10-12-dzhng-explore-unknowns-skill.md),
[abstraction layers](2026-10-12-dzhng-delete-abstraction-layers.md).

## Summary

1. The skill is one prompt file (10,825 bytes, measured). It has no scripts, no reference files and no dependency on
   other skills. It tells an agent to run an experiment loop: one baseline, one change at a time, keep or discard, then
   a written table of what each change did.
2. It needs a checkable success rule, a fixed evaluator and a baseline. It asks the user for them if they are missing.
   After that it runs without more questions until the target passes, a budget ends, or a blocker appears.
3. It fits PLAN.md steps 5 and 6 (cut the context, shorten the language). It does not fit steps 1–4 (harness and tool
   engineering) and it cannot replace the step 2 baseline or the step 7 confirmation.
4. Its "cost per task" has no fixed definition, so PVC must give it M1's "language tokens" formula. The main risk is
   tuning on T1–T6 and then claiming M1 on T1–T6. PVC needs a separate development task set first.
5. The license is MIT, so copying is allowed with the notice. Recommendation: write the research contract for step 5
   by hand now (no install). Run the skill once at step 5 with that contract. Adapt it only if that run helps.

## 1. What the skill is and what it does

**Purpose.** The `description` field says: "Run fast, progressive experiments to map tunable parameters and their
effects." It is for "optimizing code, prompts, configurations, or other artifacts against a goal or benchmark". The
text states the goal as "Optimize **learning per unit of time**" (`SKILL.md`, introduction).

**Steps.** `SKILL.md` has eight numbered steps ("Workflow"):

| # | Step | What the agent does |
|---|---|---|
| 1 | Recover the research state | Reads the objective, instructions and earlier records. Finds the best verified artifact (the "incumbent"), the hypothesis queue and the remaining budget. |
| 2 | Clarify success | Asks the user for the evaluator, metric, baseline and required improvement if any is unclear. Waits for the answers. Then picks "one cheap task that exposes the behavior being improved". |
| 3 | Baseline | Runs the unchanged artifact once, or reuses a recorded result. "Keep the baseline fixed for this task/model/harness; do not rerun it per variant." |
| 4 | Plan a hypothesis batch | Lists causes of the bottleneck from real failure traces. For each: the change, the mechanism, the predicted result, the result that would reject it, the cost. Ranks by "information gained per unit of time". |
| 5 | Test one hypothesis | Changes one factor. Records the prediction before the run. Runs cheap checks, then the focused task. Records `keep`, `discard`, `inconclusive` or `invalid`. |
| 6 | Combine and promote | Tests the winners together. "Do not assume gains add." Checks the result against earlier solved tasks (the regression set) with their saved baselines. |
| 7 | Expand | Adds one new task only after the active task passes and earlier tasks stay green. |
| 8 | Replan | After a batch, consolidates the results and picks the next batch. "Continue without asking whether to proceed after each experiment." |

**What it produces** (`SKILL.md`, "Durable research record"): five compact records in the project's own format: the
contract, the hypothesis queue, the "parameter-effect map" (one row per parameter: range tested, effect on quality and
cost, interactions, confidence, evidence), the attempt ledger, and a handoff note. The text says the map "is a primary
deliverable, not just a leaderboard". At the end it delivers "baseline versus best verified results, coverage and
limitations, budget consumed, stop reason, and exact next experiment".

**Rules on evidence** (`SKILL.md`, "Evidence discipline"). These match PVC's own rules closely:
- "A smaller output or faster microbenchmark is a diagnostic until end-to-end quality and cost gates pass."
- "never select a favorable baseline or repeat after seeing scores."
- "Keep development and confirmation separate. Repeatedly tuned cases are development evidence."
- "Account for failed trials and retries in research spend." Infrastructure failures are neither wins nor regressions.
- "Missing results are unknown, not zero" (step 5).

**What it needs.**
- Tools: any harness that loads skills. The file names no tool, no model, no API key and no script. The agent uses the
  project's own evaluation command. For PVC that is `pilot/owner-test/run.py`, which needs the `claude` command and a
  Rust toolchain (`pilot/owner-test/README.md`).
- A baseline: step 3 runs one, or reuses a recorded one. The identity of the baseline and the comparison rule must be
  settled before any experiment (step 2).
- Time: the file gives no figure. The README's "3d 18m" unattended run is for the `/goal /implement-spec` pipeline on
  Codex, not for this skill.
- Evidence of effect: the README and the author's claims only. I found no evaluation of the skill itself.

**Does it run unattended?** Partly. Step 2 stops and asks questions until the success rule is checkable. After that the
agent continues "within the authorized session and limits until the target passes its required validation, the user
stops the work, a budget is exhausted, or a genuine external blocker prevents useful progress" ("Stopping"). It does
not schedule itself: "this skill does not itself schedule future work". It names no subagent; one agent can run it.

**Sibling skills.** None are required. The README says to use the result "to inform a spec" (`write-spec`), which is
optional. The README example is
`/auto-research Reduce cost per task by at least 15% relative to the saved baseline, without reducing task success. Start with one fast development task.`
The 15% is an example for the user's own task. The skill sets no threshold itself and says not to "invent a threshold".

## 2. How it can help PVC, and where it does not fit

### 2.1 Map onto PLAN.md

| PLAN step | Fit | Reason |
|---|---|---|
| 1. Make the test fair | No | A design decision about prompts and tools. No experiment loop is needed. Skill step 3 ("verify the evaluator measures the intended outcome") supports the check afterwards. |
| 2. Baseline with variance | Method only | The harness work is engineering. The skill's rules help: fixed baseline, raw output saved, exact inputs. PLAN requires 3 runs × 6 tests × 2 agents = 36 sessions. The skill says "plan only the repeats needed". PLAN and M1 rule here, not the skill. |
| 3. `pvc check` | No | A tool change with a known effect (one fewer request). Measure it; do not search for it. |
| 4. Context as a skill | No | Same reason. |
| 5. Smaller core card | **Yes** | The core card has many parameters: which components stay, example choice, wording, the card size cap (target 1,000 tokens, PLAN step 5). Each is a hypothesis. PLAN step 5 already says to revert parts that raise request counts or failures. The skill formalizes that loop and records a table of effects. |
| 6. Shorter language | **Yes** | One spelling or default per hypothesis. Regression set: the 15 pilot reference programs and the Rust tests. PLAN step 6 already requires them. |
| 7. Final measure | Rules only | The skill's "keep development and confirmation separate" is what step 7 needs. It does not do the step. |

### 2.2 What PVC must provide

1. **A baseline.** The step 2 output, `docs/session3/BASELINE.md`. It does not exist yet. `tests-run/` holds only its
   `README.md` (measured, directory listing today). The incumbent is context v2 (2,511 o200k tokens, measured,
   `pilot/agent-context/FROZEN.md`) with the fair harness from step 1.
2. **A task set, split in two.** A development set for the loop and a confirmation set for the claim. The owner defined
   M1 as the median over T1–T6 (`PLAN.md` §1). If the loop tunes on T1–T6, those tests become development evidence by
   the skill's own rule and by `tests.json` ("Change them only before a run, never after you see results"). Needed:
   about 4 new development tasks of levels 1–3, written and checklisted before the loop starts. They must not be T1–T6
   and must not be the 15 pilot intents (`pilot/intents/`). The pilot intents are reserved for G2. The agent context
   states that it was "not tuned to the pilot intents" (`pilot/agent-context/README.md`).
3. **A success check.** The evaluator is a language model that reads code and screenshots. `pilot/owner-test/README.md`
   says it is "not the automated checker the G2 pilot specifies". T3, T5 and T6 are marked `pvc_renders: false`, so
   the evaluator judges them from code (`tests.json`). The skill asks the user to prove the evaluator detects a failure
   ("a known failing control"). PVC must prepare that control. It must also state how M1's "pass rate not lower" counts
   tests the runtime cannot draw.
4. **A cost measure.** M1's language tokens = total session tokens − (model requests × base tokens per request)
   (`PLAN.md` §1). `run.py` today reports only the total, split by input, output and cache, plus `cost_usd`
   (`usage_of`, `run.py` line 256). The request count and the base measure are step 2 work. Without them the agent
   would optimize dollars or total tokens, which is a different target.
5. **A noise rule, fixed before the loop.** Runs per candidate, and the size of difference that counts. PLAN §6 already
   says "make no claim when the ranges overlap". The skill needs this as a number, not as a judgment after the fact.

### 2.3 Does "cost per task" match M1's "language tokens"?

**Partly.** The skill does not define cost. Its README example says "cost per task". The skill asks the user for the
formula and says to "state objective cost inclusions/exclusions". That is the same idea as M1 excluding the coding
tool's base prompt. So the formula can match, but PVC must write it into the contract.

Two differences:
- **Reference.** The skill measures improvement against the saved baseline of the same artifact (here: PVC v2). M1
  measures a ratio to a control agent (bar: 0.50 or less, owner decision, `PLAN.md` §1). The inner loop can use the
  relative cut against the PVC baseline. The M1 ratio is the outer gate. The control arm runs once per task and stays
  fixed ("do not rerun it per variant").
- **Size of the step.** The README example is 15%. PLAN §1 estimates that M1 needs about a 10-times cut on small apps
  (*estimate*, from one Sonnet T3 run: PVC 42,680 total tokens, control 16,594, measured in `PLAN.md` §2). The loop
  does not reach 10 times by tuning alone. In that run the main causes were extra requests and the re-read context
  (`PLAN.md` §2). Steps 3 and 4 change those. Steps 5 and 6 tune what is left.

### 2.4 Where it does not fit, or can break a PVC rule

1. **Pre-registered bounds.** Do not point it at `measurements/session2/golden/TOLERANCE.md` (A7) or `pilot/PREREG.md`
   (G2). A loop that "improves" a renderer until it passes the FLIP bound re-tunes the evidence. The skill's own rule
   against moving criteria is compatible, but a prompt to "make it pass" is not. The contract must list these files as
   read-only.
2. **Frozen context.** CLAUDE.md: "The frozen agent context changes only with a new version, a new hash and a commit."
   Each `keep` must become a numbered candidate (v3a, v3b …) with a hash. Each `discard` stays a patch or branch, not a
   frozen version. `PREREG.md` names v2, so the G2 pilot is not affected until the owner updates PREREG (`PLAN.md` §6).
3. **Owner decision 6 (one focused agent, plan approved first, commit after each step).** The skill says to continue
   "without asking whether to proceed after each experiment". These fit only if the owner approves the contract
   (objective, evaluator, development set, budget, stop rule) before the loop starts, the agent runs alone and in
   sequence, and a commit follows each `keep`. Without that, the skill's autonomy conflicts with the rule.
4. **Owner decision 5 (accessibility is release-blocking).** A card that is shorter but leads agents to omit accessible
   names lowers the checklist pass rate (items such as T3.5, T4.3–T4.5). The non-regression gate must include those
   items and the validator's accessibility checks. Prose removed from the card must stay enforced by the validator.
5. **Owner decision 3 and 4.** The loop runs with cloud models on the container. No conflict. Local-model arms stay
   outside it.
6. **Labels.** The skill separates "measured effects from suspected mechanisms". This helps the "measured or estimate"
   rule. Every number in the map must still carry a file path to a run in `tests-run/`.
7. **Selecting noise.** With 3 runs per cell and a language-model judge, many keep/discard decisions will pick
   random differences. The step 6 "confirm noisy wins" rule helps only if the noise rule of §2.2 item 5 exists.
8. **A tiny first task.** The skill advises one cheap task first. A tiny task shows the fixed overhead, which is the
   main lever. It does not show how code size grows on T4–T6. The regression and expansion steps must cover a level 3
   task before any claim.
9. **Tooling traps.** The agent must use `run.py`, not its own token counting. `docs/session3/NOTES.md` §3 lists the
   traps (session IDs, which usage field to sum, duplicate usage events). Put a link in the contract.
10. **Style.** The skill text uses terms such as "incumbent", "bank" and "map". A PVC copy needs a rewrite in
    ASD-STE100 (CLAUDE.md).

## 3. Options to move PVC forward

All token and money figures are *estimates*. None is measured.

### Option 1. Write the research contract for step 5 by hand (no install)

- **What to do.** Add a section to `docs/session3/PLAN.md` step 5 (or a file `docs/session3/STEP5-CONTRACT.md`) with
  the items the skill asks for: the M1 formula, the baseline file, the development and confirmation sets, the
  evaluator command and its failing control, the parameters allowed to change (card contents and size cap, example
  choice), the read-only files (§2.4 item 1), the noise rule, the budget and the stop rule. Add a table for the
  attempt ledger and for the effect of each parameter. Step 6 reuses it.
- **Cost.** About 10,000–30,000 tokens to write. No install. No new measurement. Time: one session.
- **Owner decides.** (1) The development tasks (new, not T1–T6, not the pilot intents). (2) The noise rule. (3) The
  budget. (4) Whether the per-step T1–T4 runs in PLAN (steps 3–5) count as development evidence. They give the
  context repeated exposure to T1–T4 before step 7. Moving them to the development set removes the question.
- **Recommendation.** Yes, now. It is cheap, it needs nothing from the skill, and the later options depend on it.

### Option 2. Run the skill once, unchanged, at step 5 with that contract

- **What to do.** After the step 2 baseline exists and steps 3–4 are done, copy the single file to
  `.claude/skills/auto-research/` in a working copy (or paste its text into the first prompt). Start it with the
  option 1 contract as the objective. Run it on a branch. Commit each `keep` as a context candidate with its hash.
  Run T1–T6 once at the end as the confirmation, with the control already fixed from step 2.
- **Cost.** Sessions: about 300 `claude -p` sessions (estimate: 8 hypotheses × 4 tasks × 3 runs, plus combinations,
  plus the evaluator session for each), at $0.01–0.05 each (`PLAN.md` §5, estimate), so about $3–$15 (estimate).
  Lead agent: 200,000–600,000 tokens (estimate, no measurement). Time: 4–10 hours of sequential runs (estimate;
  `pilot/owner-test/README.md` says "a few minutes per test"). Record the real numbers in the map.
- **Owner decides.** (1) The budget cap and the stop rule. (2) Whether the lead agent may run without check-ins
  after the contract is approved (decision 6). (3) That the skill may be installed on the working machine. (4)
  Whether the result must also pass a second confirmation on fresh tasks before M1 is claimed.
- **Recommendation.** Yes, once, at step 5, only after option 1 and after the step 2 baseline. Do not run it before
  the baseline exists: the skill step 3 would build its own baseline, and that would not be the M1 baseline. Judge
  it by one question: did it find a card change that a hand-run step 5 would not have tried?
- **License.** MIT, "Copyright (c) 2026 David Zhang". Using one unchanged file needs the license text kept with the
  copy: put upstream `LICENSE` next to `SKILL.md`. This repository's own license is also MIT, so there is no conflict.
  This is not legal advice.

### Option 3. Adapt it into a PVC project skill (`pvc-tune`)

- **What to do.** A single file in this repository. Keep: the loop order, "one factor at a time", the ledger, the
  effect map, "development and confirmation separate". Add as fixed inputs: M1 formula, owner decisions, "label
  every number", "do not re-tune a pre-registered bound", the frozen-context rule, the read-only files. Remove the
  generic questions of step 2. Write it in ASD-STE100.
- **Cost.** About 15,000–30,000 tokens to write and one real run to test (which is option 2).
- **Owner decides.** Name and place (`.claude/skills/` or a plugin entry beside `pvc-test`). Whether contributors
  get it at install. Whether a second skill (the step 4 core card skill) confuses agents that load both.
- **Recommendation.** Not now. Decide after option 2. If option 2 shows value, a PVC-specific copy removes the
  setup cost for later phases. If not, nothing was built. If a copy keeps substantial text, keep the upstream
  license notice. A rewrite from the method alone needs no notice, but give an attribution line.

### Option 4. Do not adopt it

- **What to do.** Run steps 5 and 6 by hand under PLAN.md, as written.
- **Cost.** None.
- **Risk.** PLAN steps 5–6 already hold most of the discipline: one change per step, a pass-rate gate, revert what
  hurts, report ranges. The skill adds an explicit hypothesis queue with predictions and rejection criteria, an
  effect table that survives the session, and a written development/confirmation split. Without these, step 5 can
  end with a smaller card and no record of why each part was cut.
- **Recommendation.** Acceptable for steps 1–4. Not recommended for steps 5–6 without at least option 1.

### How this fits with explore-unknowns

| When | explore-unknowns (earlier note) | auto-research (this note) |
|---|---|---|
| Now, with steps 1–3 | Option C: an unknowns ledger per phase, written by hand | Option 1: the step 5 research contract, by hand |
| Step 2 | — | Supplies rules only (fixed baseline, raw output) |
| Step 5 (and 6) | — | Option 2: one run of the skill with the contract |
| Boundary after PLAN.md (order of phases 2–4) | Option A: run the skill once | — (no experiment can answer an order-of-phases question) |
| After the first real use of each | Option B only if the run helped | Option 3 only if the run helped |

The two skills answer different questions. Explore-unknowns lists what nobody has decided or tested. Auto-research
finds, by experiment, which change cuts tokens. The unknowns ledger can feed the contract: its open items about M1
(how untested tests count, whether an installed skill's description is part of the base prompt) must be closed
before the loop starts, because they change the cost formula. If the owner installs both, each adds its description to
the lead session's prompt. Check with an empty session that the M1 test sessions do not load them (`run.py` uses
`--restricted`; I did not verify that this blocks installed skills).

## Open points that I could not settle

- Whether any runs of this skill by others show a measured saving. I found only the author's README.
- The real token cost of one loop on PVC. Option 2 measures it.
- Whether the number of hypotheses worth testing at step 5 is 8 or fewer. The first run of step 5 by hand shows
  how many separate parameters the core card has.
- Whether `--restricted` removes installed skills from the M1 test sessions (see above). Test with an empty session
  before step 4.
