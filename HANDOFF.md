# Session 2 hand-off (checkpoint integration)

**Branch:** `session2-evidence` (base `main` @ 5fdd412). **Written by:** the Orchestrator, at an owner-directed wrap-up. **Why it stopped:** four Developer threads (dev-render, dev-native, dev-web, dev-measure) ended on an API credit error, the container restarted, and the owner then directed the session to wrap up. This file says what exists, where it is, and exactly what the next agent must do.

**Read first:**
- `docs/session2/pm/PLAN.md`: the task breakdown, deviations V1–V10, and Supervisor decisions D-1…D-10 (ruled; listed in `docs/session2/pm/briefs/COMMON.md` §6).
- `docs/session2/pm/STATUS.md`: the full thread log, rulings, commit gates, and findings for C.
- `docs/session2/pm/briefs/`: one brief per Developer, plus the unexecuted `dev-render-2.md`.
- `docs/session2/dev/<name>/`: each Developer's REPORT and CHECKPOINT, where one exists.

**Labels that apply to every number on this branch:**
- Host `container-xeon4` (4 vCPU Xeon @ 2.1 GHz, no GPU).
- **No physical Tier 1 device was used** (Ruling 1). Deliverable A is *partial* by construction.
- "WebGL2" here means ANGLE→SwiftShader (CPU) in Chromium 141 headless. "GL" means wgpu GL on Mesa llvmpipe with WebGL2 downlevel limits imposed.
- General constraint solve excluded (D-4).
- Chromebook Crostini GPU acceleration and ChromeVox bridging remain **unverified**.

## Branch layout
| Path | What | Author |
|---|---|---|
| `Cargo.toml` (workspace) | members `crates/pvc-ir`, `crates/pvc-runtime`; hosts excluded | Orchestrator |
| `crates/pvc-ir/` | IR crate: grammar, canonical form, Ed25519 signing, validator, `pvc-validate` CLI | dev-ir |
| `crates/specimens/` | `L01.pvc/.pvca`, `F01.pvc/.pvca` (current, re-signed), canonical JSON, `CHECKLISTS.md` | dev-ir |
| `crates/pilot-refs/` | reference IR for the 15 pilot specimens, **never shown to an agent** | dev-ir |
| `crates/agent-context` | symlink → `pilot/agent-context` (for pvc-ir tests) | Orchestrator |
| `crates/pvc-runtime/` | runtime core, renderers, bundled fonts (with licences) | dev-render |
| `hosts/pvc-native/`, `hosts/pvc-native-scripts/` | native host + Xvfb/dbus/AT-SPI/fcitx5 scripts | dev-native |
| `hosts/pvc-web-host/` | wasm host crate `pvc-web/`, page `www/`, Playwright `tests/`, `build.sh`, `run_web.sh` | dev-web |
| `tools/measure/`, `tools/textbar/` | A6 harness (never run), A8 text-bar tooling | dev-measure |
| `measurements/session2/` | all evidence (table below) | various |
| `pilot/` | `PREREG.md` (interim v4), `agent-context/` (frozen v2), `checklists/` (frozen), `harness/`, `intents/`, `checker/`, `DRY-RUN-not-data/` | dev-pilot, dev-ir, dev-baseline |
| `docs/session2/pm/` | PLAN, STATUS, briefs, QUIET_WINDOW, feasibility findings | Orchestrator |
| `docs/session2/dev/` | Developer REPORT/CHECKPOINT/METHOD/RFC notes/evidence | each Developer |

Integration edits (path rewrites only):
- `crates/pvc-runtime/Cargo.toml`: `pvc-ir` → `../pvc-ir`.
- `hosts/pvc-web-host/pvc-web/Cargo.toml` and `hosts/pvc-native/Cargo.toml`: absolute `/workspace/dev_work/...` → repo-relative `crates/...`. dev-native's file already pointed at the **live** dev-ir crate; that edit is time-stamped 01:41:54Z, after the restart, and its author is unrecorded.
- `hosts/pvc-web-host/www/fonts/NotoSansCJKsc-Regular.otf` was dropped as a 16 MB duplicate. Copy it from `crates/pvc-runtime/fonts/` before serving.

Builds and tests:
- **Check at checkpoint:** `cargo test --release -p pvc-ir -p pvc-runtime --features pvc-runtime/cpu,pvc-runtime/test-unsigned` → **89 passed, 0 failed** (`measurements/session2/integration/workspace-tests.txt`).
- `pvc-runtime` tests still read specimens from the absolute `/workspace/dev_work/dev-ir/specimens` (`crates/pvc-runtime/tests/common/mod.rs:4`). Those files are identical to `crates/specimens`. **Next agent:** change that constant to `concat!(env!("CARGO_MANIFEST_DIR"), "/../specimens")`.
- Hosts were **not** built by the Orchestrator at integration.

## Per-task state

| Task | State | Evidence on this branch | What the next agent must do |
|---|---|---|---|
| **A1** IR, grammar, signing, validator subset, L01/F01 | **done** | `crates/pvc-ir/`, `crates/specimens/`, `docs/session2/dev/dev-ir/{REPORT,GRAMMAR,CHECKPOINT}.md`, `docs/session2/dev/dev-ir/evidence/` (D1 native = wasm32, tamper refusal, hard-wall scan) | Nothing. Current specimens: L01.pvca sha256 `b86a4f74…7813`, F01.pvca `88dfb47c…1139` (re-signed after the font re-hash). Anything measured on earlier artifacts must be labelled |
| **A2** runtime core | **done** | `crates/pvc-runtime/`, `docs/session2/dev/dev-render/REPORT.md`, `measurements/session2/dev-render-d3/` (geometry byte-identical native vs wasm32), `dev-render-tests.log`, `dev-render-hardwall/`, `dev-render-pins/`, `dev-render-rss/` (Tier 0 c headless peak VmHWM ≤ 23,648 kB, indicative, loadavg 5.04) | Fix the absolute specimens path in tests (above) |
| **A3** renderers (vello_cpu, vello_gpu on GL, WebGL2 replay) | **done** | as A2; renders in `measurements/session2/golden/` | — |
| **A4** native host, AT-SPI, system IME | **partial** (author thread terminated before REPORT.md; evidence files as left) | `hosts/pvc-native/`, `docs/session2/dev/dev-native/CHECKPOINT.md`, `measurements/session2/native-as-left/`. `out/ime_zh/`: real fcitx5 pinyin IME → winit `Ime::Preedit` … `Ime::Commit`, runtime state `name="你好"` (`ime_2_commit.state.json`, `host.log`). `out/a11y/`: AT-SPI dumps via pyatspi. `sanity/{gl,cpu}/` scenario runs with current specimens | 1. Restart the env: `hosts/pvc-native-scripts/run_native.sh` brings up Xvfb :77, dbus, AT-SPI, fcitx5; apt packages are present. 2. Confirm `cargo tree -e normal` shows **one** `pvc-ir`, at `crates/pvc-ir`. The pre-restart binary was built against a stale `dev-render/scaffold/pvc-ir-a1` snapshot. 3. Rebuild (release) and record the binary sha256. 4. **Re-run** `a11y`, `ime_zh` and `idle` on that binary with the current specimens. The existing `a11y`/`ime_zh` runs (00:39Z) used a **debug** build and the **old** F01 artifact (canonical `81b12fdb…`). 5. Write `REPORT.md`. Observed but not a budget result: VmHWM 163,828 kB in the debug-build IME run (`out/ime_zh/run.txt`) |
| **A5** web host, mirror, overlay | **partial** (author thread terminated before REPORT.md; evidence files as left) | `hosts/pvc-web-host/`, `docs/session2/dev/dev-web/CHECKPOINT.md`, `measurements/session2/web-as-left/out/`: `a11y` (Chromium AX tree shows every F01 role/name/state, live error), `ime` (labelled "browser IME composition pipeline driven via CDP; no OS IME engine involved", commit in state + AX tree), `idle` (0 frames / 0 draw calls in 5 s), `tamper` (refused), `dsf`, `requests.json` hard-wall audits | 1. Copy the CJK font into `www/fonts/`. 2. **Replace `www/artifacts/*.signed`**, which are A1-era artifacts written 23:44Z before the re-sign, with `crates/specimens/*.pvca`. Rebuild with `build.sh` (wasm-bindgen-cli 0.2.129 was in `dev_work/dev-web/tools/`; reinstall if absent). 3. Re-run `a11y`, `ime`, `idle`, `tamper`. 4. Write `REPORT.md` |
| **A6** measurement matrix (frame/RSS/idle/first-frame/mirror cost) | **not run** | Method only: `docs/session2/dev/dev-measure/METHOD.md`, harness `tools/measure/`. Indicative only: `measurements/session2/spotcheck/SUPERVISOR_frame_bench_tier0c.md` (Tier 0 c headless F01 type_en p50 12.26 / p95 15.26 ms; zh-CN IME p50 12.98 / p95 16.57 ms; idle no frame; relayed, not a raw file) | After A4/A5 re-runs: post binary shas, run the matrix inside the quiet window (`docs/session2/pm/QUIET_WINDOW.md`, exclusive `flock -x` on a CPU lock file), write `measurements/session2/SUMMARY.md` with host labels, then REPORT. **Q3 (mirror cost) and Q4 (Tier 1 budget with text + a11y) have no measurement until this runs** |
| **A7** golden parity | **done — 2/8 PASS, 6/8 FAIL** | `measurements/session2/golden/TOLERANCE.md` (committed d2f4f24 before any comparison), `golden/results.md`, diff images, `DIAGNOSIS_posthoc.md` (post-hoc) | Nothing. The bound stays as committed. Any new bound is a separate, dated post-hoc note |
| **A8** reduced G6 text bar | **done — bar NOT passed** | `measurements/session2/text-bar/RESULT.md` (+ METHOD_TEXTBAR, corpus, raw, work/ PNGs), `docs/session2/dev/dev-measure/` | Optional: Arabic/Devanagari/emoji and a human legibility study (not attempted) |
| **B1** pre-registration | **interim** (v4) | `pilot/PREREG.md` (sha256 `d6b7dffa…e678`; history a7ef25f → 18cac7e → 29ae5a7) | Fill the 8 PENDING markers (check-pvc command, baseline wrappers + B2 library name/version, validator binary sha, checker validation, harness hash). Re-hash and commit **before any run** |
| **B2** PVC agent context | **done** (frozen v2) | `pilot/agent-context/FROZEN.md`: **2,511** o200k tokens, sha256 `d3050304…20d9` (v1 2,509 superseded after the specimen-string leak) | Nothing. Any change needs a new hash and a commit |
| **B3** harness | **done (prep)** | `pilot/harness/`, `recount.py`, `docs/session2/dev/dev-pilot/REPORT.md`, `pilot/DRY-RUN-not-data/` | — |
| **B4** checklists + checkers | **partial** | `pilot/checklists/` (frozen, commit 70b5d73, 37 hashes verified), `pilot/checker/`, `docs/session2/dev/dev-baseline/PVC_CHECKER_NEEDS.md` (v1.1). No dev-baseline REPORT.md | dev-baseline: name the B2 component library + version, publish the adapter command lines and exit codes (`pilot/harness/adapters/ADAPTERS.md`), and do the final clean validation run. Then run the PVC checker validation once pvc-probe exists |
| **B4-rt** `pvc-probe` | **not started** | brief `docs/session2/pm/briefs/dev-render-2.md` | Build the headless `pvc-probe` per `PVC_CHECKER_NEEDS.md` v1.1, in `crates/pvc-runtime`. Landmarks use `region` + name (no new roles) |
| **B5** pilot run + analysis | **not run** | — | Gate list below, then run per PREREG: S4 = Qwen3-4B, then S7 = Qwen2.5-Coder-7B, under an exclusive CPU lock. Models are in `/workspace/tools/models/`, which is outside the repo; their sha256s are in PREREG |
| **C1** confidence update | **not started** | findings to use are in `docs/session2/pm/STATUS.md` ("Findings flagged for Deliverable C") | After B5: revise §0/§8.1 with build-readiness and token claim as separate rows, every point traced. **§0/§8.1 were deliberately not revised at this checkpoint** |
| **C2** integration | checkpoint only | this commit | Final integration after A–C |
| **C3** mechanical cleanup | not started | — | — |

## Pilot gate list (all must hold before the first real run)
1. A6 measurements finished (the pilot needs the CPU exclusively, and the S4 server RSS is about 9.7 GB).
2. `pvc-probe` built and dev-baseline's PVC checker validated against it.
3. B2 library name/version and adapter command lines published.
4. Final PREREG with every §12 hash filled, committed.
5. Checklists unchanged since 70b5d73, or re-committed with a new INDEX.
6. Pilot under `flock -x` for its whole duration.

Rulings already given:
- History is not re-counted (primary P).
- The decision rule applies per arm, and the trigger fires if either arm fails to win layout-heavy.
- `max_tokens` is 3072.
- Fixture sentences are appended identically to all stacks.
- L04 stays in the set.

## Findings already established (for C; each with its file)
- README HTML block = **184** o200k tokens, not 185: `measurements/session2/tokens/readme_recount.txt`.
- A *valid* L01 program = **150** o200k tokens, not G4 §15's 118 (that snippet is not a valid program): `docs/session2/dev/dev-ir/evidence/l01-token-count.txt`.
- PVC fixed context overhead = **2,511** tokens per turn, uncached: `pilot/agent-context/FROZEN.md`.
- G4 §8.1 omits `banner`/`contentinfo`/`complementary` (AccessKit 0.25.1 has them): `docs/session2/pm/STATUS.md`.
- A7: L01 PASS; F01 FAIL on p99.9 (0.0526–0.0533 vs 0.05): `measurements/session2/golden/results.md`.
- A8: Latin M3 8/8, zh-CN M3 2/8, line breaks 4/8 per script: `measurements/session2/text-bar/RESULT.md`.
- Real system IME (fcitx5 pinyin) reached winit and committed 你好 on the native host under Xvfb (debug build, A1-era artifact): `measurements/session2/native-as-left/out/ime_zh/`.
