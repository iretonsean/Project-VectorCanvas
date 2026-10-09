# Brief: dev-measure — measurement method, harness and runs (A6); reduced G6 text bar (A8); tolerance review (A7)

**Your name / directory:** `dev-measure` → `/workspace/dev_work/dev-measure/`
**Read first:** `/workspace/orchestrator_work/pm/briefs/COMMON.md` (mandatory, especially §4 CPU lock and §5 host labels); analysis §2.3, §4.3 and §4.4.1 (budgets and rules 1–4); `docs/G4_PVC_IR_v0_Spec.md` §8.3 and §14; `docs/G6_Text_Strategy.md` §4, §5 and §9; and FINDINGS (all).
**Serves:** Q3 (mirror per-frame cost), Q4 (frame budget), Q2 (text legibility).

## A6, step 1 — `METHOD.md` first, before any number exists
Write `/workspace/dev_work/dev-measure/METHOD.md` **as your first deliverable**. The hosts implement your schema, so publish it early and announce it in `CHECKPOINT.md`. It must define:
- **Interaction frame:**
  - which interactions: pointer move over the L01 button, Tab focus traversal through F01, typing 20 Latin characters into F01 name, the zh-CN IME composition+commit, the show/hide toggle, the checkbox (§4.4.1 rule 2);
  - exactly where timing starts and stops on each path. For native: from event receipt through update/layout/scene/a11y/render/submit, *GPU completion* (`device.poll(wait)` or equivalent) and present. For web: rAF callback work, *and* a completion-forcing variant (e.g. a 1-pixel `readPixels`). Label both; SwiftShader completes asynchronously.
- **CSV schema** for per-frame rows: `frame_idx, t_ns, scenario, event, update_ms, layout_ms, scene_ms, a11y_ms, mirror_ms (web only), render_ms, total_ms, completion_ms`.
- **Idle:** frames presented in a 5 s window with no input after a 1 s settle. The pass value is exactly 0.
- **First frame:** process start (native) or navigation start (web) to first present. Budgets: <1 s (Tier 1) and <1.5 s (Tier 0) per §4.4.1.
- **RSS:**
  - native: peak `VmHWM` of the process;
  - web: sum of RSS of the Chromium renderer and GPU processes from /proc at the end of the run (method labelled);
  - budget 150 MB (§4.4.1 rule 4). Say exactly what is included.
- **Statistics:** warm-up frames discarded (state how many); repetitions; p50/p95/p99/max.
- **Pass/fail:** against 16.6 ms (Tier 1 paths) and 33 ms (Tier 0) **on this host only**.
- **Required labels:** the host label (COMMON §5), "general constraint solve excluded (D-4)", the commit or sha256 of the artifacts and binaries, `/proc/loadavg` at start, and the command line.

## A6, step 2 — harness and runs (after dev-native/dev-web/dev-render report their hooks)
- `tools/measure/run.sh <path> <specimen> <scenario>` and `run_all.sh`. One command must reproduce any single number, because the **Supervisor will re-run one measurement themselves**. Every measured run is wrapped in `flock -x /workspace/orchestrator_work/pm/CPU_LOCK`.
- Paths: `tier0c-vello_cpu`, `gl-llvmpipe` (via `pvc_native`), `chromium141-webgl2-swiftshader` (via `pvc_web`). For Tier 0 windowed vs offscreen, state which you measured. Offscreen vello_cpu frame time plus the same runtime stages is acceptable if labelled.
- Outputs go to `/workspace/dev_work/dev-measure/measurements/session2/<path>/<specimen>/<scenario>/`: raw CSV, `summary.json`, and `summary.md` with the budget comparison.
- A top-level `measurements/session2/SUMMARY.md` table: path × specimen × scenario, with p50/p95/p99/max, idle frames, first frame, RSS, mirror_ms p95 and pass/fail. **Each cell links its file.**
- Q3: report `mirror_ms` both as an absolute number and as a share of the web frame.

## A8 — reduced G6 text bar (Latin + zh-CN)
Compare PVC's text rendering (vello_cpu path, through dev-render's text API) with Chromium 141. Use Firefox 142 vs Chromium 141 as the G6 §5 relative threshold.
- Install Firefox with `PLAYWRIGHT_BROWSERS_PATH=/workspace/dev_work/dev-measure/pw playwright install firefox`. Install `flip-evaluator` in your venv.
- **Same font file** for all three: the bundled faces dev-render uses, served to the browsers via `@font-face`.
- Strings: the L01/F01 strings, the F01 zh-CN strings, plus a short G6 §5 Latin and Simplified Chinese corpus. Write your corpus file **before** measuring.
- Sizes 11, 13, 16 and 24 px; device scale 1.0 and 2.0. Same canvas size and background.
- Measure, per script × size × scale:
  1. mean LDR-FLIP PVC↔Chromium;
  2. mean LDR-FLIP Firefox↔Chromium;
  3. pass if (1) ≤ (2);
  4. line-break equality at a fixed width (PVC from Parley layout, Chromium from Range rects). Must match exactly.
- Write `measurements/session2/text-bar/RESULT.md`. Labelled gaps: no human legibility study; Arabic, Devanagari and emoji not run; Linux rasterisers only; the G6 §5 caveat that Chromium and Firefox may share rasterisation code (*unverified*).

## A7 — tolerance review
When dev-render writes `/workspace/dev_work/dev-render/TOLERANCE.md`, review it: is the metric defined, are the thresholds justified, and does it say what happens on failure? Put your review in your REPORT. You don't block them.

## Done means
- `METHOD.md` was published before any measurement file. Your earliest file timestamps must show it.
- The harness reproduces any single number with one command.
- All path × specimen × scenario cells are either measured or explicitly marked "not measurable here", with the reason.
- `SUMMARY.md` exists.
- The text-bar `RESULT.md` exists.
- `REPORT.md`, with checked and inferred kept separate. No number is presented as a Tier 1 device result.

## Files you may touch
Only `/workspace/dev_work/dev-measure/**` (Firefox and the venv go inside it).
