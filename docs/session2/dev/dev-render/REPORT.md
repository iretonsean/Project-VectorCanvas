# REPORT — dev-render (A2 runtime core, A3 renderers, A7 golden parity)

Author: dev-render. Date: 2026-10-09. Repo main @ 5fdd412 (read-only). Everything lives under `/workspace/dev_work/dev-render/`. API: `CHECKPOINT.md`. Bound: `TOLERANCE.md`.

## 1. Summary
- `pvc-runtime` (`pvc_runtime`) loads the signed L01/F01 artifacts through `pvc_ir::load_signed`, solves layout with a direct sugar solver (general constraint solve excluded, D-4), shapes text with Parley 0.11.1 on bundled, content-hashed fonts (no system fonts), edits with `PlainEditor` (masking, show/hide, IME preedit/commit), runs the F01 behaviour, emits one `accesskit::TreeUpdate` stream, and asks for **no frame when idle**.
- One display list is replayed on three paths: Tier 0 `vello_cpu`, `vello_gpu` on wgpu-GL (llvmpipe, WebGL2 downlevel limits), and `vello_gpu` WebGL2 in Chromium 141 (`navigator.gpu` hidden, SwiftShader). All four golden states were rendered on all three paths.
- **A7 golden result, stated plainly: L01 PASSES on both paths; the three F01 states FAIL the pre-registered bound on both paths, on one statistic (p99.9 FLIP 0.0526–0.0533 against a 0.05 bound).** Mean (≤0.0005) and max (≤0.056) are far inside their bounds, and every differing pixel differs by 1/255. The bound was not edited. The cause is flat-area 8-bit rounding in opacity-layer and stroke compositing (CPU u8 pipeline vs GL), see `measurements/golden/DIAGNOSIS_posthoc.md` (post-hoc).
- D3: solved geometry (and text line widths, caret position) at 1366×768 and 360×640 is byte-identical native vs wasm32. The F01 four-item G2 checklist passes headlessly. Hard-wall: no HTTP/LLM/socket crate in any normal dependency tree.
- Nothing here ran on a phone or Chromebook. Frame-time numbers in this report are headless Tier 0 numbers at high background load and are superseded by dev-measure's A6 window.

## 2. Done criteria (brief → status → evidence)
| Criterion | Status | Evidence |
|---|---|---|
| A2.1 Load signed IR; refuse unsigned except test path | met | `Runtime::from_signed` only takes the output of `pvc_ir::load_signed`; `new_unsigned_for_test` exists only under feature `test-unsigned`; tampered artifact refused: `tests/specimens.rs::unsigned_or_tampered_is_refused` (`measurements/tests.log`) |
| A2.2 Direct sugar layout, 1/64 snapping, D-4 sentence | met | `src/layout.rs`; sentence in CHECKPOINT and below |
| A2.3 Parley + bundled content-hashed fonts, system fonts off, glifo glyph path; font bytes and CJK RSS | met | `src/fonts.rs` (hash unit test, `no_system_fonts` test); glyphs via `vello_*::glyph_run` on glifo 0.4.0; numbers in §4 (`measurements/rss/`) |
| A2.4 PlainEditor: caret, selection basics, masking, show/hide, IME preedit; `ime_preedit`, `ime_commit`, `text_input_rect` | met (headless API) | `src/edit.rs`; `tests/f01_trace.rs` (masking edit-in-the-middle, show/hide, zh-CN preedit→commit, candidate-area growth). Selection: Ctrl+A, caret motion, drag/double/triple-click are implemented via Parley but only Ctrl+A and Left/Home are exercised by tests |
| A2.5 F01 interpreter; submit disabled until valid; live=polite errors; effects to logging stub | met | `src/eval.rs`; `tests/f01_trace.rs`; `Runtime::effect_log` (stub only) |
| A2.6 `accesskit::TreeUpdate` 0.25.1, stable ids, incremental | met | `src/a11y.rs`; `f01_incremental_updates_are_small_and_idle_is_silent` (incremental update < half the nodes) |
| A2.7 `needs_frame()`; idle = no frame | met | asserted in `l01_layout_and_idle`, incremental test, and `frame_bench` (idle check) |
| A2.8 Per-stage timings, cheap API | met | `FrameTimings` via `Runtime::timings()`; hosts write the CSV rows (METHOD.md schema) |
| A2.9 Input API incl. `resize` | partly met | pointer/key/text/IME/Tab/a11y_action tested; **`resize()` has no dedicated test of mine** (dev-web exercises it in its DSF scenario; not re-verified here) |
| A3 Tier 0 `vello_cpu` golden reference | met | `render::render_cpu_*`; PNGs in `measurements/golden/*.tier0c-vello_cpu.png` |
| A3 GL path: `vello_gpu` on wgpu, GL only, downlevel limits, offscreen+readback | met | `src/render_gl.rs` (only wgpu feature `gles`), `new_headless()` requests `Limits::downlevel_webgl2_defaults()`; adapter reported = llvmpipe (LLVM 20.1.2), Mesa 25.2.8 (`measurements/golden/inputs/fingerprints.native.txt`) |
| A3 Web path via `vello_gpu` `webgl`, shared scene function | met | `render::replay_gpu` (feature `webgl`); used by dev-web's host and by my harness `golden-web/` |
| A7 TOLERANCE.md written and committed before any comparison | met | TOLERANCE.md sha256 `0e6f1c8a…f6f7`; Orchestrator confirmed commit d2f4f24 before the comparison ran. **Disclosure:** before it existed I looked at one L01 render from vello_cpu and one from GL by eye (no metric computed) |
| A7 golden results table + diff images, host labels | met (results are partly FAIL) | `measurements/golden/results.md`, `README.md`, `*.flip.png`, `*.absdiff_x8.png` |
| D3 identical geometry L01/F01 at 1366×768 and 360×640, native vs wasm32 | met | `measurements/d3/` (method below) |
| F01 state-trace test, four G2 A-items | met | `tests/f01_trace.rs::f01_four_checklist_items` |
| L01/F01 PNGs from all three paths | met | `measurements/golden/` (and `…/hostpage/` from dev-web's page) |
| Font/RSS numbers | met (indicative, see §4) | `measurements/rss/` |
| `cargo tree -e normal` native + wasm32, saved | met | `measurements/hardwall/` |
| CHECKPOINT.md current | met | `CHECKPOINT.md` |

**D3 method:** `d3-wasm/src/lib.rs::dump_all()` loads L01/F01 (en and zh-CN) at 1366×768 and 360×640, performs scripted input (type into email/password, IME commit into name, caret move) and dumps `id x y w h` in 1/64-px integers for every visible node, plus text line widths and the caret rect. It is built once as a native binary and once as a `wasm32-unknown-unknown` cdylib with no imports (no wasm-bindgen), executed under node. `d3-wasm/run_d3.sh` (one command) compares the two files: sha256 `03a2687f139656e0a08e77adcccfb6bc9a53f383fbd17d03cec98c237a5e4cca` for both, 332 lines. Specimens: L01.pvca `b86a4f74…7813`, F01.pvca `88dfb47c…1139`.

**Layout statement for any measurement file:** general constraint solve excluded (D-4): direct row/col/pad/gap/align/justify/fill/hug/fixed sizes with 1/64-px snapping.

## 3. Checked vs inferred
**Checked (ran it; file exists)**
- All tests green on the live dev-ir crate: `measurements/tests.log` (2 font tests, 3 F01 trace tests, 3 specimen tests).
- D3 native == wasm32 byte-identical: `measurements/d3/{geometry_native.txt,geometry_wasm32.txt,sha256.txt}`.
- Display-list fingerprints identical native vs wasm32 for the 4 states: `measurements/golden/inputs/fingerprints.native.txt`, `inputs_web/fingerprints.web.json`.
- Golden FLIP results for gl-llvmpipe and web-webgl2 (and the host-page variant for 3 states): `measurements/golden/results.{md,json}`, `hostpage/results.md`. Tier 0 and GL renders are bit-reproducible across two runs: `measurements/golden/determinism_rerun.txt`.
- Chromium 141.0.7390.37, `navigator.gpu` is `undefined`, WebGL2 renderer = ANGLE/Vulkan/SwiftShader: `inputs_web/fingerprints.web.json`.
- Pins: the resolved graph has exactly one version of each pinned crate: `measurements/pins/pins_resolved.txt`.
- Hard-wall greps over the three `cargo tree` outputs and over `pvc-runtime/src`: no match (`measurements/hardwall/`).
- Bundled font hashes equal the embedded bytes (unit test), and no system font is registered (unit test).
**Inferred (reasoned, or not run here)**
- The golden F01 p99.9 miss is caused by CPU-u8 vs GL rounding in layer/stroke compositing: inferred from which pixels differ (checked) and from the CPU/GPU agreement pattern; no renderer-internals investigation.
- RSS: the CJK face costs little because its pages are demand-paged from the binary. On wasm32 the 16 MB data segment is copied into linear memory (inferred; dev-web's measurement decides).
- Hinting flag (size ≤ 16 and scale < 1.5, G6 §4) is passed to glifo; I did not verify it matches Chromium.
- Behaviour on a real Tier 1 GPU, ChromeOS, Crostini: unverified (kept unverified).
- That vello_cpu single-thread render of a 1366×768 frame takes ~12 ms here says nothing about a Celeron N4000.

## 4. Measurements (all `host=container-xeon4`; none from a phone or Chromebook)
1. **Fonts and RSS** — `host=container-xeon4 / tier0c-vello_cpu`, headless native release, 5 fresh processes, exclusive CPU lock, `measurements/rss/rss_probe.jsonl` (loadavg at start 5.04; indicative).
   - Font bytes embedded and registered: **17,984,448** (DejaVu Sans 759,720 + Bold 708,920 + Noto Sans CJK SC Regular 16,437,340 + Bold subset 78,468). The Bold CJK face is authored-only and subset to the 143 characters of the specimens' strings (G6 §3); the Regular CJK face is not subset (user-input face).
   - RSS (median of 5, kB): start 3,432 → fonts registered 5,656 (+2,224) → first Latin layout 6,336 → **first CJK layout 7,208 (+872)** → F01 loaded + first frame 7,888 → after one vello_cpu 1366×768 render 19,352; peak VmHWM ≤ 23,648. Binary 24,069,952 bytes.
   - Reading: native RSS counts touched pages only, so the 16.4 MB CJK file contributes ≈0.9 MB RSS (first CJK layout) plus its share of the +2.2 MB registration. This is *not* its cost on wasm or in a download (that is the full 16.4 MB; G6 risk 6).
2. **Headless frame stages** — `host=container-xeon4 / tier0c-vello_cpu`, no window, no present, no input-event path; `measurements/frame_bench/{summary.jsonl,frames.csv}`, exclusive lock **but loadavg 5.39 at start** (builds had just finished). General constraint solve excluded. 200 typing frames at 1366×768, DSF 1.0:
   - F01 typing (en): total p50 12.1 ms, p95 17.4 ms, max 19.2 ms; update p50 0.008, layout 0.000 (typing does not change the tree layout), scene 0.027, a11y 0.026, **render (vello_cpu + RGBA convert) p50 12.06 ms**. First frame (load excluded) + render 13.7 ms.
   - F01 zh-CN IME preedit/commit: total p50 12.0, p95 14.3, max 18.5 ms.
   - Runtime stages before rendering are ≈0.06 ms; the rest is whole-frame vello_cpu rendering (no damage rectangles: every frame repaints everything). Against the 16.6 ms budget these are *not* host results; dev-measure's A6 numbers supersede them.
   - Binaries frozen for dev-measure: `bin/frame_bench` a213611a…22b4, `bin/rss_probe` 188cd235…b4e6 (the numbers above came from earlier builds: sha 053fb5d5… and fa31100d…).
3. **Golden FLIP** — table in `measurements/golden/results.md` (mean / p99.9 / max / pixels not identical / max channel diff):

| state | path (host label) | mean | p99.9 | max | verdict vs TOLERANCE.md |
|---|---|---|---|---|---|
| L01 idle | gl-llvmpipe | 0.000126 | 0.01866 | 0.0324 | PASS |
| L01 idle | web-webgl2 (chromium141-webgl2-swiftshader) | 0.000110 | 0.01782 | 0.0301 | PASS |
| F01 empty | gl-llvmpipe | 0.000365 | 0.05292 | 0.0559 | **FAIL** (p99.9) |
| F01 empty | web-webgl2 | 0.000363 | 0.05329 | 0.0555 | **FAIL** (p99.9) |
| F01 error | gl-llvmpipe | 0.000421 | 0.05326 | 0.0555 | **FAIL** (p99.9) |
| F01 error | web-webgl2 | 0.000408 | 0.05326 | 0.0555 | **FAIL** (p99.9) |
| F01 zh preedit | gl-llvmpipe | 0.000335 | 0.05255 | 0.0559 | **FAIL** (p99.9) |
| F01 zh preedit | web-webgl2 | 0.000319 | 0.05275 | 0.0555 | **FAIL** (p99.9) |

   Reference: Tier 0 `vello_cpu`. Bound: mean ≤ 0.005, p99.9 ≤ 0.05, max ≤ 0.25 (set before results). Each path measured once per state (renders were bit-reproducible, so repetition adds nothing).

## 5. Deviations and labelled gaps
- **No physical Tier 1 device** (V1). GL = llvmpipe with imposed downlevel limits; "WebGL2" = ANGLE→SwiftShader (V2).
- **Golden: FAIL on the p99.9 bound for F01 states, both paths.** The tolerance was not widened. Post-hoc remark only: a p99.9 bound of about 0.06 would pass every state; that is a proposal for the Orchestrator, not a verdict change.
- The web golden states were produced by the same runtime API calls compiled to wasm32 (my harness `golden-web/`), not by typing through dev-web's overlay/CDP IME; dev-web's host page was used for a cross-check of three states (its zh-CN script differs, so that state is void there, per TOLERANCE §6.3).
- Golden only at DSF 1.0 and 1366×768; no Firefox, no real GPU.
- Runtime gaps: no caret blink (timer frames would break 0-FPS idle); no clipboard (host copies `selected_text()`); IME preedit ignored while a field is masked; AccessKit text-run children / text selection not exposed (inputs expose `value` only; password exposes bullets); `parley/complex-scripts` off; no scroll beyond single-line horizontal scroll in inputs; **no hover state** (pointer-move produces no frame, as dev-web observed — by design here, not a defect to paper over); every frame repaints the whole surface (no damage rectangles).
- Disabled controls draw in a 0.4-opacity layer; this rule comes from dev-ir's CHECKPOINT and is not in G4 (see §7).
- Constructs beyond L01/F01 are refused at load (`LoadError::Unsupported`, `src/support.rs`). The pilot extension (B4-runtime) is not started.
- Latin face is DejaVu Sans (what dev-ir's manifest pinned), not a Noto Sans face. Honest but note for A8: text-bar comparisons use these exact files.
- Hosts' frame instrumentation rows (CSV/`__pvcFrames`) are written by the hosts, not by this crate.

## 6. Dependencies
- **Cargo additions outside the G7 table in `pvc-runtime`: none** (no RFC needed). Pinned and used: parley =0.11.1 (default-features off, `std`), fontique =0.11.1 (off, `std`), harfrust ~0.12.0 (0.12.0), skrifa ~0.44.0 (0.44.0), accesskit =0.25.1, glifo =0.4.0, vello_cpu =0.3.0, vello_gpu =0.3.0 (features `text` + `wgpu` or `webgl`), wgpu =30.0.1 (features `std`,`wgsl`,`gles`). Path dependency: `pvc-ir` (dev-ir). Dev-dependency (test only): `sha2` 0.10.x (0.10.9 in the lock).
- Harness crates (not shipped, listed per COMMON §3): `d3-wasm` (pvc-runtime, pvc-ir); `golden-web` adds `wasm-bindgen =0.2.129`, `js-sys =0.3.106`, `web-sys =0.3.106` (the same versions D-10 pre-approved for pvc-web).
- Tooling/installs: pip in `.venv`: `flip-evaluator` 1.7, `numpy` 2.4.6, `pillow` 12.3.0; system python `fontTools` 4.64.0 (already installed; used to extract the SC face from the Noto TTCs and subset the Bold); `wasm-bindgen` CLI 0.2.129 (dev-web's copy at `/workspace/dev_work/dev-web/tools/bin`, used read-only); Playwright/Chromium 141.0.7390.37 from `/opt/pw-browsers`; node 22. No apt installs.
- Font files embedded: DejaVu Sans / Bold (system copies, hashes match dev-ir's manifest), Noto Sans CJK SC Regular (extracted, unmodified glyphs), Noto Sans CJK SC Bold subset (143 chars; `fonts/NotoSansCJKsc-Bold.subset.chars.txt`). Licences copied in `pvc-runtime/fonts/LICENSE-*.txt`.
- Build targets: `CARGO_TARGET_DIR=/workspace/cargo-target` (native), `/workspace/cargo-target-wasm`.

## 7. Inconsistencies flagged
- G7/COMMON say Latin face + Noto Sans CJK SC "bundled"; dev-ir's manifests pinned DejaVu (system files) and TTC CJK. dev-ir re-hashed to my CJK files at my request; fine, but the specimens' font identity now depends on my extraction/subset of Noto (documented in CHECKPOINT).
- dev-ir's "disabled ⇒ 40 % opacity" is a runtime rule with no counterpart in G4 §12 (G4 only says a degradation must not change meaning). I implemented it as a rule from dev-ir's CHECKPOINT; G4 should state it, or the validator should carry it.
- G6 §4 says hinting at ≤16 px when scale < 1.5, but the Parley/glifo pinned stack exposes only a boolean vertical-hint flag per run; "font hinting" fidelity versus Chromium is unmeasured.
- A real finding for G6/G4 §6.4: the "CPU and GPU compute the same function" guarantee shows up as a systematic 1/255 compositing-rounding difference in flat areas (not just AA edges) once opacity layers or strokes are used. FLIP's p99.9 on large flat light regions weights it at ≈0.053. Any future bound should be set with this in mind.
- dev-native's Cargo.toml still points at my deleted-soon snapshot `scaffold/pvc-ir-a1` (dev-web already switched). Snapshot stays until dev-native moves to the live path.
- Process: per-Developer cargo target dirs filled the container disk once and blocked my shell for a while; no data was lost and the A7 ordering (TOLERANCE committed before comparison) was not affected.

## 8. Open issues
1. Golden: decide whether to accept the F01 p99.9 miss as a labelled finding or commission a renderer-side fix/bound change (a post-hoc proposal, not mine to apply).
2. `resize()` and DSF ≠ 1 golden are untested by me.
3. Frame numbers need dev-measure's exclusive-window values; mine ran at load 5.4.
4. B4-runtime (headless layout + behaviour for the pilot constructs: grid, breakpoints, slot, view, layer, decimals, lists) is not started; the extension point is `src/support.rs` (gate), `layout.rs`, `eval.rs`.
5. Delete `scaffold/pvc-ir-a1` once dev-native switches path.
6. Possible follow-ups with evidence value: a Firefox/DSF-2 golden run; clipboard and AccessKit text-run exposure for inputs (native a11y fidelity for editing).
