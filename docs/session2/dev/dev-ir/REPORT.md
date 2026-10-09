# dev-ir REPORT

Owner: dev-ir. Directory: `/workspace/dev_work/dev-ir/`. Repo docs read at main @ 5fdd412. Host for everything below: `host=container-xeon4` (no timing numbers are reported in this section, so no path label applies).

# A1 — `pvc_ir` crate, grammar, validator subset, signing, specimens

## 1. Summary
* `pvc_ir` builds and passes **45 tests** (`evidence/cargo-test-A1.txt`; 0 failures) natively and builds for `wasm32-unknown-unknown`.
* Concrete terse grammar for the L01 + F01 subset: `GRAMMAR.md`. One canonical byte form (compact JSON, fixed field order, integer-only, NFC), Ed25519 sign / verify-on-load, hand-rolled (no serde).
* Validator subset with the seven required codes plus five safety codes; diagnostics are keys + params + structured fixes; compact one-line text format via `pvc-validate`.
* Signed specimens: `specimens/L01.pvca` (7 810 canonical bytes), `specimens/F01.pvca` (18 682 canonical bytes, en + zh-CN). Both satisfy their G2 checklists by construction (`specimens/CHECKLISTS.md`, asserted on the loaded signed artifacts).
* D1: byte-identical canonical output and signed artifacts across runs **and** native vs wasm32 (node 22): `evidence/d1-wasm.txt`.
* X24 tamper test (add `mic` after signing) is refused with `BadSignature`; the same edit re-signed loads (so the refusal is the signature check).
* Hard wall: `cargo tree -e normal` for native and wasm32 has no HTTP/socket/LLM/async-runtime crate (`evidence/hardwall-scan.txt`).
* One finding for Deliverable C (§7, item 1): the *valid* L01 program is 150 o200k tokens, not the 118 of the G4 §15 snippet.

## 2. Done criteria (A1)
| Criterion (brief) | Status | Evidence |
|---|---|---|
| `cargo test` green, output saved | **met** | `evidence/cargo-test-A1.txt` (45 passed) |
| D1 test: same input → byte-identical canonical across runs | **met** | `pvc-ir/tests/d1_and_tamper.rs::d1_same_input_same_bytes_across_runs`, `committed_artifacts_match_a_fresh_compile`, `whitespace_and_layout_of_terse_input_does_not_change_canonical_bytes` |
| D1 across native vs wasm32; method recorded | **met** | `d1-wasm/` (plain C-ABI cdylib driven from node's WebAssembly API, no wasm-bindgen), `d1-wasm/run_d1.sh`, `evidence/d1-wasm.txt`: canonical sha256 L01 `cf3e7e88…3fd8`, F01 `81b12fdb…cbd7` equal on both targets; signed-artifact sha256 equal too (Ed25519 is deterministic); wasm `load_signed` accepts the native artifact and refuses a one-byte-tampered one |
| Tampered manifest (G2 X24) refused at load, with test | **met** | `tests/d1_and_tamper.rs::x24_tampered_manifest_is_refused` (+ `single_flipped_byte_is_refused`, `wrong_signer_is_refused`, `non_canonical_but_signed_body_is_refused`, `garbage_frames_are_refused`) |
| `GRAMMAR.md` complete for the subset | **met** | `GRAMMAR.md` (+ `tests/grammar.rs` for the statements the specimens do not exercise) |
| Concrete terse grammar: kinds, std components, layout sugar, paint, semantics, behaviour, one `net` effect, strings table | **met** | `GRAMMAR.md` §3–§7; specimens |
| Canonical form `canonicalize(&Module) -> Vec<u8>` | **met** | `src/canon.rs`; round-trip exactness `canonical_roundtrip_is_exact`; NFC `nfc_strings_in_canonical_form` |
| Signing + `load_signed(bytes,&VerifyingKey)` refuses bad signature; committed **test** key labelled | **met** | `src/sign.rs`; key = `sha256("pvc-session2-test-key-DO-NOT-USE-IN-PRODUCTION")` as Ed25519 seed, public key `203dab1b…f3f0`, `sign::testkey::NOTICE`; no secret file (the seed phrase is public by design) |
| Validator: the seven codes with G4 §9 shapes (code, severity, location, key+params, fixes) | **met** | `tests/diagnostics.rs` — one minimal bad input per code (`PARSE`, `SCHEMA`, `UNKNOWN-KIND`, `REF`, `A11Y-NAME`, `CAP-UNDECLARED`, `STRING-MISSING`); messages are keys; validator takes no I/O argument (`hard_wall_validator_has_no_io`) |
| L01.pvc and F01.pvc, signed artifacts; each satisfies its checklist by construction, written down | **met** (structure/state level); layout/pixel level is the runtime's | `specimens/{L01,F01}.pvc`, `.pvca`, `.canonical.json`, `specimens/CHECKLISTS.md`, `tests/specimens.rs` |
| L01 exact strings + the manifest entry G4 §15 omitted | **met** | `specimens/L01.pvc` (`cap net.main …`, `effect connect_user uses net.main`) |
| F01 en **and** zh-CN strings table | **met** | `specimens/F01.pvc`; test checks every key in both tables and CJK content |
| `cargo tree -e normal` native + wasm32 saved, no HTTP/LLM/net crate | **met** | `evidence/cargo-tree-native.txt`, `evidence/cargo-tree-wasm32.txt`, `evidence/hardwall-scan.txt` |
| RFC notes | **met** | `rfc/ed25519-dalek.md`, `rfc/sha2.md`, `rfc/icu_normalizer.md` |
| Checkpoint published early | **met** | `CHECKPOINT.md` |
| A1 section in REPORT.md | **met** | this file |

## 3. Checked vs inferred
**Checked** (I ran it; file given):
* 45 tests pass: `evidence/cargo-test-A1.txt`.
* Native vs wasm32 byte equality of canonical bytes and signed artifacts; wasm32 verify accepts/refuses: `evidence/d1-wasm.txt`.
* Dependency trees contain none of `reqwest|hyper|ureq|curl|openai|llm|tokio|socket2|mio|http|native-tls|rustls|…`; source/Cargo.toml contain none of `reqwest|hyper|openai|llm|tokio|std::net|TcpStream|UdpSocket|http(s)://`: `evidence/hardwall-scan.txt`.
* F01 checklist behaviour (submit disabled until valid, email rule, toggle masking, error shown only while invalid) by running the reference evaluator over the **loaded signed** artifact: `tests/specimens.rs`.
* L01/F01 text/background WCAG contrast ≥ 4.5:1 (≥ 3:1 large text) for every painted text node; input border ≥ 3:1 vs page background in F01: `tests/specimens.rs::u3_…` (WCAG formula implemented in `src/theme.rs`, thresholds from general knowledge as in G2 §3.1).
* Token counts (o200k_base, tiktoken 0.14.0): G4 §15 terse card = 118 (matches G4/README), **committed valid `L01.pvc` = 150**, the four explicit font lines = 226, `fonts std` = 3: `evidence/l01-token-count.txt`.
* `fonts std` expands to exactly the entries of the explicit lines (the committed artifacts, built from the explicit form, are byte-equal to a fresh compile from the `fonts std` form).
* AccessKit 0.25.1 `Role` has `Banner`, `Complementary`, `ContentInfo` (`accesskit-0.25.1/src/lib.rs` lines 123/132/136).

**Inferred / not checked:**
* That dev-render's solver implements the layout semantics in `CHECKPOINT.md` identically. I specified them; nothing here executes layout (the checklist rows "button spans card width", "avatar left of name" are checked as **fields of the IR**, not solved geometry).
* U2 (keyboard), U4/U5 (320 px, 200 % text) for L01/F01: they depend on the runtime. Not claimed.
* That a font bundle of the four system files (≈ 40 MB of CJK) is acceptable to dev-render/dev-web; the hashes are of the *system* files on this container.
* The email rule is a deliberately small ASCII subset of RFC 5322 (stated in GRAMMAR.md §6.1); it is not claimed to match any browser's `type=email`.
* wasm32 determinism is checked on one engine (node 22 / V8). It is integer-only code with no floats, which is why I expect parity everywhere, but only this engine was run.
* Ed25519 `verify_strict` semantics are the crate's; no independent test vectors were run beyond sign→verify and tamper refusal.

## 4. Measurements
None in A1 (no timings or RSS). File sizes: canonical L01 7 810 B, F01 18 682 B; signed artifacts 8 025 B and 18 897 B at the time of first build (artifact = canonical + 2 header lines, unchanged since).

## 5. Deviations and labelled gaps
* Canonical layout = explicit sugar record, not a constraint list (D-4). `show=<expr>`, `derive`, `is-email`, fixed-width cap in columns, `fonts std`: additions to G4 listed in `GRAMMAR.md` §10.
* Validator rules **not** implemented in A1 (G4 §8.2): contrast (`A11Y-CONTRAST`), target size, live (`A11Y-LIVE`), reflow, `CONSTRAINT-CONFLICT`, ambiguity warnings, `CAP-IN-STRING`. Out of scope for the brief; U3 is checked in tests for the two specimens only.
* No acknowledgements table, no provenance in the canonical module (provenance is held in memory for diagnostics and never serialised).
* Parser reports only the first syntax error (the one-line diagnostic format is stable); schema/validator errors are all reported.
* Test key: the seed is public and derived from a phrase in source. This is intentional and labelled; it must never sign real artifacts.
* The `Checkbox` mark is a filled square, not a tick glyph (no `shape`/icon kind in the subset).

## 6. Dependencies
Cargo (shipped crate `pvc-ir`; all exact-pinned):
| crate | version | RFC note |
|---|---|---|
| `ed25519-dalek` | `=2.2.0` (pre-approved D-10) | `rfc/ed25519-dalek.md` |
| `sha2` | `=0.10.9` (pre-approved D-10) | `rfc/sha2.md` |
| `icu_normalizer` | `=2.3.0` — **not on D-10; approved by the Supervisor 2026-10-08. Orchestrator: please add it to STATUS.md.** | `rfc/icu_normalizer.md` |

Tooling / test-only (no RFC, listed per COMMON §3): `d1-wasm` test crate (path dep on pvc-ir; no extra crates), node v22.22.0 (already installed), `rustup target wasm32-unknown-unknown` (already installed), `tiktoken` 0.14.0 (system python3, already installed). No apt/pip/npm installs by dev-ir.
Toolchain: cargo/rustc 1.97.0; crate declares `rust-version = "1.89"` (not separately tested on 1.89).

## 7. Inconsistencies flagged
1. **G4 §15 / README token claim vs a valid program.** The 118-token snippet is explicitly "not a valid program by itself" (G4 §15). The *valid* L01 (manifest with the `net` capability, `effect`, `locales`) is **150 tokens** (`evidence/l01-token-count.txt`) with the std default font set, and would be 373 if the agent had to write the four hashed `font` lines. Versus the README HTML's 184 (F4: 184, not 185) that is 18 % fewer, not 36 %. Deliverable C and the README should not carry "118" as the like-for-like number. This is also why `fonts std` exists.
2. **G4 §8.1 omits landmark roles that AccessKit 0.25.1 has** (`Banner`, `Complementary`, `ContentInfo`); G2 L04 ("five regions with landmark roles") and L09 (footer) need them. Per the Orchestrator ruling v0 adds no landmark roles: header/aside/footer are `region` + a name (B2 will say so).
3. **G4 §7.3 does not list an email predicate**, yet G2 F01 requires "email validated (A)"; added `is-email` with its exact rule (GRAMMAR.md §6.1).
4. **G4 §5.2 vs D-4:** G4 says sugar expands into constraints; this build keeps the explicit stack record (labelled).
5. G4 §3 lists "Acknowledgements" and an "optional provenance" part; only provenance exists (in memory).
6. G2 F01 needs conditional presence (error text) but G4 has only `slot` for it, and `slot` is outside the L01/F01 subset the Orchestrator allowed; `show=` bridges that (labelled).

## 8. Open issues
* dev-render must confirm or replace the font file set (hashes). Cheapest path: bundle the four system files named in `src/expand.rs::STD_FONTS`.
* `show`, `disabled` 40 % opacity and "Enter raises `submit` on the nearest ancestor with a handler" are runtime rules stated in `CHECKPOINT.md`; nothing in the IR enforces them.
* Part 2 will change `NodeKind`, `Role`, `Expr`, `Action`, `Ty`, `Value` by **adding** variants (non-breaking for code that matches with a wildcard; code that matches exhaustively will need an arm). Say if you want those enums marked `#[non_exhaustive]` now.

---

# Part 2 — pilot validator (D-2), pilot references, agent context (B2), `pvc-validate`

## P2.1 Summary
* `pvc_ir` now parses, validates and canonicalises everything the 15 pilot specimens L01–L10 + G01–G05 need (grid, breakpoint overrides, lists/`view`, `slot`, `layer`, `decimal`, row actions, links + `open-url` capability, new roles). No rendering; execution stays with dev-render. The A1 signed artifacts are **byte-identical** after the extension (`tests/d1_and_tamper.rs::committed_artifacts_match_a_fresh_compile` still green).
* 15 hand-written reference programs, `pilot-refs/*.pvc`, each validate with **zero diagnostics**, round-trip canonical form, sign/load, and are identical native vs wasm32 (`evidence/d1-wasm.txt`). G01–G05 behaviour and the L-specimens' structural checklist facts are asserted in `tests/pilot_refs.rs` with the reference evaluator (e.g. G04 vs an independent integer-cents model over 49 quantity pairs; G02 20-operation script vs a model).
* `PILOT_COVERAGE.md` maps every construct to the specimens that need it and lists what is deliberately not covered.
* Agent context **frozen**: `agent-context/FROZEN.md`. The system message is **2 511 o200k_base tokens** (8 354 bytes), sha256 `d3050304…85320d9` (re-frozen v2; v1 `ae8de918…`/2 509 superseded). Three worked examples, none from the corpus.
* `pvc-validate <file.pvc>` / `--json` implemented; output format documented in `agent-context/README.md`.
* 81 tests pass (`evidence/cargo-test-part2.txt`).

## P2.2 Done criteria (Part 2)
| Criterion | Status | Evidence |
|---|---|---|
| `PILOT_COVERAGE.md` exists, one entry per construct with the specimen that needs it | **met** | `PILOT_COVERAGE.md` §1–§2 (the matrix was produced by scanning the references: only constructs a reference uses are listed) |
| Hand-written valid terse IR for each of the 15 specimens passes the validator; kept in `pilot-refs/`; never shown to the agent | **met** | `pilot-refs/*.pvc`; `tests/pilot_refs.rs::all_fifteen_refs_validate_clean_and_are_deterministic` (zero diagnostics each, D1, round trip, signed load); not referenced by any agent-context file (`tests/agent_context.rs` also asserts no context example equals a reference) |
| Each new rule has a failing-input test | **met** | `tests/pilot_rules.rs` (17 tests): A11Y-IMG, UNBOUNDED, CONTRAST, CAP-UNDECLARED/CAP-SCOPE for `href`, A11Y-ACTION (link/tab/radio), A11Y-NAME for new roles, text-hole REF/SCHEMA, list/view/alias/field REF, TYPE (decimal, list, filter), slot rules + warning, breakpoint-override errors, layer rules, list-initialiser errors, evaluator rounding cases |
| Agent context frozen with hash and token count; "B2 frozen" announced | **met** (announcement sent with this report) | `agent-context/FROZEN.md`, `agent-context/count_context.py` |
| Agent context enough for category 1/4 specimens, not tuned to the pilot intents, examples outside the corpus | **partly met** — *enough* is checked only in the sense that every construct a reference needs is documented and every documented form compiles (`tests/agent_context.rs`); whether a 4B/7B model can use it is **not tested** (no pilot runs, D-6) | `agent-context/*.md`, `tests/agent_context.rs` |
| CLI works | **met** | `pvc-validate [--json] file` (`pvc-ir/src/bin/pvc-validate.rs`); text format in `agent-context/README.md`; exit 0/1/2; output covered by `tests/diagnostics.rs::diagnostics_are_keys_not_prose_and_render_is_stable` and `tests/agent_context.rs::documented_diagnostic_line_has_the_documented_shape` |
| `cargo tree` still free of HTTP/LLM/net crates; no new Cargo dependency | **met** | `evidence/hardwall-scan.txt`, `evidence/cargo-tree-*.txt` |
| `REPORT.md` updated | **met** | this section |

## P2.3 Checked vs inferred (Part 2)
**Checked** (ran it; file given):
* 81 tests green: `evidence/cargo-test-part2.txt` (A1 45 + Part 2 36).
* 15 refs × {validate clean, D1 twice, canonical round trip, signed load}: `tests/pilot_refs.rs`; native vs wasm32 canonical bytes + signed-artifact bytes identical for all 15 refs and L01/F01, wasm `load_signed` accepts native artifacts and refuses tampered ones: `evidence/d1-wasm.txt` (engine: node v22.22.0 only).
* G01–G05 state-level checklist traces with the reference evaluator over the compiled references: G01 (hint shown/hidden/dismissed/never returns; new session = fresh state; nothing persisted: no capability declared), G02 (20 scripted operations vs a reference model: visible task set and "N items left" after every step), G03 (prompt only when dirty; cancel keeps tab and text; discard switches and drops text), G04 (Subtotal/Discount/Tax/Total at 4 states + 49 quantity pairs vs an independent integer-cents half-up model, initial tax $2.16 for subtotal $26.14; wrong code shows an error containing "Invalid" and changes nothing; quantity floor 1), G05 (skip both ways, Back returns to the true previous step, exactly one "Step N" heading per step).
* IR-level facts behind L02–L10 checklist items (grid column counts and `md` overrides, 4 list items per plan, single h1, region/navigation/main landmarks, label-left/control-right rows with name = label text, equal panel heights, decorative icons, link roles/names, icon-only links named, email field labelled) : `tests/pilot_refs.rs`.
* Every example program and every documented syntax form in the agent context compiles/validates: `tests/agent_context.rs`.
* Agent-context size: 415 + 1 324 + 769 tokens per file, **2 511 for the joined message (v2)**, recount `python3 agent-context/count_context.py`: `agent-context/FROZEN.md`. Hand-written reference programs: 130–472 tokens each, 4 525 in total: `evidence/pilot-refs-token-count.txt` (**not agent output**; lower-bound illustration only).

**Inferred / not checked:**
* **That the language is learnable by ≤8B models from this 2.5 k-token context.** Unmeasured; that is what the pilot measures. A weak spot I expect (inference): newline-terminated node headers, the `a - b` spacing rule, and the mandatory boilerplate (`fonts std`, `on.press={ }` stubs for action-less buttons, `alt=`/`decorative` on every icon) will cost repair turns.
* **Geometry-based checklist items** (L02.1, L03.1, L04.1–3, L05.1, L06.1–2, L07.1/3, L08.1, L09.1, L10.1–2 and U4/U5) are *not* decided here: the references encode the intended structure (grid `cols`, `md.*` overrides, `w=fill` labels, `h=fill` middle band, `w=640` column), but whether they hold at 320/480/768/1024/1280 px depends on dev-render's solver and on Parley text measurement. In particular **L05.1** (45–90 characters per line at desktop) assumes ~70 characters in a 640-px DejaVu Sans 16 px column: not measured by me.
* U2 keyboard operability of the references (focus order, Space/Enter, arrow keys inside `radio-group`/`tab-list`) is a runtime property.
* Contrast: my validator rule and dev-baseline's U3 use the same WCAG formula and the same "nearest filled ancestor" idea but are separate implementations; agreement is not tested.
* Emoji/icon-font text is not used anywhere (icons are `shape`s), so L10's "emoji" option is untested.

## P2.4 Measurements
No timing or memory numbers in Part 2. Token counts are from files I produced: `evidence/l01-token-count.txt`, `evidence/pilot-refs-token-count.txt`, `agent-context/FROZEN.md`. Tokenizer: `o200k_base`, tiktoken 0.14.0, counted on the exact text.

## P2.5 Deviations and labelled gaps (Part 2)
Full list with reasons: `PILOT_COVERAGE.md` §3–§5 and `GRAMMAR.md` §10–§11. The ones that can move pilot results:
1. **Bound-row row actions** (`remove x`, `x.f = …`) instead of G4's `remove-where`/`update-where`; no `sort-by`/`clear`.
2. **No ICU plural/select**; "1 item left / N items left" needs two guarded texts.
3. **Fixed 2-digit `decimal`; one rounding function (`percent`).**
4. **Fixed breakpoints** md ≤ 768, sm ≤ 480.
5. **`link href` requires a declared `open-url` capability** (+ prefix match). This costs PVC one `cap` line on L04/L08/L09 and is a deliberate G4 §10 choice (capabilities are declared). A program may avoid it by using `on.press` links.
6. **`PVC-E-A11Y-ACTION` makes every `button` need an `on.press`** even on layout-only pages (G4 §8.1). Same trade-off: faithful to G4, charges PVC tokens on L02/L03/L09 where the intent specifies no behaviour.
7. `select`/combo-box, `slider`, accordion, `scroll`, `image`, hover events are **absent**: an agent that reaches for them gets `PVC-E-UNKNOWN-KIND`/`PVC-E-SCHEMA` with fixes (L06's intent allows a select; the reference does not use one).
8. The parser reports only the first syntax error.
9. The `pvc-validate` binary is a **debug build** at `/workspace/cargo-target/debug/pvc-validate`; behaviour does not depend on the profile.

## P2.6 Dependencies (Part 2)
No new Cargo dependency (still `ed25519-dalek =2.2.0`, `sha2 =0.10.9`, `icu_normalizer =2.3.0`). Tooling: none new (node v22.22.0, tiktoken 0.14.0 already installed; shared target dirs `/workspace/cargo-target` and `/workspace/cargo-target-wasm` per the Orchestrator's rule; my per-directory `target/` dirs were deleted).

## P2.7 Inconsistencies flagged (Part 2)
1. **dev-baseline `PVC_CHECKER_NEEDS.md` §4 vs the language**: it lists `list<record>` ops `append`/`update-where`/`remove-where`, `stepper`, `combo-box`, `slider`, `dialog` nodes and `storage`; the pilot subset provides bound-row `append`/`remove`/field assignment, a `layer` dialog and no `stepper`/`combo-box`/`slider`/`storage`. The references need none of the missing ones (G01 never needs storage: a "new session" is simply a fresh instance). dev-render must scope `pvc-probe` to what is in `PILOT_COVERAGE.md`.
2. **G2 §5 L06** says controls may be "switch, checkbox, select or text field"; `select` is not in this subset (see P2.5 item 7).
3. **G4 §4 `view`** says "declarative sort/filter"; only `where=` (filter) exists in the pilot subset.
4. **G4 §8.1 lists `list`/`list-item` and `tab`/`tab-list`/`tab-panel`, `dialog`/`alert-dialog`, `radio`/`radio-group`, `text-area`, `search-field`**: all now in `Role`. Still absent: `banner`, `complementary`, `contentinfo` (AccessKit has them; Orchestrator ruling: `region` + name), `combo-box`, `slider`, `stepper`, tables, menus, tree.
5. **PREREG (dev-pilot) §4.4 fixture text for G01** says "(a restart for the PVC runtime)": nothing in IR persists anything; a PVC program cannot store the dismissal at all (no `storage` capability in the pilot subset), so G01.3 holds for every valid PVC program by construction, while the baseline agent has to avoid `localStorage` explicitly. This asymmetry favours PVC and should be stated next to the G01 result.
6. G2 L03/L08/L09/L10 allow "inline vector icons or emoji"; PVC has no emoji font in the std font set (`fonts std`): an emoji in a `text` would render as a missing glyph on the reference fonts. Not checked on a real runtime.

## P2.8 Open issues
* dev-render must implement: grid, `md`/`sm` overrides, `view`/`slot`/`layer` instantiation with ids, `text` holes (`eval::text_content` is the reference), multi-handler press, hit-testing in templates, direction mirroring. API: `CHECKPOINT.md` (2026-10-09 update).
* A real-model smoke test of the context (does Qwen3-4B emit parseable `pvc` blocks?) is for dev-pilot's dry run; it would show whether the 2 511-token context needs a change (which means a new frozen hash).
* `#[non_exhaustive]` on the AST enums: say if wanted.
* Parser error recovery (more than the first syntax error per turn) would cut repair turns for PVC; deliberately not done (scope).

---
## Addendum 2026-10-09 — font manifest re-hash (request from dev-render via Supervisor)
`fonts std` now carries the hashes of dev-render's bundled files (`NotoSansCJKsc-Regular.otf` 6ead94f3…, `NotoSansCJKsc-Bold.subset.otf` a44805f5…, index 0; DejaVu unchanged). L01/F01 re-signed (canonical sha256 L01 `51dfc0bd…`, F01 `d1519615…`; signer unchanged). 81 tests still green (`evidence/cargo-test-part2.txt`), native-vs-wasm32 D1 re-run PASS (`evidence/d1-wasm.txt`). The A1 "open issue" about the font set is closed; token counts unaffected (`fonts std` is still 3 tokens; agent context unchanged, FROZEN hashes still valid).

## Addendum 2 — context re-frozen (v2)
The Orchestrator found L01's strings ("JD", "Jane") in `01_std_reference.md`. Fixed, and the three sent files were searched for other pilot strings: replaced `link "Docs"`, `radio "Yes"`, and in example 3 `button "Add"` / `name="Remove"` (labels used by L04/L09, G05, G02/G01/G04). Remaining intentional overlaps are language words, not specimen content: `region name="Header"` (the ruled landmark encoding), role/variant words such as `search`, `email`. New message: 8 354 bytes, **2 511 tokens**, sha256 `d30503040f448a0f9921a93c404c32469c901da7430942838bde80ea685320d9` (`agent-context/FROZEN.md`).
