# FROZEN — PVC agent context (B2)

Frozen 2026-10-09 by dev-ir; **re-frozen 2026-10-09 (v2)** after the Orchestrator found specimen strings in `01_std_reference.md` (`avatar "JD" … alt="Jane"`, L01's own strings). Changed strings only: avatar `"AK"`/`alt="Ana"`, `link "Help"` (was "Docs"), `radio "Small"` (was "Yes"), example 3 `button "Queue it"` (was "Add") and `name="Drop song"` (was "Remove"), the last three being G02/G04/G05/L04/L09 button or link labels. **Any change to a sent file needs a new hash recorded here and the Supervisor's knowledge.** Not tuned to the pilot intents (README.md in this directory).

Previous (v1, superseded, do not use): system message sha256 `ae8de918d416a20d91ce5adc850412b3da8eb712936c53f056742e4f30bbf6d7`, 2509 tokens.

## Sent files (system message = the files below joined with `"\n\n"`, exactly as `dev-pilot/harness/pilot.py::context_text`)

| # | file (absolute path) | bytes | sha256 | o200k_base tokens (file alone) |
|---|---|---|---|---|
| 1 | `/workspace/dev_work/dev-ir/agent-context/00_card.md` | 1584 | `79d42faecbfe747f02dd3387fd647aa0c7e97be51ac054438e6198248c18f927` | 415 |
| 2 | `/workspace/dev_work/dev-ir/agent-context/01_std_reference.md` | 4405 | `d5a15160ece839227295ca0c6ba32d49755eb06009efaa2539faecc66f2e7a87` | 1324 |
| 3 | `/workspace/dev_work/dev-ir/agent-context/02_dialect_en.md` | 2361 | `a869c398f228ffae794b2001f3b85a32f35fb9b4532075af8915e7013f086315` | 771 |

**Exact message sent as the system prompt:** 8354 bytes, sha256 `d30503040f448a0f9921a93c404c32469c901da7430942838bde80ea685320d9`, **2511 tokens** (`o200k_base`, tiktoken 0.14.0, counted on the joined text; chat-template special tokens excluded). Fixed overhead for PVC release `ir 0.1 / std 0.1 / en dialect 0.1`.

Reproduce: `python3 /workspace/dev_work/dev-ir/agent-context/count_context.py`.

## Not sent
| file | sha256 |
|---|---|
| `README.md` | `ca05669f23b7a01524c6b9a28a2de7f209f8a1e7f1abffa89ca893a3d6b6c174` |

## Companion artefacts
- Validator CLI: `/workspace/cargo-target/debug/pvc-validate` built from `/workspace/dev_work/dev-ir/pvc-ir` (`CARGO_TARGET_DIR=/workspace/cargo-target cargo build`); output format in `README.md`. Accepted fence tag: `pvc`.
- Language grammar: `/workspace/dev_work/dev-ir/GRAMMAR.md`, `PILOT_COVERAGE.md` (not sent).
