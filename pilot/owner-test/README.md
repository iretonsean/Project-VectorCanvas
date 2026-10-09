# Owner A/B test

This test compares two Claude agents that build the same small app:

- the **PVC agent** writes the app in PVC-IR, with the validator;
- the **control agent** picks any language it wants and says which one and why.

A third agent, the **evaluator**, marks each result against a fixed checklist. It sees neither the token counts nor which result is PVC (the results are labelled A and B at random).

Token counts come from Claude Code's own usage report for each agent session, not from the agents.

## Run it from Claude Code (pvc-test mod)

1. Clone this repository and start Claude Code in its folder.
2. Install the mod once:
   ```
   /plugin install pvc-test --marketplace iretonsean/Project-VectorCanvas
   ```
   Answer `y` to add the marketplace, then choose a scope.
3. Run the test:
   ```
   /pvc-test              all six tests
   /pvc-test T1 T3        only these tests
   /pvc-test --model opus all tests with one model
   /pvc-test T2 --effort high   set the effort level of every agent
   /pvc-test results      open the results pane again
   ```
The PVC test pane shows progress and results.

## Where the results go

Every test run is written to [`tests-run/`](../../tests-run/) at the repository root:

- `tests-run/<UTC time>_<test id>_<name>.md`: one file per test, with the verdict, scope, agents, models and effort, tokens and cost side by side, code size, the checklist item by item, screenshots, fix attempts, each agent's final message, errors, all code produced, the exact prompts, versions and machine, the repo commit, SHA-256 hashes of the inputs, and the blind labels.
- `tests-run/<same name>/`: the screenshots, every file each agent left, and the raw Claude Code transcripts.
- `tests-run/INDEX.md`: one row per test run.

Commit `tests-run/` to keep the results in the repository.

## Run it without the mod

```sh
python3 pilot/owner-test/run.py              # all tests
python3 pilot/owner-test/run.py T1 T2 --model sonnet --effort medium
```

## Requirements

- Claude Code (the `claude` command), signed in.
- Python 3.9 or later.
- A Rust toolchain (`cargo`, 1.89 or later). The script builds the PVC validator, compiler and screenshot tool first.
- Optional: Chrome, Chromium or Edge, for screenshots of the control app. Without one, the evaluator judges the control app from its code.
- Optional: `tiktoken` (`pip install tiktoken`), to count the final code size in tokens.

## The tests

| ID | Test | Level | PVC app drawn? |
|---|---|---|---|
| T1 | Hello + Save button | 1 static | yes |
| T2 | Profile card | 1 static | yes |
| T3 | Counter | 2 state | yes |
| T4 | Sign-up form | 2 state | yes |
| T5 | Wizard with skip | 3 logic | no: the runtime cannot draw this yet; judged from code |
| T6 | Dashboard shell | 4 layout | no: the runtime cannot draw this yet; judged from code |

Descriptions and checklists are in `tests.json`. Change them only before a run, never after you see results.

## What each run does

For each test:
1. The PVC agent runs in a new temporary folder that holds only the three PVC context files, `pvc-validate` and `pvc-compile`.
2. The control agent runs in a new, empty temporary folder.
3. The script makes a screenshot of each app where it can.
4. The evaluator runs in a third folder with the description, the checklist and both results.

Every agent runs headless (`claude -p`) with web tools off, user settings and MCP servers off where the Claude Code version supports it, and a new session ID.

## Cost and time

Each test runs three agent sessions. Expect a few minutes per test. The cost depends on the model. Use `--budget <usd>` to cap each agent session.

## Limits

- One run per test is a first look, not proof.
- The evaluator is a language model. It reads code and screenshots; it is not the automated checker the G2 pilot specifies.
- Token totals include Claude Code's own system prompt, the same for both agents.
