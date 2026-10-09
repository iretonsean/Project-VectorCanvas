# Brief: dev-ir — PVC-IR crate, validator, specimens, signing (A1); then the pilot validator and agent card (B2, B4-validator)

**Your name / directory:** `dev-ir` → `/workspace/dev_work/dev-ir/`
**Read first:** `/workspace/orchestrator_work/pm/briefs/COMMON.md` (mandatory), then `docs/G4_PVC_IR_v0_Spec.md` (all of it, especially §2 P1–P9, §3, §4, §5.1–5.3, §6, §7, §8, §9, §10, §14 D1, §15), `docs/G2_Benchmark_Corpus.md` §2, §3, §5 (specimens L01, F01, L01–L10, G01–G05) and `docs/G6_Text_Strategy.md` §3 (fonts in the manifest).
**Serves:** Q4, because the runtime must load *real* signed IR. It also enables Q1, because the pilot's PVC arm needs the validator and agent card.

## Part 1 — A1 (do this first; others are waiting on your checkpoint)

Build crate `pvc_ir` at `/workspace/dev_work/dev-ir/pvc-ir/`.

1. **Concrete terse grammar** for the L01 + F01 subset, written down in `/workspace/dev_work/dev-ir/GRAMMAR.md`. Start from the G4 §15 illustrative syntax, which keeps the token story comparable, and make it precise.
   - Kinds: `surface`, `box`, `text`, `text-input` (variants single-line, email, password).
   - `std` components needed by L01/F01: card, avatar, button, text field with label/help/error, checkbox, password field with show/hide toggle, form.
   - Layout sugar: `row`, `col`, `pad`, `gap`, `align`, `w=fill`, fixed `w`/`h`.
   - Paint: rounded-rect, circle, solid fill, 1-px border, theme tokens.
   - Semantics: role, name, states as G4 §8 requires for these.
   - Behaviour per G4 §7, only what F01 needs: typed state (`bool`, `text`, `enum`), events `change`/`press`/`submit`, guards, assignments, and validity predicates (non-empty; email shape — define the rule exactly in GRAMMAR.md).
   - One effect (`connect_user`), declared with a `net` capability in the manifest. **It is never dispatched over a network.** The runtime routes effects to a dispatcher stub that only logs.
   - Strings table: inline strings in terse form move to a per-locale table in canonical form.
2. **Canonical form:** one byte serialisation (fixed field order, normalised numbers, NFC strings), as G4 §3 requires. `canonicalize(&Module) -> Vec<u8>`.
3. **Signing:** Ed25519 over the canonical bytes, using `ed25519-dalek` + `sha2`. These are pre-approved; write the RFC notes anyway. `load_signed(bytes, &VerifyingKey) -> Result<Module, LoadError>` refuses a bad signature. Use a fixed test keypair committed as a *test* key (label it so).
4. **Validator subset** with G4 §9 diagnostic shapes (code, severity, location, message key + params, fixes): `PVC-E-PARSE`, `PVC-E-SCHEMA`, `PVC-E-UNKNOWN-KIND`, `PVC-E-REF`, `PVC-E-A11Y-NAME`, `PVC-E-CAP-UNDECLARED`, `PVC-E-STRING-MISSING`. Messages are keys, not English prose (G4 §9.1). The validator makes **no network calls**.
5. **Specimens:**
   - `/workspace/dev_work/dev-ir/specimens/L01.pvc`: the README/G2 card with the exact strings, including the manifest entry that G4 §15 deliberately left out.
   - `/workspace/dev_work/dev-ir/specimens/F01.pvc`: the G2 F01 sign-up form, with an **en and a zh-CN** strings table.
   - Their signed canonical artifacts.
   - Each must satisfy its G2 checklist *by construction*, so write down how.
6. **Tests:**
   - D1: the same input gives byte-identical canonical output across runs. Also native vs `wasm32-unknown-unknown`: build a tiny wasm test or compare a `cargo run` output with a wasm build run under node, and record the method.
   - A tampered manifest (add `mic` after signing, G2 X24) is refused at load.
   - Each diagnostic code fires on a minimal bad input.

**Checkpoint (publish early):** as soon as the AST types and the two specimen files are stable, write `/workspace/dev_work/dev-ir/CHECKPOINT.md` with the public API (`parse_terse`, `canonicalize`, `sign`, `load_signed`, the `Module` AST and how to walk it, and the test key path). dev-render, dev-native and dev-web code against it.

**Done means (A1):**
- `cargo test` is green, with the output saved to a file.
- The D1 and tamper tests exist and pass.
- `GRAMMAR.md` is complete for the subset.
- Signed L01/F01 artifacts exist.
- `cargo tree -e normal` for native and wasm32 is saved, and shows no HTTP/LLM/net crate.
- The RFC notes exist.
- An A1 section is in `REPORT.md`.

**Then tell the Supervisor "A1 done" before starting Part 2.**

## Part 2 — after A1: pilot validator (headless) and the PVC agent context (B2)

Supervisor decision D-2 allows the validator to cover the node kinds and constructs the **pilot specimens L01–L10 and G01–G05** need (G2 §5), with no rendering. Extend `pvc_ir`, in your own files only, with the parser, validator and canonical form for exactly what those 15 specimens need. *Likely* (decide and list it): `grid` with `fr` tracks, breakpoint-keyed constraint sets, `wrap`, `scroll`, `view` over a bounded list, `slot`, `layer` (dialog), landmark roles, heading levels, decorative icons (`shape`/icon), links, `list<record>` with max length, list ops (`append`, `remove-where`, `update-where`), `decimal`, `enum`, `timer` if any specimen needs it, the validator a11y rules relevant to U1–U3, and `PVC-E-HOVER-ONLY`.
- **Do not add kinds no pilot specimen needs.** For each construct, record which specimen needs it, in `PILOT_COVERAGE.md`.
- Layout and behaviour *execution* for the checker is **not yours**. dev-render owns the runtime layout and behaviour. You own syntax, validation and canonical form. Coordinate the AST through your CHECKPOINT.
- Raw constraints stay out (D-4). Unsupported constructs yield `PVC-E-SCHEMA` or `PVC-E-UNKNOWN-KIND` with a fix.

**B2 — agent context:** write the PVC agent card, `std` reference and en dialect pack that the pilot gives the model as context: `/workspace/dev_work/dev-ir/agent-context/`.
- It must be enough for a general model to write valid terse PVC-IR for any G2 category 1/4 specimen.
- **It must not be tuned to the specific pilot intents.** No worked example may be one of L01–L10 or G01–G05. Use examples outside the corpus.
- Keep it compact, because its size *is* the fixed overhead the pilot charges against PVC. Report its `o200k_base` token count (tiktoken 0.14.0), counted on the exact bytes sent.
- When done, write `FROZEN.md` with the sha256 of each file and the token count, and tell the Supervisor "B2 frozen". After that, any change requires a new hash and the Supervisor's knowledge.
- Expose the validator to the pilot harness as a CLI: `pvc-validate <file.pvc>`. It prints diagnostics as a compact, stable text format, which the harness feeds back to the model as tool output and counts as tokens. Document the format in `agent-context/`.

**Done means (Part 2):**
- `PILOT_COVERAGE.md` exists.
- Hand-written valid terse IR for each of the 15 pilot specimens passes the validator. Keep it in `pilot-refs/`. It is **reference only, never shown to the agent**, and it proves the language can express them.
- Each new rule has a failing-input test.
- The agent context is frozen, with its hash and token count.
- The CLI works.
- `REPORT.md` is updated.

## Files you may touch
Only `/workspace/dev_work/dev-ir/**`.
