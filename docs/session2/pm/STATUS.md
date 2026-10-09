# Session 2 STATUS (Orchestrator-maintained)

**Phase:** CHECKPOINT. Owner-directed wrap-up after four Developer threads terminated (API credit error) and the container restarted. See the "State at checkpoint" section and `HANDOFF.md` on the branch. The plan was approved by the Supervisor; decisions D-1 to D-10 were ruled (recorded in `briefs/COMMON.md` §6).
**Integration branch:** `session2-evidence` (local, created from main @ 5fdd412; Orchestrator commits only, Supervisor pushes).
**Plan:** `PLAN.md` · **Feasibility:** `feasibility/FINDINGS.md` · **Briefs:** `briefs/` (all Developers read `COMMON.md` first)

## State at checkpoint (honest summary)
**Measured (each with a file on `session2-evidence`):**
- A7 golden parity: 2/8 PASS, 6/8 FAIL. L01 passes; F01 fails p99.9 by about 6% (`measurements/session2/golden/results.md`).
- A8 text bar: NOT passed (`measurements/session2/text-bar/RESULT.md`).
- D3 geometry native = wasm32; D1 canonical bytes native = wasm32; tamper refusal; hard-wall scans clean.
- 89/89 workspace tests at checkpoint (`measurements/session2/integration/workspace-tests.txt`).
- Tier 0 c RSS probe, indicative (`measurements/session2/dev-render-rss/`).
- Supervisor spot-check of the Tier 0 c frame bench, relayed, indicative (`measurements/session2/spotcheck/`).
- Real fcitx5 IME commit on the native host; AT-SPI and Chromium AX evidence (author threads terminated; evidence as left; old artifacts / debug build).
- Token facts: 184 (README HTML), 150 (valid L01), 2,511 (PVC fixed overhead).

**Not measured / not done:**
- A6 matrix (Tier 1-path frame budgets, RSS, first frame, idle on hosts, web mirror cost): **not run**. Q3 and Q4 have no measurement.
- No physical Tier 1 device.
- A4/A5 evidence must be re-run on rebuilt binaries with current specimens; their REPORTs are missing.
- pvc-probe (B4-rt) not started; B4 final validation and adapters outstanding; PREREG still interim (8 PENDING).
- **G2 pilot not run**, so the falsification trigger is undetermined.
- **C not drafted**; §0/§8.1 unchanged by decision.

## Threads

| ID | Thread | Owner | State | Note |
|---|---|---|---|---|
| P0 | Feasibility | Orchestrator | done | F1–F20 |
| P1 | Plan | Orchestrator | done | approved; D-1…D-10 ruled |
| A1 | IR, grammar, signing, validator subset, L01/F01 | dev-ir | **done** (`dev_work/dev-ir/REPORT.md`) | Orchestrator re-ran `cargo test`: 45 passed; recount L01.pvc = 150 o200k with header comment stripped (185 with it) |
| A2/A3 | Runtime core and renderers | dev-render | **done** (`dev_work/dev-render/REPORT.md`) | now on B4-rt (`briefs/dev-render-2.md`), building only after the quiet window |
| A4 | Native host, AT-SPI, system IME | dev-native | **partial; thread terminated before REPORT.md, evidence as left** | scaffolds before A2 checkpoint; sole apt user |
| A5 | Web host, mirror, overlay | dev-web | **partial; thread terminated before REPORT.md, evidence as left** | scaffolds before A2 checkpoint |
| A6/A8 | METHOD, harness, measurements, text bar | dev-measure | A8 **done**: bar NOT passed overall (Latin M3 8/8, zh-CN M3 2/8, line breaks 4/8 per script), published as-is (`dev_work/dev-measure/measurements/session2/text-bar/RESULT.md`). A6 **not run**; thread terminated | |
| A7 | Golden parity | dev-render | **done**: 2/8 PASS, 6/8 **FAIL** (`dev_work/dev-render/measurements/golden/results.md`) | L01 idle PASS on both paths. F01 empty/error/zh-preedit FAIL the pre-registered p99.9 ≤ 0.05 bound on both paths (p99.9 0.0526–0.0533; mean ≤ 0.0005 and max ≤ 0.056, both within bounds; max channel diff 1/255). Reported as FAIL per TOLERANCE §6, not re-tuned. Results files 01:13Z, after the tolerance commit d2f4f24 at 00:38Z (Orchestrator checked). Post-hoc diagnosis in `DIAGNOSIS_posthoc.md`, labelled post-hoc |
| B1/B3 | PREREG, harness | dev-pilot | **prep done** (`dev_work/dev-pilot/REPORT.md`); interim PREREG v4 committed (supersedes 18cac7e), FROZEN v2 wired; 8 PENDING; final hash pending | Supervisor rulings: history NOT re-counted (primary P, H secondary); D-7 = every arm must satisfy, trigger fires if either arm fails to win layout-heavy; max_tokens 3072 labelled. Order `L01,G01,…,G05,L06…L10`, arm-major S4 then S7 |
| B4 | Checklists, baseline checker, PVC checker | dev-baseline | checklists committed 70b5d73; still owes the B2 library name/version, adapter command lines and final validation | independent of the agent-card author |
| B2 | Agent context frozen + pilot validator | dev-ir (part 2) | **done** (`dev_work/dev-ir/REPORT.md`); frozen context committed 4160bf1 | v2: 2511 o200k tokens, sha256 d3050304…20d9 (v1 2509/ae8de918 superseded). 81 tests. `pvc-validate` at `/workspace/cargo-target/debug/pvc-validate`. **Open:** the std reference avatar example uses L01's "JD"/"Jane", a contamination flag (see below) |
| B4-rt | Headless runtime API for pilot constructs | dev-render (2nd brief) | **not started** (dev-render thread terminated) | from `PVC_CHECKER_NEEDS.md` v1 |
| B5 | Pilot run and analysis | dev-pilot | open | after A reported, PREREG + checklists + B2 committed |
| C1 | Confidence-update draft | — | **not started** | |
| C2 | Integration | Orchestrator | checkpoint integration done (see HANDOFF.md) | |
| C3 | Mechanical cleanup | Technical Writer | open | brief later |

## Orchestrator decisions logged
- **L04 landmarks:** L04 stays in the pre-registered set (no post-hoc drop). Header/aside/footer are encoded as G4 `region` + name. The checker accepts the ARIA landmark role or region + matching name, identically on both stacks. No new roles are added. G4 §8.1 omits `banner`/`contentinfo`/`complementary` although AccessKit 0.25.1 has `Banner`/`Complementary`/`ContentInfo`; this is flagged for C.
- **Fixture sentences (G01–G05 and any L rows):** dev-baseline's `checklists/<ID>.md` "Fixture" sentences are appended verbatim to the G2 intent, identically for PVC/B1/B2, counted as request tokens, and fixed in PREREG before any run. They are a deviation from G2 §4.1's "same intent text", labelled (same for both stacks, so the comparison stays paired).

## Build beyond L01/F01 (D-2, must also appear in the README scope note)
The headless pilot validator and checker cover the node kinds of L01–L10 and G01–G05 (dev-ir part 2; dev-render B4-rt). No rendering is built for those kinds.

## Pilot run blockers (B5 "go" requires all of these)
- [x] dev-ir `FROZEN.md` committed 4160bf1 (see contamination flag)
- [ ] working `pvc-validate` (dev-ir) and `check-pvc` (dev-baseline, needs dev-render `pvc-probe` B4-rt)
- [ ] dev-baseline: B2 component library name + version; adapter command lines and exit codes per `dev-pilot/harness/adapters/ADAPTERS.md`
- [ ] checklists committed; PREREG committed with all §12 hashes filled
- [ ] **Quiet CPU window** (exclusive-lock amendment relayed to dev-pilot by the Supervisor): A measurements finished; no builds. The pilot holds the CPU lock **exclusively** for its duration, not shared. Reason: under load 15–17 generation fell to 0.05–2 tok/s (dev-pilot dry run). S4 server RSS ≈ 9.7 GB at `-c 32768`, so no other multi-GB job may run alongside it.

## Quiet window (declared): see `QUIET_WINDOW.md`
State: **pre-window.** dev-render ready (frame_bench a213611a…, rss_probe 188cd235…). Waiting for dev-native (pvc-native) and dev-web (pvc-web). While `/workspace/orchestrator_work/pm/QUIET_WINDOW_OPEN` exists, only dev-measure may build or run CPU jobs.

## Sequencing
1. Finish all A builds and measurements: A6 runs, A7 golden, A8 text bar.
2. Report A to the Supervisor.
3. dev-render's B4-rt (pvc-probe) and dev-baseline's PVC checker validation: build work only, no exclusive lock.
4. Commit the final PREREG, checklists and FROZEN.md.
5. Pilot under the exclusive lock. S4 is about 270 turns, *estimate*, at about 9 tok/s, so it takes hours with all other CPU work blocked. Then S7, which is slower at about 5.5 tok/s.
6. C1, written while the CPU is idle after the pilot or during the S7 run (writing only, no CPU-heavy work).

## Infra rulings
- Shared Cargo target dirs: `/workspace/cargo-target` (native) and `/workspace/cargo-target-wasm` (wasm32), from the Supervisor ruling after the disk filled. dev-native and dev-render delete their own `target/` after their next shared build. COMMON.md §2 is updated. The Orchestrator deleted the feasibility probe targets. The Supervisor truncated the unused Phi-4-mini GGUF, so it is no longer usable.

## Commit gates (order-sensitive, verifiable in branch history)
- [x] `TOLERANCE.md` committed before any golden comparison (A7): commit d2f4f24 on `session2-evidence`, path `measurements/session2/golden/TOLERANCE.md`, sha256 0e6f1c8a…f6f7. Author disclosure: eyeballed one L01 render per path beforehand, no metric computed. dev-measure review requested.
- [x] `checklists/` committed before any pilot run: 70b5d73 (`pilot/checklists/`, 15 specs + U.md + core/*.mjs + core/specs/*.mjs; all 37 INDEX hashes verified by the Orchestrator). Preliminary validation: 30/30 pass examples pass and 42+ mutations fail the intended item. Final clean re-run pending.
- [x] agent context `FROZEN.md` committed before any pilot run: 4160bf1 (`pilot/agent-context/`). **Re-frozen v2, committed** over 4160bf1: 2511 tokens, sha256 d3050304…20d9 (Orchestrator re-verified the joined hash and count; specimen-string grep is clean). `region name="Header"` kept by ruling.
- [~] `PREREG.md`: **interim v4** (sha256 d6b7dffa…e678; history a7ef25f → 18cac7e → v4; FROZEN v2 wired; rulings recorded, PENDING markers left for the freeze and adapters). The **final** re-hash must be committed before the first real run.

## Integration notes (apply when integrating)
- **Blocking before the window:** dev-native's `pvc-ir` path pointed at `dev-render/scaffold/pvc-ir-a1` (stale A1 snapshot, 8 source files differ) while pvc-runtime and pvc-web use the live `dev-ir/pvc-ir`. Asked dev-native (through the Supervisor) to switch, verify a single pvc-ir in `cargo tree`, and re-run key evidence before measurement-ready. At integration, every path dep must point at the integrated `crates/pvc-ir`.
- **Specimens superseded:** dev-ir re-hashed the CJK font entries to dev-render's bundle (`NotoSansCJKsc-Regular.otf` 6ead94f3…, `NotoSansCJKsc-Bold.subset.otf` a44805f5…) and re-signed. Current files: `L01.pvca` sha256 b86a4f74…7813 (canonical 51dfc0bd…), `F01.pvca` 88dfb47c…1139 (canonical d1519615…). Every A measurement must state the specimen sha. Reject any measurement made against the earlier A1 artifacts, or label it as such. Token counts are unaffected.

## Findings flagged for Deliverable C
- F4: README HTML block = 184 o200k tokens, not 185 (Orchestrator recount).
- dev-ir A1 §7.1: a *valid* L01 program (manifest `cap net.main`, `effect`, `locales`, `fonts std`) = **150** o200k tokens (`dev_work/dev-ir/evidence/l01-token-count.txt`), not G4 §15's 118, which G4 itself labels "not a valid program". Against 184 that is ~18% fewer, not 36%. `fonts std` sugar (3 tokens) replaces 4 hashed font lines (226 tokens). README/G4 §2.1 "118 vs 185" must not be carried as the like-for-like number.
- G4 §8.1 omits `banner`/`contentinfo`/`complementary` (AccessKit 0.25.1 has them); v0 encodes as `region` + name.
- **Fixed PVC overhead (measured):** 2511 o200k tokens per turn, uncached (v2) (`pilot/agent-context/FROZEN.md`).
- dev-ir: G01.3 ("hint returns in a new session") passes PVC *by construction* (session state semantics), so state that beside the result. Geometry items depend on pvc-probe. Learnability by small models is untested. PVC boilerplate (header/manifest) costs tokens on layout pages.
- **Contamination flag (Orchestrator):** `01_std_reference.md` line 9 uses `avatar "JD" … alt="Jane"`, which are L01's strings, although the brief said no example may come from a pilot specimen. It is small (syntax example only), but it is a leak toward L01.
- Risk for A: dev-ir's font set is the system files, about 40 MB of CJK. Watch the 150 MB RSS budget (dev-render/dev-measure).

- **A7 golden parity:** 6/8 state×path comparisons FAIL the pre-registered p99.9 bound by about 6% (0.053 vs 0.05). The differences are uniformly 1/255 per channel. Report as FAIL, not re-tuned. C may cite the post-hoc diagnosis only as post-hoc.
- **A8 text bar:** not passed overall (zh-CN M3 2/8; line breaks 4/8 per script). Q2 (legibility) does not earn credit for CJK.

## Labelled gaps known now
- No physical Tier 1 device (A is partial, Ruling 1). Chromebook Crostini GPU/ChromeVox: **unverified, stays unverified**.
- Both pilot arms are small local models; no hosted or frontier arm.
- IME under Xvfb unverified (F14).
- README/G2/G7 say the HTML block is 185 o200k tokens; the recount gives 184 (F4). This is to be reported in C, not fixed.

## Environment changes (all agents; Developers' installs are added on report)
| Change | Version | Where | By |
|---|---|---|---|
| pip `tiktoken` (+ `regex`) | 0.14.0 | system python | Orchestrator |
| llama.cpp CPU release | b11514 (de7fa0a3c) | `/workspace/tools/llama-b11514/` | Orchestrator |
| GGUF models | Qwen3-4B Q4_K_M; Qwen2.5-Coder-7B-Instruct Q4_K_M; Phi-4-mini-instruct Q4_K_M (unused; truncated by Supervisor for disk) | `/workspace/tools/models/` | Orchestrator |
| rustup target | wasm32-unknown-unknown | toolchain 1.97.0 | Orchestrator |
| Planned by Developers (D-9) | apt at-spi2-core, python3-pyatspi, fcitx5(+chinese-addons)/ibus(+libpinyin), xdotool; Playwright Firefox; wasm-bindgen-cli 0.2.129; flip-evaluator; npm baseline toolchain | own dirs / apt | dev-native, dev-measure, dev-web, dev-render, dev-baseline |

## Cargo dependencies outside the G7 table (RFC notes)
Pre-approved (D-10), notes pending: ed25519-dalek, sha2 (pvc-ir); wasm-bindgen 0.2.129, web-sys 0.3.106, js-sys 0.3.106 (pvc-web).
Approved by Supervisor mid-flight: `icu_normalizer =2.3.0` (pvc-ir; NFC canonical strings per G4 §3; already in the pinned graph's lock via parley/ICU4X); RFC note at `/workspace/dev_work/dev-ir/rfc/icu_normalizer.md` (to verify at integration).
