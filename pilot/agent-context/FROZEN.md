# FROZEN — PVC agent context (B2)

Frozen 2026-10-09 by dev-ir. **Any change to a sent file needs a new hash recorded here and the Supervisor's knowledge.** Not tuned to the pilot intents (README.md in this directory).

## Sent files (system message = the files below joined with `"\n\n"`, exactly as `dev-pilot/harness/pilot.py::context_text`)

| # | file (absolute path) | bytes | sha256 | o200k_base tokens (file alone) |
|---|---|---|---|---|
| 1 | `/workspace/dev_work/dev-ir/agent-context/00_card.md` | 1584 | `79d42faecbfe747f02dd3387fd647aa0c7e97be51ac054438e6198248c18f927` | 415 |
| 2 | `/workspace/dev_work/dev-ir/agent-context/01_std_reference.md` | 4404 | `ce32ef5e56ced1473f24df7d7e209c4b6318b693e7b717b70e7203c2b893c6de` | 1324 |
| 3 | `/workspace/dev_work/dev-ir/agent-context/02_dialect_en.md` | 2353 | `2700ef76c28096d67b4209cd2dec6c8a64496d73cd23de58dfd5fa1a0434b44c` | 769 |

**Exact message sent as the system prompt:** 8345 bytes, sha256 `ae8de918d416a20d91ce5adc850412b3da8eb712936c53f056742e4f30bbf6d7`, **2509 tokens** (`o200k_base`, tiktoken 0.14.0, counted on the joined text; chat-template special tokens excluded). This is the fixed overhead published for PVC release `ir 0.1 / std 0.1 / en dialect 0.1`.

Reproduce: `python3 /workspace/dev_work/dev-ir/agent-context/count_context.py`.

## Not sent
| file | sha256 |
|---|---|
| `README.md` | `ca05669f23b7a01524c6b9a28a2de7f209f8a1e7f1abffa89ca893a3d6b6c174` |

## Companion artefacts (the tool the context describes)
- Validator CLI: `/workspace/cargo-target/debug/pvc-validate` built from `/workspace/dev_work/dev-ir/pvc-ir` (`CARGO_TARGET_DIR=/workspace/cargo-target cargo build`); output format in `README.md`. Accepted fence tag: `pvc`.
- Language grammar: `/workspace/dev_work/dev-ir/GRAMMAR.md` and `PILOT_COVERAGE.md` (not sent).
