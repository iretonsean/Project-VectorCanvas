# Brief: dev-pilot — G2 pilot pre-registration, harness, and (later) run + analysis (B1, B3, B5)

**Your name / directory:** `dev-pilot` → `/workspace/dev_work/dev-pilot/`
**Read first:** `/workspace/orchestrator_work/pm/briefs/COMMON.md` (mandatory, especially §6, the decisions in force). Then `docs/G2_Benchmark_Corpus.md` **in full**: §1 is the decision rule and falsification trigger, which you quote verbatim; §2 defines T_total; §3, §4 and §7 matter too. Then analysis §2.6 and §8.1, `docs/G4_PVC_IR_v0_Spec.md` §2.1, and FINDINGS F3, F4 and F16–F19.
**Serves:** Q1, whether PVC's per-session context overhead eats the token savings.
**Phase now: prep only (D-6). No pilot runs until the Orchestrator says "go" through the Supervisor.** A dry run on one specimen to test the harness is allowed (see B3). Its results are discarded and labelled.

## B1 — `PREREG.md` (must exist, be seen by the Supervisor and be committed before any real run)
Write `/workspace/dev_work/dev-pilot/PREREG.md` containing:
- **Specimens and run order:** L01–L10 and G01–G05 (D-3). Fix an **interleaved run order** with a deterministic rule, stated. If compute binds, the analysed set is the completed prefix of the order, never less than L01–L05 + G01–G05.
- **Arms (D-1):**
  - S4 = Qwen3-4B Q4_K_M (`/workspace/tools/models/Qwen3-4B-Q4_K_M.gguf`, sha256 7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5), thinking **off** (`chat_template_kwargs: {enable_thinking:false}`).
  - S7 = Qwen2.5-Coder-7B-Instruct Q4_K_M (`/workspace/tools/models/qwen2.5-coder-7b-instruct-q4_k_m.gguf`, sha256 509287f78cb4d4cf6b3843734733b914b2c158e43e22a7f4bf5e963800894d3c).
  - Both run on llama.cpp b11514 (`/workspace/tools/llama-b11514/llama-server`), 4 threads, temperature 0, fixed seed, fixed context size, and fixed max output tokens per turn (state it).
  - Mandatory label: "both arms small local models; no hosted/frontier arm run; result speaks only for that class."
- **Stacks:** PVC; B1 (HTML + Tailwind + vanilla JS, one file); B2 (React + TS + Tailwind + one component library). Both baselines run on every specimen (G2 §3.3). Reported baseline = the lower passing T_total of B1/B2 per specimen.
- **Exact T_total accounting** per G2 §2 items 1–5, counted with `o200k_base` (tiktoken 0.14.0) on the exact message text.
  - **Resolve explicitly and flag to the Supervisor:** does re-sent conversation history (prior agent outputs, prior tool and repair messages) count again on later turns, or only the items G2 §2 lists? G2 says the context overhead is "counted on every turn in which it is sent". It is silent on history. Propose a primary definition, report the other as a secondary variant, and get the Supervisor's ruling before the first real run.
  - Also state how chat-template special tokens are handled (excluded from o200k counts, stated).
  - Also give the **cached-at-zero** variant.
- **Secondary count:** llama-server `usage` (the model's own tokenizer), in place of vendor billing (PLAN V10). If the direction disagrees with the primary count for a category, that category is reported as **inconclusive** (G2 §2).
- **Turn cap 6.** Failures are excluded from the medians and counted in the pass rate (G2 §2). How unpaired failures are handled in the paired difference must be stated before the run.
- **Repair feedback:** the standardised failing-checklist-item text, identical across stacks (G2 §4.4). The templates appear verbatim in PREREG.
- **Tools (G2 §4.3):** PVC gets `pvc-validate` (dev-ir). Baselines get html-validate, `tsc` (B2) and axe-core (dev-baseline). Say how tool output is fed back (harness-mediated after each agent turn) and that it is counted as tool tokens.
- **Fidelity:** dev-baseline's checkers. G2 A-items + U1–U5. H items "not judged" (D-5). An LLM is never the judge.
- **CI:** paired percentile bootstrap over specimens, 10 000 resamples, fixed seed, per category per arm. Note that temperature 0 means the CI reflects specimen variability, not sampling variability.
- **Decision rule and falsification trigger quoted verbatim from G2 §1.** Applied to each arm individually (D-7), and as written even if underpowered (D-8). State the exact sentence that will appear if the trigger fires, with n and arms beside it.
- **Deviation list:** PLAN §5 V7–V10 plus anything you add (e.g. intents are PM-drafted, G2 §7.3).
- **PVC agent context:** reference dev-ir's frozen `FROZEN.md` (hash and token count) once it exists. Until then, leave a placeholder that blocks the run.

When the draft is complete, tell the Supervisor "PREREG ready: <path> sha256 <hash>". The Orchestrator commits it to the branch.

## B3 — harness (`/workspace/dev_work/dev-pilot/harness/`)
- Starts and stops `llama-server` per arm.
- Builds messages from: the intent text (G2 §5, exact), the stack instructions, and the context (the PVC agent context, or the baseline instructions).
- After each agent turn: extract the code/IR, run the stack's tools, run the fidelity check, and feed back the tool output and the standard repair text. Stop at pass or at turn 6.
- **Saves every turn verbatim** to JSONL: role, exact text, o200k count, server `usage`, `timings`, timestamps, model sha256, harness git hash or file sha256.
- `recount.py <transcript.jsonl>` recomputes every count from the saved text with `o200k_base` (the Supervisor will run this) and prints per-item totals matching your accounting.
- Every model run is wrapped in `flock -s /workspace/orchestrator_work/pm/CPU_LOCK` (shared; measurements take it exclusively).
- **Dry run:** one specimen, one arm, one stack, to prove the plumbing works. Label it `DRY-RUN — not data` and keep it out of the results directory.

## B5 — later, only after "go"
Run, then analyse into `/workspace/dev_work/dev-pilot/measurements/g2-pilot/`:
- transcripts;
- per-run CSV;
- `RESULT.md`: T_total per category, per arm, per stack; medians, p90, pass@1, pass@6, output-only tokens, cached variant, the per-category range with 95% CI, the decision rule outcome per arm, and the falsification statement.

Publish whichever way it goes.

## Done means (now)
- PREREG drafted and announced.
- Harness plus `recount.py` working, with the dry run proving it.
- `REPORT.md` covers B1 and B3, with checked and inferred kept separate.

## Files you may touch
Only `/workspace/dev_work/dev-pilot/**`. Use your own venv for numpy/scipy if needed.
