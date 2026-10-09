# Brief: dev-web — web host: wasm32 + WebGL2, DOM/ARIA semantics mirror, input overlay (A5)

**Your name / directory:** `dev-web` → `/workspace/dev_work/dev-web/`
**Read first:** `/workspace/orchestrator_work/pm/briefs/COMMON.md` (mandatory); `docs/G4_PVC_IR_v0_Spec.md` §8 and §8.3; `docs/G6_Text_Strategy.md` §6; `docs/G7_Dependency_Pins_and_Findings_Recheck.md` S.1 and S.3; FINDINGS F5, F6, F11 and F12; and `/workspace/orchestrator_work/pm/feasibility/webgl2_probe.mjs`, a working Playwright WebGL2 probe that hides `navigator.gpu`. Code against `/workspace/dev_work/dev-render/CHECKPOINT.md` once it exists. Scaffold before then.
**Serves:** Q3 (mirror per-frame cost), Q2 (web IME path), Q4 (web frame budget).

## What to build: crate `pvc_web` at `/workspace/dev_work/dev-web/pvc-web/` + a static page
1. **wasm32 build:**
   - `wasm32-unknown-unknown`, `vello_gpu` with `default-features = false, features = ["webgl", "text", "std"]`, and `pvc_runtime` + `pvc_ir`;
   - direct deps `wasm-bindgen =0.2.129`, `web-sys =0.3.106`, `js-sys =0.3.106` (pre-approved; RFC notes required);
   - install `wasm-bindgen-cli` at exactly `0.2.129` (`cargo install wasm-bindgen-cli --version 0.2.129 --root /workspace/dev_work/dev-web/tools`).
   
   The page loads the **signed** L01/F01 artifacts through `pvc_ir::load_signed`.
2. **WebGL2 only:**
   - the page asserts `navigator.gpu === undefined`, via an init script that hides it, as in the probe;
   - it asserts that a `webgl2` context was obtained;
   - it records the `UNMASKED_RENDERER` string in every run's output.
   
   Rendering is requestAnimationFrame-driven **only when `needs_frame()`**, so idle is 0 frames.
3. **Semantics mirror (on by default, G4 §8.3):** a hidden, absolutely positioned DOM tree with ARIA roles, names and states, generated from `pvc_runtime`'s `accesskit::TreeUpdate`, applied incrementally. Map with the G4 §8.1 ARIA column. Time the mirror update separately every frame (`mirror_ms`). This number is Q3.
   - **Done** = Playwright's Chromium accessibility tree (CDP `Accessibility.getFullAXTree` or `page.accessibility.snapshot`) shows every F01 interactive element with the correct role, name and state. A checkbox toggle and an error appearing are reflected. Save the snapshots.
4. **Text-input overlay (G6 §6, Flutter precedent G7 S.3):** a real `<input>` (type text/email/password) pixel-aligned over each F01 field, using `text_input_rect`. Keyboard input and `compositionstart/update/end` feed `pvc_runtime` (preedit/commit). The canvas draws the text. The overlay *is* the accessible text field the mirror points at.
   - **IME test:** drive Chromium's IME composition pipeline with CDP `Input.imeSetComposition` (preedit "ni", "nih", …) and then `Input.insertText("你好")`. Assert the preedit was drawn (screenshot) and the commit landed in runtime state and in the AX tree.
   - **Label it exactly:** "browser IME composition pipeline driven via CDP; no OS IME engine involved". Do not call it system IME.
   - Check overlay/canvas caret alignment at device scale factors 1.0, 1.25, 1.5 and 2.0 (G6 risk 5). Record the pixel offsets.
5. **Instrumentation:**
   - push per-frame rows (stage timings from `pvc_runtime` plus `mirror_ms`, plus total rAF-callback work) to `window.__pvcFrames`, using `/workspace/dev_work/dev-measure/METHOD.md`'s schema;
   - frames-presented counter;
   - first-frame time from navigation start.
   
   Provide Playwright scripted-input scenarios: pointer move over the button, Tab traversal, typing 20 Latin characters, the IME sequence, show/hide, checkbox. dev-measure will call them for the measured runs.
6. **Hard wall:** `pvc_web` makes no fetch or XHR at runtime other than loading its own static assets (wasm, artifact, fonts) from the same origin. Assert it in Playwright by logging every request and saving the list.

## Done means
Items 1–6. Plus:
- a one-command `run_web.sh <scenario>` (serves locally, runs Playwright with `PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers`);
- `cargo tree -e normal --target wasm32-unknown-unknown` saved, with no HTTP/LLM/net crate;
- `REPORT.md`, with every number labelled `chromium141-webgl2-swiftshader`. This is CPU-emulated WebGL2, not a Tier 1 GPU.

## Files you may touch
Only `/workspace/dev_work/dev-web/**`. npm packages, if any, are local to your directory. Playwright's Chromium is at `/opt/pw-browsers` and the global `playwright` module at `/opt/node22/lib/node_modules/playwright`.
