# Brief: dev-render — runtime core, renderers, golden parity (A2, A3, A7); later the headless checker API (B4-runtime)

**Your name / directory:** `dev-render` → `/workspace/dev_work/dev-render/`
**Read first:** `/workspace/orchestrator_work/pm/briefs/COMMON.md` (mandatory), then:
- analysis `docs/PVC_Adversarial_Analysis_and_Architecture.md` §2.3, §4.3, §4.4.1;
- `docs/G4_PVC_IR_v0_Spec.md` §4, §5, §6, §7, §8, §12, §14;
- `docs/G6_Text_Strategy.md` (all);
- `docs/G7_Dependency_Pins_and_Findings_Recheck.md` §2;
- dev-ir's `/workspace/dev_work/dev-ir/CHECKPOINT.md` and `GRAMMAR.md` once they exist. Until then, start with the internals below.

**Serves:** Q2 (text), Q4 (frame budget with real IR, text and a11y together).

## A2 — `pvc_runtime` core (`/workspace/dev_work/dev-render/pvc-runtime/`)
Scope is **only what L01 and F01 need** (PLAN §2). Not in scope: scroll, view, slot, layer, image, gradients, shadows, animation, raw constraints, general constraint solver.

1. **Load:** take a `pvc_ir::Module` that has come through `load_signed`. The runtime refuses unsigned input except in a test-only path.
2. **Layout:** a direct sugar solver for row/col/pad/gap/align/fill/fixed sizes, snapped to 1/64 logical px (G4 §5.1). This is Supervisor decision D-4: no Cassowary. Put a sentence in your REPORT and CHECKPOINT so measurement files can state "general constraint solve excluded".
3. **Text:**
   - Parley 0.11.1 with **bundled, content-hashed fonts**: a Latin face plus Noto Sans CJK SC, from `/usr/share/fonts/opentype/noto/` or a copy in your directory. Record the file, its sha256 and its size. Set Fontique so system fonts are not silently used for authored text (G6 §3).
   - Glyphs go through the pinned glyph path (`glifo` 0.4.0 / vello `glyph_run`).
   - Measure and report font bytes loaded and the CJK face's contribution to RSS (G6 risk 6).
4. **Editing:** F01 fields use Parley's `PlainEditor`: caret, selection basics, password masking, show/hide toggle, and **IME preedit** (set/clear composition, commit). Expose `ime_preedit(text, cursor)`, `ime_commit(text)` and `text_input_rect(node) -> Rect` so hosts can place the candidate window or the web overlay.
5. **Behaviour:** an F01 interpreter for typed state, change/press/submit, guards, and validity. Submit stays disabled until all fields are valid. Errors carry `live=polite`. Effects go to a **logging stub dispatcher, never a network**.
6. **Semantics:** build `accesskit::TreeUpdate` (0.25.1) from IR roles, using the G4 §8.1 mapping, with stable node IDs. Send incremental updates on change only. Native and web both consume this. There is no second semantics model.
7. **Damage/idle:** `needs_frame() -> bool`. With no input and no pending change, the runtime asks for **no frame**. That is how hosts get 0 FPS idle.
8. **Instrumentation:** per-stage timings for the last frame (update, layout, scene, a11y, render, total) via a cheap API. The CSV schema comes from dev-measure's `METHOD.md` (`/workspace/dev_work/dev-measure/METHOD.md`); follow it once it exists.
9. **Input API** that hosts call: pointer move/down/up, key, text, IME, focus traversal (Tab), and `resize`.

## A3 — renderers
- **Tier 0 (c):** `vello_cpu` 0.3.0 → RGBA8 pixmap. This is also the golden reference.
- **GL path:** `vello_gpu` 0.3.0 on `wgpu` 30.0.1, **GL backend only**, with `wgpu::Limits::downlevel_webgl2_defaults()` requested, so llvmpipe's compute is not relied on (FINDINGS F8). Offscreen render-to-texture plus readback for tests. Hosts present it to a surface.
- **Web:** `vello_gpu` with feature `webgl` (no wgpu). Coordinate with dev-web: you provide a scene-building function they can call with their WebGL2 renderer, or a thin wrapper. Agree on it in your CHECKPOINT.
- Scene building is shared, so all three paths draw the same scene from the same layout.

## A7 — golden-image parity (the order matters: the Supervisor checks it)
1. **Before any comparison**, write `/workspace/dev_work/dev-render/TOLERANCE.md`:
   - metric: LDR-FLIP via `flip-evaluator` (pip, in your venv) — mean, plus a stated bound on max or a high percentile;
   - per path: gl-llvmpipe and web-webgl2, each vs Tier 0 vello_cpu;
   - states compared: L01 idle, F01 empty, F01 with an error shown, F01 with zh-CN text and IME preedit;
   - rationale, and what happens on failure.
   
   Ask dev-measure to review it. That review is a note in their REPORT; you don't need to wait for it. Then **stop and send the Supervisor "TOLERANCE ready: <path> sha256 <hash>"**. The Orchestrator will commit it to the integration branch. **Do not run any comparison until the Supervisor relays "tolerance committed".**
2. Then run the comparisons. Write a results table and diff images under `/workspace/dev_work/dev-render/measurements/golden/`, with the host labels from COMMON §5.

## Done means
- D3 test: identical solved geometry for L01/F01 at 1366×768 and 360×640, native vs wasm32. Record the method.
- F01 state-trace test passes F01's four G2 checklist items headlessly: submit disabled until valid; email validated; show/hide toggles masking and is named; errors announced (live).
- L01/F01 PNGs from all three paths (web via dev-web's page, or your own wasm test page under Playwright Chromium at `/opt/pw-browsers`, `PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers`).
- `TOLERANCE.md` was committed before the results existed.
- Golden results table.
- Font/RSS numbers.
- `cargo tree -e normal` (native + wasm32) shows no HTTP/LLM/net crate. Saved.
- `CHECKPOINT.md` is kept current.
- `REPORT.md`.

## Later (second brief, not now)
After A is reported, the Orchestrator may ask you to extend the **headless** layout and behaviour execution to the pilot specimens' constructs (D-2), so the fidelity checker can query geometry, the a11y tree and state traces. Don't start that now. Do keep layout and behaviour separable from rendering.

## Files you may touch
Only `/workspace/dev_work/dev-render/**`. pip installs go in your own venv.
