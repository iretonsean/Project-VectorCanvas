# Reference: dzhng's "explore-unknowns" skill, and what it can do for PVC

Researched 2026-10-12. Read-only. The skill is not installed. Nothing was copied into this repository.
Source: https://github.com/dzhng/skills, folder `skills/engineering/explore-unknowns/` (files read: `SKILL.md` and six
files in `references/`, 17,039 bytes in total; also the repo `README.md` and `LICENSE`).
Related note: [dzhng on abstraction layers](2026-10-12-dzhng-delete-abstraction-layers.md).

## Summary

1. The skill is an interview that fills a four-part map of a task before any code is written: known knowns, known
   unknowns, unknown knowns, unknown unknowns. The map is the output. Building is a separate task after it.
2. It costs the owner's time and the agent's tokens at planning time. It does not change the tokens that M1 counts,
   because M1 counts PVC-versus-control app sessions, not PVC development sessions.
3. It fits PVC's decision-heavy points: the order of phases 2–4, the pilot arms, and the rule for tests the runtime
   cannot draw. It does not fit the mechanical steps of `PLAN.md` (steps 1–3), which already have a measured baseline.
4. The license is MIT. Copying is allowed if the copyright and permission notice stay with the copy.
5. Recommendation: record the unknowns of the next phase by hand now (option C, cheap). Run the skill once, as it is,
   when the owner chooses the next phase (option A). Adapt it only if that run helps (option B).

## 1. What the skill is and what it does

**Purpose.** The `description` field in `SKILL.md` says the skill guides "the user through a quadrant walk that maps the
unknowns of a task". It is for requests that are "ambiguous or underspecified", for unfamiliar code, and for changes
that need "buy-in or verified understanding before merge".

**Steps.** The walk has five stages, in order, one at a time (`SKILL.md`, "The Quadrant Walk"):

| Stage | File in `references/` | What the agent does | What the user gets |
|---|---|---|---|
| 1. Known knowns | `stage-1-known-knowns.md` | Scans the code first, with parallel subagents. Then lists what is settled, with file citations. Marks assumptions as "settled unless you say otherwise". Names the next three stages. | The settled facts, and the assumptions the agent treats as settled |
| 2. Known unknowns | `stage-2-known-unknowns.md` | Lists the questions it can name. Asks them one at a time, highest "architectural blast radius first", each with a recommended answer as lettered options. Each question closes as: user, agent on the user's behalf, territory (the agent reads the code instead), or OPEN. | A decisions table |
| 3. Unknown knowns | `stage-3-unknown-knowns.md` | Gives the user something concrete to react to (rendered options, a throwaway mock, a sample output). Asks about "who consumes this, where does it run, what does done look like to whoever inherits it". | Tacit preferences and context, written down |
| 4. Unknown unknowns | `stage-4-unknown-unknowns.md` | Sweeps every file the task touches. Reports each finding as a card: evidence (file and line), "why it bites", "what it changes". Worst first. Looks at earlier or reverted attempts and why they failed. | Findings, each decided, OPEN, or noted |
| 5. Hand over the map | `stage-5-hand-over-the-map.md` | Builds one self-contained HTML page with all four quadrants. Adds an optional "tweakable plan" and a "copyable implementation prompt". | The map, and the next prompt, ready to paste |

**After the plan.** `references/after-the-walk.md` adds three moves: a running log of deviations during the build, a
"buy-in doc" before shipping, and a quiz before merge.

**Rules that matter for PVC** (`SKILL.md`, "Rules"):
- "Stop at every stage boundary that needs the user's reaction. Never barrel into implementation on unconfirmed
  guesses."
- "Nothing closes off-screen." Every closed question must have been shown to the user.
- "Claims about the territory cite real files actually read; invented data is labeled as such."
- "Close items as decisions, not discussions." Each decision is one line plus its reason, so a spec can carry it as a given.
- An optional "Preference Checkpoint" lets the agent answer questions for the user after about five or six answers.
  The user must opt in. Each delegated answer is labeled "Answered by the agent on your behalf".

**What it needs.**
- Tools: any agent harness that supports skills (README: "Claude Code, Codex, opencode, Cursor"). Stage 1 uses
  subagents. Stage 3 and 5 use HTML artifacts, so a browser or an artifact viewer helps.
- Model: none is named in the files I read. It is plain prompt text.
- Time: the README says "30 minutes to a few hours" for the explore and spec steps of a large feature. This is the
  author's statement. I found no measurement of it.
- A user who answers. The skill is interactive by design. It does not run unattended.
- Evidence of effect: the author's post on X and the README are the only evidence I read. There is no evaluation of the
  skill in the files I read.

**Sibling skills.** `SKILL.md` links to `write-spec`, and the README chains `/explore-unknowns` → `/write-spec` →
`/implement-spec`. The README also lists `auto-research` ("Reduce cost per task by at least 15% relative to the saved
baseline, without reducing task success"). It is closer to M1 than `explore-unknowns` is. I did not read its files.

## 2. How it can help PVC, and where it does not fit

**What PVC does now.** Owner decision 6: one focused agent, a written plan that the owner approves before code, a
commit after each step (`CLAUDE.md`). `docs/session3/PLAN.md` is the model: goal, measured start, rules, steps with
"done when" lines, a risk table, and later phases. The method matches the skill's stage 5 rule (map first, build
second). The difference: PLAN.md has no step that lists open questions before the plan is written.

**Unknowns that this method would have surfaced, or would surface now.** Each item cites a file I read. Items marked
(inference) are my reading, not a statement in the file.

| Quadrant | Unknown in PVC | Evidence |
|---|---|---|
| Unknown unknown (it was found late, by a run) | The plan assumed the T3 counter could be drawn. The runtime refuses arithmetic. | `docs/session3/NOTES.md` §2: "the plan said the counter could be drawn. That was wrong". The refusal list is in `CLAUDE.md`, "What the runtime can draw today". A sweep of `crates/pvc-runtime/src/support.rs` in stage 4 targets exactly this. |
| Unknown unknown (it was found late, by a run) | The test was unfair: PVC was told to validate and compile, the control was told nothing. | `NOTES.md` §2; `PLAN.md` step 1 |
| Unknown known (inference) | Session 2 aimed at "run on old machines". The owner's intent was different: the developer chooses devices. A stage 3 question ("where does it run, who consumes this") asks this early. | `CLAUDE.md`, owner decision 1; `HANDOFF.md` "owner clarification" |
| Known unknown | How M1 treats tests the runtime cannot draw. `tests.json` marks T3, T5 and T6 with `pvc_renders: false`. M1 requires "pass rate not lower". I found no use of this field in `run.py` or in `mods/pvc-test`. | `pilot/owner-test/tests.json`; `PLAN.md` §1 |
| Known unknown | Does the skill listing and the loaded core card (step 4) count as language tokens for PVC only, or also for the control? The base is measured from an empty session. A skill installed for PVC only changes the PVC base. (inference: I did not test this.) | `PLAN.md` §1 and step 4 |
| Known unknown | The order of phases 2, 3 and 4 after `PLAN.md`. | `PLAN.md` §7: "Phase order after this plan is the owner's decision." |
| Known unknown | Pilot gate items still open: the `PENDING` markers in `PREREG.md` (`HANDOFF.md` says 8; `grep -c PENDING` counts 9 lines), cloud arms not named, no hosted-API adapter, `pvc-probe` not started, an API key needed. | `HANDOFF.md` B1, B4, B4-rt, B5; `NOTES.md` §4; `pilot/PREREG.md` |
| Known unknown | Four held-back confidence items: token thesis, text quality (A8 bar not passed), mirror cost, frame time with text and accessibility. A6 is "not run". | analysis §8.1; `HANDOFF.md` A6, A8 |
| Unknown unknown (inference) | Release-blocking accessibility work that has no budget: no shipped AccessKit web adapter (G7 S.1), and the runtime exposes no text or value for inputs. | analysis §8.1; `HANDOFF.md` A4 |

**Where it helps most.** It forces three things that PVC sessions did not do in a fixed way: (a) a sweep of the
code that the plan touches before the plan is final; (b) every open item recorded with "what unblocks it", not left in
a chat log; (c) a rule that the agent shows the user every closed question. This supports owner decisions 2–6
and the rule "do not reopen owner decisions". A walk would treat the `CLAUDE.md` decisions as settled ground (stage 1).

**Where it does not fit, or can cost tokens.**
1. **It does not move M1 by itself.** M1 counts the tokens of app-building sessions (`PLAN.md` §1). A planning walk
   spends development tokens only. It helps M1 only if it prevents a wrong step (for example the T3 mistake).
2. **Planning cost.** Stage 1 and 4 read many files, with parallel subagents. The skill text alone is 17,039 bytes
   (about 4,300 tokens, estimate: bytes ÷ 4). A full walk on this repository would read the large `docs/` files. My
   estimate is 50,000 to 300,000 tokens per walk. This is a guess, not a measurement. Measure it on the first run.
3. **Poor fit for measurement-driven steps.** `PLAN.md` steps 2, 5 and 6 depend on data from runs, not on opinions.
   An interview cannot replace the step-2 baseline. The map would be mostly "OPEN until measured".
4. **Stage 3 is built for visual design.** Its techniques (design directions, clickable mocks) target UI taste. The
   PVC runtime draws four node kinds today (`CLAUDE.md`), so a rendered option is limited to those.
5. **Delegated answers.** The preference checkpoint lets the agent answer for the user. For PVC this must stay
   off for owner decisions: the phase order, the M1 bound and the pre-registered bounds are the owner's.
6. **Style conflict.** The skill text uses metaphors ("fog", "landmines", "walk", "barrel into"). `CLAUDE.md` bans
   metaphors in project text. Any copy in this repository needs a rewrite in ASD-STE100.
7. **Output format.** The map is an HTML page. PVC records live in Markdown under `docs/`. A PVC use should write
   Markdown.
8. **Install cost.** `npx skills add dzhng/skills` installs the whole library (about 30 skills, per the folder
   listing). Each installed skill adds its description to the session's system prompt. For one skill, use the
   documented `--only explore-unknowns` option of `npm run install-skills`, or copy the single folder.

## 3. Options to move PVC forward

All token figures are estimates. Time figures for agent work are estimates.

### Option A. Run the skill once, unchanged, on the next phase

- **What to do.** Install only this skill (copy `skills/engineering/explore-unknowns/` to `.claude/skills/` in a
  working copy, or to the home folder). Run it when the owner picks the next phase after `PLAN.md`. The best
  candidate is phase 2 (runtime gaps), because the validator accepts more than the runtime draws and three of six
  owner tests do not render. Run it after `PLAN.md` step 2 gives a baseline, so the map has numbers. Turn off the
  preference checkpoint. Write the map to `docs/` as Markdown.
- **Cost.** Owner time: about one to two hours (the README says 30 minutes to a few hours; author's figure). Tokens:
  estimate 50,000–300,000. Measure and record them in the map file.
- **Owner decides.** (1) Which phase to try it on. (2) Whether the agent may use parallel subagents for the stage 1 scan (this raises the token cost).
  (3) Whether the skill may be installed on the working machine. (4) Whether the map is HTML, as upstream, or Markdown.
- **Recommendation.** Yes, once, at the next phase boundary. Judge it by one question: did the map record an unknown
  that the old method would have found only by a run?

### Option B. Adapt it into a PVC planning skill in this repository

- **What to do.** Write `pvc-plan` (a project skill in `.claude/skills/`, or a second plugin entry in
  `.claude-plugin/marketplace.json` beside `pvc-test`). Keep the five-stage order, the "closed in front of the user"
  rule, the "decision plus reason" format and OPEN items with "what unblocks them". Add PVC's fixed inputs: the six
  owner decisions, the M1 definition, "label every number measured or estimate", "do not re-tune a pre-registered
  bound". Remove: HTML mock techniques, the preference checkpoint, `write-spec` and `implement-spec` links. Output a
  Markdown file shaped like `PLAN.md`. Write it again in ASD-STE100.
- **License.** The upstream license is MIT, "Copyright (c) 2026 David Zhang". MIT allows copy and change if the
  copyright and permission notice are kept "in all copies or substantial portions". If the adaptation keeps
  substantial text, put the upstream `LICENSE` text in the skill folder and name the source in the skill's header.
  A full rewrite from the method (the idea, not the text) needs no notice, but an attribution line is good practice.
  This repository's own `LICENSE` is also MIT (holder: "Project VectorCanvas contributors"). The two do not conflict.
  This is not legal advice.
- **Cost.** Writing: estimate 20,000–40,000 agent tokens and one commit. Then it needs one real run to test (that
  is option A again). Maintenance: one more file set to keep current.
- **Owner decides.** The location (project skill or plugin). Whether PVC contributors get the skill at install.
  Name. Whether the skill stays separate from the step-4 skill that carries the PVC core card (two skills named
  alike would confuse agents).
- **Recommendation.** Not now. Do it after option A, and only if the run helped. The run shows which stages to
  keep. This avoids writing a skill that nobody uses.

### Option C. Use the method without the skill: a written unknowns ledger per phase

- **What to do.** For each new phase, add one section to its plan file with four lists (settled, open questions,
  tacit context, findings from a sweep of the touched code). Each open item has an owner, a decision-plus-reason line
  when closed, and "what unblocks it" when not. The first ledger is the table in section 2 of this file, moved
  into `docs/session3/` and extended by the owner. The `CLAUDE.md` rule "written plan approved before code" can then
  name the ledger as a required part of the plan. That change to `CLAUDE.md` is the owner's.
- **License.** The method is an idea. No copying is needed.
- **Cost.** Estimate 10,000–30,000 tokens to write the first ledger. No install. No change to the M1 sessions.
- **Owner decides.** Whether the ledger becomes a required part of every plan, and whether to edit `CLAUDE.md`.
- **Recommendation.** Yes, now. It is the cheapest option and it matches owner decision 6.

### Option D. Do not adopt it now

- **What to do.** Keep the reference note. Start `PLAN.md` step 1.
- **Cost.** None. The risk: the next phase decisions (order of phases, rule for tests the runtime cannot draw) are
  made inside a chat, as the T3 assumption was.
- **Recommendation.** Acceptable only for steps 1–3 of `PLAN.md`. Not for the phase after it.

### Order that I recommend

1. Now: option C, together with `PLAN.md` steps 1–3 (no installs, no new cost to M1).
2. At the next phase boundary: option A, once, with the checkpoint off and the token cost recorded.
3. After that run: option B only if the map found something the old method would have missed.

## Open points that I could not settle

- Whether Claude Code counts an installed skill's description in the "base prompt" that M1 subtracts. Check this with
  an empty session before step 4 of `PLAN.md`.
- The real token cost of one walk on this repository. Option A measures it.
- Whether `auto-research` (same repository) suits `PLAN.md` steps 5 and 6 better than `explore-unknowns` suits any
  step. I did not read it. Read it before the owner chooses.
