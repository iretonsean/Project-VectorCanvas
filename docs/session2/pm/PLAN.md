# Session 2 Plan — Project VectorCanvas (Orchestrator)

**Status:** draft for Supervisor plan pass. No Developer has been spawned. Nothing in the repo has changed (main @ 5fdd412).
**Grounding:** feasibility checks run before writing this plan → `/workspace/orchestrator_work/pm/feasibility/FINDINGS.md` (F1–F20, each with evidence file or command).
**Product of the session:** evidence. Every task below exists because one of the four §8.1 questions needs it:

| Q | §8.1 open question | Deliverable |
|---|---|---|
| **Q1** | Does PVC's per-session context overhead eat the token savings? | B |
| **Q2** | Text legibility and system-IME integration on Tier 1 hosts | A |
| **Q3** | Per-frame cost of the web accessibility mirror | A |
| **Q4** | Tier 1 frame budget with real IR, text and accessibility together | A |

---

## 1. Feasibility verdicts (the two things most likely to be impossible)

### 1.1 Physical Tier 1 device — **impossible here; Ruling 1 applies**
No phone, Chromebook or adb (F2). Deliverable A is planned as **partially met by construction**. What this container can measure, and how each is labelled in every file:

| Path | What actually runs | Label used on every number |
|---|---|---|
| Tier 0 (c) | `vello_cpu` 0.3.0, offscreen and windowed under Xvfb | `host=container-xeon4 / tier0c-vello_cpu` |
| Native Tier 1 *code path* | winit 0.30.13 + wgpu 30.0.1 **GL backend on Mesa llvmpipe** + `vello_gpu` 0.3.0, WebGL2 downlevel limits **imposed** (llvmpipe itself exposes compute, F8) | `host=container-xeon4 / gl-llvmpipe (Tier 1 code path on Tier 0 (c) hardware class)` |
| Web Tier 1 path | wasm32 + `vello_gpu` `webgl` feature in **Chromium 141 headless, `navigator.gpu` hidden**, WebGL2 = ANGLE→SwiftShader (CPU) (F5, F6) | `host=container-xeon4 / chromium141-webgl2-swiftshader` |

**Device I would pick:** the **Intel Celeron N4000-class Chromebook, 4 GB** (§4.4.1 Tier 1 (d)). Reasons: (1) the web path measured here (Chrome, WebGL2, DOM mirror, hidden-input IME overlay) runs unchanged in Chrome on ChromeOS, so it is the strongest transfer of this session's work; (2) it is also the §4.7 authoring reference device. **Risk stated up front (unverified):** a native host on a Chromebook would run in the Crostini Linux VM; whether Crostini gives GPU acceleration on N4000-class hardware and whether ChromeOS bridges a Crostini app's AT-SPI tree to ChromeVox are both unverified. If either fails, the native-host half of the Tier 1 test would have to move to the budget Android reference (Tier 1 (a)), which needs an Android host (android-activity, `accesskit_android`, InputConnection IME) that this session does not build.
**What remains to run on the device:** every frame-time, RSS, idle and first-frame number; IME through the ChromeOS IME; ChromeVox reading both the native tree and the web mirror; the G6 text bar at the device's panel scale.

### 1.2 Real ≤4B local model arm — **feasible (checked)**
llama.cpp b11514 CPU binary runs; `Qwen3-4B` Q4_K_M (Apache-2.0, official Qwen quantisation) generates at **9.4 tok/s, prefill 102 tok/s** on the 4 cores (F16–F18). A chat call produced a plausible Tailwind card in 25.9 s. `llama-server` returns per-call `usage`, which becomes G2's secondary count.
**The second arm is the problem, not the ≤4B arm:** no hosted-model API key exists (F19), so G2 Arm F / Arm M cannot run unless the Supervisor provides a key. Two other real local models are downloaded and benchmarked: `Qwen2.5-Coder-7B-Instruct` Q4_K_M (5.5 tok/s) and `Phi-4-mini-instruct` Q4_K_M (third-party quant, 9.1 tok/s). See decision **D-1**.

### 1.3 Other checks
- **tiktoken / o200k_base:** installs (0.14.0) and works (F3). Recount finding: README HTML block is **184** tokens, not 185 (F4). Carried to Deliverable C as a flagged inconsistency.
- **Playwright Chromium:** present (Chromium 141.0.7390.37). Firefox 142 installable for G6's relative bar (F7).
- **G7 pins:** resolve to one version each and compile natively (64 s) and for wasm32 (58 s) (F10, F11).
- **Hard-wall scan:** no HTTP/LLM client in the graph. `tokio` is in Cargo.lock via `wasm-bindgen-futures` but only for emscripten+unstable cfg, features `rt,time` (F12). The Supervisor's grep should run on source and on `cargo tree -e normal --target <t>`, not on Cargo.lock text.
- **IME:** no IME framework installed; fcitx5/ibus + pinyin installable via apt. Whether they drive winit XIM under Xvfb is unverified (F14).
- **Native a11y consumer:** AT-SPI registry and pyatspi installable via apt (F15).

---

## 2. Scope guard (answers the red-team question "does any task build something L01/F01 do not need?")

- **Renderer, native host and web host build only L01 + F01 needs:** node kinds `surface`, `box`, `text`, `text-input` (single-line, email, password), plus `std` card/avatar/button/checkbox/text-field expansions; layout sugar `row`, `column`, `pad`, `gap`, `align`, fill-width; paint = rounded-rect, circle, solid fill, 1-px border; semantics for the roles those use; behaviour = typed state, `change`/`press`/`submit`, guards, assignments; one effect stub (`connect_user`, capability declared, **never dispatched to the network**). No `scroll`, `view`, `slot`, `layer`, `image`, `shape`, gradients, shadows, animations, raw constraints, general Cassowary solver.
- **One thing is needed by B, not by L01/F01: the pilot's PVC validator/checker** must accept and check whatever the ≥10 pilot specimens need (grid, breakpoints, lists, `slot`, list ops, decimal…). This is headless (validate + layout solve + semantics tree + behaviour trace, no pixels). It is flagged as decision **D-2**, not hidden.
- No visual editor, no Tier 2, no second CJK locale, no iOS/macOS/Windows host, no shared-evolution upload, no LLM or network call in any runtime/host crate.

---

## 3. Threads (tasks), with "done" and the question each serves

Owners are Developer names; each works only in `/workspace/dev_work/<name>/`. One author per file. Cross-Developer use is by **read-only path dependency** on another Developer's directory; the Orchestrator rewrites paths at integration.

### Deliverable A — Tier 1 vertical slice

| ID | Task | Owner | Done means | Serves |
|---|---|---|---|---|
| **A1** | `pvc-ir` crate: concrete terse grammar for the L01/F01 subset (written down in `GRAMMAR.md`), terse→canonical expansion with canonical byte serialisation, Ed25519 signing + verify-on-load, validator rules needed by L01/F01 (`PARSE`, `SCHEMA`, `UNKNOWN-KIND`, `REF`, `A11Y-NAME`, `CAP-UNDECLARED`, `STRING-MISSING`); `specimens/L01.pvc`, `specimens/F01.pvc` (F01 with zh-CN strings table in addition to en) | dev-ir | `cargo test` green; D1 test: same input → byte-identical canonical output across 2 runs and across native vs wasm32 build; tampered-manifest artifact (G2 X24) refused at load with a test; L01/F01 signed artifacts committed; early checkpoint: AST types + specimen files published so A2 can start | Q4 (real IR), enables Q1 |
| **A2** | `pvc-runtime` core: layout solve for the A1 subset (direct sugar solver, 1/64-px snapping), Parley 0.11.1 text with **bundled** fonts (Noto Sans + Noto Sans CJK SC, content-hashed), Parley `PlainEditor` for F01 fields (masking, show/hide, caret, IME preedit), behaviour interpreter for F01 (validity guards, submit disabled until valid, error with `live=polite`), semantics → `accesskit::TreeUpdate` (single source for native tree **and** web mirror), damage/dirty flag (idle ⇒ no frame), scene build for vello | dev-render | D3 test: identical solved geometry for L01/F01 at 360×640 and 1366×768, native vs wasm32; F01 state-trace test passes F01's four G2 checklist items headlessly; `cargo tree -e normal` shows no HTTP/LLM/net crate | Q2, Q4 |
| **A3** | Renderers: `vello_cpu` (Tier 0 c, golden reference) and `vello_gpu` on wgpu-GL with `Limits::downlevel_webgl2_defaults()` | dev-render | Both produce L01/F01 frames offscreen; PNGs committed | Q4 |
| **A4** | Native host `pvc-native`: winit 0.30.13 window on Xvfb, wgpu GL/llvmpipe + vello_gpu, `accesskit_winit` 0.34.1 → AT-SPI; system IME via winit `Ime` events with **fcitx5 + pinyin (zh-CN)** (ibus-libpinyin as fallback) driven by `xdotool` | dev-native | (a) external `pyatspi` process reads the live tree: roles/names for every F01 interactive element, checkbox state change visible after a key press; (b) typing `nihao` + select through the real IME commits `你好` into F01's name field and preedit is drawn — **or** a written, evidenced statement that the IME framework could not be made to talk to winit under Xvfb, with what was tried. No "IME works" claim from synthetic events | Q2, Q4 |
| **A5** | Web host `pvc-web`: wasm32, `vello_gpu` `webgl` on a canvas, hidden positioned DOM/ARIA mirror generated from the A2 `TreeUpdate` (on by default), hidden native `<input>` overlay per text field (G6 §6), `navigator.gpu` hidden by init script and asserted | dev-web | Playwright: Chromium accessibility snapshot (`page.accessibility`/CDP `Accessibility.getFullAXTree`) shows F01 roles/names/states from the mirror; typing into overlay updates canvas text; CDP `Input.imeSetComposition` + `insertText` commits `你好` — **labelled "browser IME composition pipeline driven via CDP; no OS IME engine involved"**; WebGL2 context asserted, `navigator.gpu === undefined` asserted per run | Q2, Q3, Q4 |
| **A6** | Measurement harness + method: `METHOD.md` written **before** any number (frame definition, interaction scripts, RSS method, idle method, first-frame method, host label format); one-command re-run script per scenario so the Supervisor can repeat one; outputs raw per-frame CSV + summary JSON with host label, commit, command, date | dev-measure | `measurements/session2/{tier0c,gl-llvmpipe,web-webgl2}/{L01,F01}/…` exist; each summary gives p50/p95/p99/max interaction-frame ms, idle frames in a 5 s window, first-frame ms, peak RSS (native: `VmHWM`; web: sum of renderer+GPU process RSS from /proc, labelled), and mirror-update ms per frame separately (Q3); every number compared to §4.4.1 (16.6 ms / 33 ms Tier 0, 150 MB, 0 FPS idle, first frame <1 s / <1.5 s) as **pass/fail on this host only** | Q3, Q4 |
| **A7** | Golden-image parity: `TOLERANCE.md` committed to the repo branch **before** the first comparison (metric: LDR-FLIP mean and a max-pixel bound, rationale, per path), then gl-llvmpipe and web-webgl2 vs Tier 0 `vello_cpu` for L01/F01 states | dev-render (tolerance reviewed by dev-measure) | Tolerance commit precedes results commit in branch history; result table with FLIP numbers, pass/fail, diff images | Q4 |
| **A8** | G6 text bar, reduced: Latin + Simplified Chinese strings from L01/F01 + G6 corpus, 11/13/16/24 px, DSF 1.0 and 2.0, same font file; LDR-FLIP PVC(vello_cpu) vs Chromium 141 and **Firefox 142 vs Chromium 141**; line-break equality vs Chromium | dev-measure | Table per script/size/scale with pass/fail against G6 §5 relative threshold; labelled gaps: no human legibility study, no Arabic/Devanagari/emoji, Linux rasterisers only, Chromium/Firefox code-sharing caveat (G6 §5) | Q2 |

### Deliverable B — G2 pilot (runs only after A is reported; prep may run in parallel, see D-6)

| ID | Task | Owner | Done means | Serves |
|---|---|---|---|---|
| **B1** | Pre-registration `pilot/PREREG.md`, committed before any run: specimens + **fixed run order**, arms (model, quant, sha256, llama.cpp build, sampling: temp 0, seed, thinking off), turn cap 6, repair-feedback templates, exact T_total accounting per G2 §2 items 1–5 (what counts as request vs context overhead vs tool vs repair; chat-template special tokens excluded from o200k counts, stated), secondary count = llama-server `usage`, CI method (paired percentile bootstrap over specimens, 10 000 resamples, fixed seed), decision rule + falsification trigger quoted verbatim, deviation list | dev-pilot | File committed; Supervisor has seen it; hash recorded in STATUS | Q1 |
| **B2** | PVC agent context: agent card + `std` reference + en dialect pack, **frozen and hashed before runs**; its o200k_base size is the published "fixed overhead" number | dev-ir | Frozen file + token count + hash in PREREG | Q1 |
| **B3** | Pilot harness: drives llama-server, saves every turn verbatim (JSONL: role, exact text, o200k count, server usage), validator tool for PVC, equivalent tools for baselines (html-validate, `tsc`, axe-core), standard repair feedback | dev-pilot | Dry run on one specimen per stack; one transcript recount script (`recount.py <transcript>`) the Supervisor can run | Q1 |
| **B4** | Fidelity checks as executable code, written before runs: per-specimen G2 checklist A-items + universal U1–U5, for PVC (headless: dev-ir's validator/checker, D-2) and for baselines (Playwright on Chromium) | dev-baseline (baseline side + checklist specs), dev-ir (PVC checker) | Each checker validated on a hand-written passing and failing example per specimen before runs | Q1 |
| **B5** | Run + analysis: T_total per run, medians/p90, pass@1/pass@6, output-only tokens, cached-at-zero variant, per-category range with 95% CI per arm, decision rule applied as written, falsification statement | dev-pilot | `measurements/session2/g2-pilot/` with transcripts, per-run CSV, `RESULT.md`; result published whichever way it goes | Q1 |

### Deliverable C — confidence update

| ID | Task | Owner | Done means | Serves |
|---|---|---|---|---|
| **C1** | Draft revised §0, §8.1 and README claims: two separate rows (build-readiness; token claim), points earned only for what was measured, each number cited to a measurement file path, "checked vs inferred" split, F4 (184 vs 185) inconsistency reported | dev-evidence (independent of every builder) | Draft in own dir; sums verified; every number traced | Q1–Q4 |
| **C2** | Integration of A/B/C into repo branch; README/analysis edits applied by Orchestrator only | Orchestrator | Branch builds and tests from clean checkout | — |
| **C3** | Mechanical cleanup only (links, table formatting, file index) | Technical Writer | Diff touches no number or claim | — |

### Dependency order
A1(checkpoint) → A2 → A3 → {A4, A5} → A6 → A7/A8. B1/B2/B3/B4 can be prepared alongside A (D-6); B5 runs after A is reported; C1 after B5.
**CPU contention rule:** the container has 4 cores. A lock file `/workspace/orchestrator_work/pm/CPU_LOCK` must be held for any timing measurement and for any pilot run; no compile, model or browser job may run during a timing measurement. Each measurement summary records load average at start.

---

## 4. Repository layout at integration (branch `session2-evidence`, local only; Supervisor pushes)

```
Cargo.toml                 workspace, exact G7 pins
crates/pvc-ir/             dev-ir     (validator: no network, P2)
crates/pvc-runtime/        dev-render (the "runtime crate" for the hard-wall grep)
hosts/pvc-native/          dev-native
hosts/pvc-web/             dev-web
specimens/                 L01.pvc, F01.pvc + signed artifacts
tools/measure/             dev-measure
pilot/                     dev-pilot, dev-baseline, agent card by dev-ir
measurements/session2/     raw + summaries, all host-labelled
docs/S2_Evidence_Report.md dev-evidence (checked vs inferred)
```

## 5. Deviations declared in advance (each will be repeated in the deliverable where it applies)

| # | Deviation | Why |
|---|---|---|
| V1 | No physical Tier 1 device; A is partial | F2, Ruling 1 |
| V2 | "WebGL2" = SwiftShader CPU emulation; "native GL" = llvmpipe with imposed downlevel limits | F6, F8 |
| V3 | IME: Linux X11 + fcitx5/ibus under Xvfb (if achievable) and CDP-driven browser composition; no Android/ChromeOS IME | F14 |
| V4 | Native a11y verified by an AT-SPI client (pyatspi), not a screen-reader user test | F15 |
| V5 | No general Cassowary solver; sugar solved directly | scope guard (D-4) |
| V6 | Text bar: Latin + zh-CN only, no human legibility test, Linux rasterisers | scope, no judges |
| V7 | Pilot arms local-only (unless D-1 yields a key) | F19 |
| V8 | H checklist items not human-judged (D-5) | no judges |
| V9 | Intent texts are PM-drafted G2 texts, not externally rewritten (G2 §7.3 unmet) | no external authors |
| V10 | Secondary count = llama.cpp `usage` (model's own tokenizer), not vendor billing | local arms |

## 6. Decisions needed from the Supervisor before I spawn Developers (my default in bold)

- **D-1 Second arm.** No hosted key. (a) you provide a hosted API key for an Arm F/M; or (b) **second local arm = Qwen2.5-Coder-7B-Instruct Q4_K_M** (official quant, Apache-2.0, ≤8B = G2 Arm S class); or (c) Phi-4-mini (≤4B, different family, third-party quant). Under (b)/(c) the pilot speaks for small local models only and says so.
- **D-2 Pilot validator/checker beyond L01/F01** (headless, no rendering) so the PVC arm is tested against v0, not against a subset that makes most specimens fail with `UNKNOWN-KIND`. **Recommend yes.** Alternatives are biased: subset-only (measures our subset) or choosing only subset-friendly specimens (selection bias).
- **D-3 Specimens.** **L01–L10 (all of category 1, which carries the falsification trigger) + G01–G05 (lowest five IDs of category 4) = 15**, with a pre-registered interleaved run order; if compute binds, the analysed set is the completed prefix of that order (fixed in advance, not chosen by results), never below L01–L05 + G01–G05.
- **D-4** Direct sugar solver instead of Cassowary for this session (V5). **Recommend yes**; frame budgets then exclude a general solve, which is stated.
- **D-5 H items** (L02, L03, L05 contain one each). **Recommend: reported as "not judged", pass decided on A items only, labelled.** Alternative: you or the owner act as two blind judges on screenshots.
- **D-6** May B1–B4 *prep* (no model runs) proceed in parallel with A, under the CPU lock rule? **Recommend yes.**
- **D-7** "Across all agent arms" in the G2 §1 decision rule: **each arm must individually satisfy it (conservative)**, or pooled?
- **D-8 Falsification trigger on a small pilot:** **apply as written** — if PVC does not *win* layout-heavy (including CI touching zero), the README says the trigger fired, with n and arms stated next to it. Confirm.
- **D-9 Environment changes outside Cargo:** apt `at-spi2-core python3-pyatspi fcitx5 fcitx5-chinese-addons xdotool` (ibus + ibus-libpinyin fallback); Playwright Firefox; `wasm-bindgen-cli =0.2.129`; pip `flip-evaluator`; npm baseline toolchain (react, react-dom, typescript, tailwindcss, esbuild, html-validate, axe-core, one component library). **Recommend: allowed, each listed in STATUS.md with version; RFC notes required only for Cargo dependencies of shipped crates.** Confirm or extend the RFC rule to tooling.
- **D-10 Foreseen Cargo additions outside G7 (RFC note each):** `ed25519-dalek` + `sha2` (signing, pvc-ir); `wasm-bindgen`, `web-sys`, `js-sys` as direct deps of `pvc-web` at the versions already in the graph (wasm-bindgen 0.2.129, web-sys 0.3.106, js-sys 0.3.106). No serde in runtime crates. Any other addition comes to you first.

## 7. Risks I am tracking

1. **IME under Xvfb may not work** (no desktop session, XIM handshake). Mitigation: two frameworks tried; result reported either way. If it fails, Q2's native-IME half earns nothing.
2. **4B models may rarely pass** either stack within 6 turns → few valid pairs → wide or undefined CI. That is a result, reported as such.
3. **SwiftShader/llvmpipe frame times are CPU-bound** and may *fail* budgets that real Tier 1 GPUs would pass, or pass for the wrong reason (4 Xeon cores vs a Celeron). Numbers are host-labelled and not extrapolated.
4. **Agent-card tuning bias:** the card author knows the public G2 intents. Card frozen and hashed before runs; checklists written by a different Developer.
5. **CPU contention** between builds, browser, and LLM corrupts timings → lock rule (§3).
