# Project VectorCanvas (PVC): notes for agents

Read this file, then `HANDOFF.md`, before you change anything.

## What PVC is

PVC is a language for AI agents to write apps with fewer tokens. An agent (cloud or local) writes PVC-IR; the validator checks it; a small runtime draws it with no browser engine or UI toolkit. No AI model runs inside a finished app.

## Owner decisions (do not reopen them without the owner)

1. **The developer chooses the devices an app runs on. PVC does not.** "Virtually any machine" means the machine the developer *writes* on: any machine that can reach a local or cloud AI model. Hardware tiers are targets a developer can select, not language requirements or release gates (analysis §4.4). Do not plan work that makes PVC "run on old machines" as a goal of the language. An earlier multi-agent session drifted into this. It was a mistake.
2. Authoring works with a cloud agent or a local model. Neither is mandatory.
3. The G2 token pilot runs with cloud models only. Local-model runs are community contributions (`CONTRIBUTING.md`).
4. A6 measurements run on the container only. Real-device results are community contributions.
5. Accessibility is release-blocking on every target the developer selects.
6. Work method: one focused agent, a written plan approved by the owner before code, a commit after each step.

## Current work

`docs/session3/PLAN.md`: cut PVC's fixed token cost per session. It is approved; read its status line to see where work stands. Later phases (runtime gaps, developer toolchain, language reference) are listed at its end. Their order is the owner's decision.

## Where things are

| Path | What |
|---|---|
| `crates/pvc-ir/` | Grammar, validator (`pvc-validate`), compiler (`pvc-compile`), signing |
| `crates/pvc-runtime/` | Runtime and renderers; `examples/pvc_shot.rs` renders an app to PNG |
| `crates/specimens/` | Reference apps L01 (profile card) and F01 (sign-up form) |
| `pilot/agent-context/` | The context an agent gets to learn PVC (frozen v2, 2,511 o200k tokens) |
| `pilot/owner-test/` | The owner A/B test (PVC agent vs control agent, evaluator agent) |
| `mods/pvc-test/` | Claude Code mod: `/pvc-test` runs the A/B test |
| `tests-run/` | One result file per A/B test run; `INDEX.md` lists them |
| `docs/` | Vision analysis and G2–G7 documents; `docs/session2/` and `docs/session3/` session records |

## Build and test

```sh
cargo test --release -p pvc-ir -p pvc-runtime --features pvc-runtime/cpu,pvc-runtime/test-unsigned
claude plugin validate mods/pvc-test && claude plugin test mods/pvc-test
python3 pilot/owner-test/run.py T1 --model haiku    # one cheap end-to-end check of the A/B test
```

No GPU is needed. Expect 89 Rust tests to pass.

## What the runtime can draw today

Only the node kinds surface, box, text and text input. It refuses arithmetic and decimal expressions, lists (`view`, `append`, `remove`), grid layout, screen-size rules, shapes, layers, slots and link actions (`crates/pvc-runtime/src/support.rs`). The validator accepts more than the runtime draws.

## Rules for every change

- Numbers: label each one *measured* (with the file that holds it) or *estimate*. Never invent a statistic.
- Do not re-tune a pre-registered bound after you see a result (`measurements/session2/golden/TOLERANCE.md`, `pilot/PREREG.md`).
- A token saving that lowers the checklist pass rate does not count.
- The frozen agent context changes only with a new version, a new hash and a commit.
- Writing style for every project text: ASD-STE100 Simplified Technical English, main point first (Minto pyramid), literal words, no metaphors. Swearing never appears in project text.

## Known traps

See `docs/session3/NOTES.md` § Tooling traps (headless `claude -p` session IDs, Chrome as root, which usage field to trust, and more).
