# CHECKPOINT — dev-web (`pvc_web`)

Crate `/workspace/dev_work/dev-web/pvc-web/` (cdylib, wasm32-unknown-unknown) + static page `/workspace/dev_work/dev-web/www/`.
Status 2026-10-09: **real** — runs against the live `pvc_runtime` facade (dev-render) and the live `pvc_ir` (dev-ir) in headless Chromium 141 with `navigator.gpu` hidden, WebGL2 via ANGLE→SwiftShader (**CPU emulation, not a GPU**).
Host label for every number: `host=container-xeon4 path=chromium141-webgl2-swiftshader`. **NOT a Tier 1 device result.** General constraint solve excluded (D-4).

## Build / run (one command each)
```
/workspace/dev_work/dev-web/build.sh                      # cargo (CARGO_TARGET_DIR=/workspace/cargo-target-wasm, shared CPU lock) + wasm-bindgen 0.2.129 -> www/pkg
/workspace/dev_work/dev-web/run_web.sh <scenario> [--variant raf|sync] [--mirror on|off] [--dsf N] [--specimen L01|F01] [--locale en|zh-CN] [--out DIR]
PVC_LOCK=x /workspace/dev_work/dev-web/run_web.sh ...     # exclusive CPU lock (timing runs); default is shared
```
Scenarios (`tests/scenarios.mjs`): `smoke a11y tab type-latin ime toggle-show checkbox ptr-move idle dsf golden selection tamper perf`. Output: `out/<scenario>/` (`result.json`, `run_meta.json` with host label + loadavg + command, `requests.json` = every request the page made + hard-wall audit).
`tests/lib.mjs` exports `open({dsf,variant,mirror,specimen,locale,scenario,...})`, `rects, frames, meta, state, settle, axTree, axCompact` — dev-measure may import it; `tests/scenarios.mjs` exports `SCENARIOS` and `GAP_MS=50` (METHOD §2.1 gap).

## Page contract (dev-measure METHOD.md §11 web hooks — all implemented)
`index.html?scenario=<id>&variant=raf|sync&mirror=on|off&specimen=L01|F01&locale=en|zh-CN[&artifact=<file>][&debug=1]`
* `window.__pvcFrames`: array of objects with the 12 METHOD §3 keys (`frame_idx,t_ns,scenario,event,update_ms,layout_ms,scene_ms,a11y_ms,mirror_ms,render_ms,total_ms,completion_ms`) + extra columns (`overlay_only_ms`, `mirror_nodes_in_update`, `mirror_nodes_changed`, `mirror_dom_writes`, `mirror_elements`). `null` = stage does not exist (e.g. `mirror_ms` with `mirror=off`, `completion_ms` in `raf`).
  * `t_ns = round(t_recv_ms*1e6)` (`performance.now()`), `total_ms` = event-handler work + rAF-callback work (waiting excluded), `completion_ms` (variant=sync) = total + 1×1 `readPixels`, `mirror_ms` = DOM mutation + one forced `getBoundingClientRect()` flush of the mirror subtree, `update_ms` = DOM-handler work + runtime update. `event` = class of the last *real* input in the batch (focus/selection/resync bookkeeping events never relabel it).
  * `mirror=off` keeps the `<input>` overlays (needed for typing/IME regardless of the mirror) and drops every other ARIA element; its cost is reported in `overlay_only_ms`, not `mirror_ms`.
* `window.__pvcMeta`: `frames_presented, events_received, renderer (UNMASKED_RENDERER), backend, limits, mirror, variant, navigator_gpu ("undefined"), webgl2, perf_caveat_bypass, init_ms, shader_compile_ms, wasm_init_ms, artifact, dpr, …`
* `window.__pvcRects`: `{alias: [x,y,w,h]}` CSS px (F01 `name,email,password,toggle,checkbox,submit`; L01 `connect_button,bio_text`; plus all IR ids). `window.__pvcRectsRefresh()` re-reads.
* `window.__pvcReady = true` after the first presented frame. **Idle = 0 frames**: rAF is requested only from input events and only when `needs_frame()` (checked: `out/idle`).
* Page asserts `navigator.gpu === undefined` (hidden by the Playwright init script `HIDE_GPU`) and refuses to boot otherwise; asserts a webgl2 context; records the renderer string.

## Hooks for tests / golden images (`window.__pvc` = the `PvcWeb` object)
`focus(alias|irId)`, `text(s)`, `ime_preedit(s, cursorStart, cursorEnd)` (-1,-1 = none), `ime_commit(s)`, `pointer('move'|'down'|'up', x, y)`, `key(name, shift, ctrl, alt)`, `tab(shift)` (true = focus stayed inside the app; false → browser default runs, no keyboard trap), `resize(w,h,dpr)`, `debug_state()` (JSON; `raw` = runtime `debug_state`, `focused_input` = {value, composing, selection}), `caret_rect(nodeIdDecimalString)`, `rects_json()`, `golden()`.
`await window.__pvcGoldenCapture()` → `window.__pvcGolden = {rgba_png_base64, dl_fingerprint, width, height}`: forced frame, pixels by `gl.readPixels` in the same task as the render (not a screenshot), rows flipped to top-left. Script: `run_web.sh golden` → `out/golden/*.png` + `golden.json`.

## Not yet / limits
No real-hardware GPU, no OS IME, no screen reader (Chromium AX tree via CDP only), no Firefox/Safari. Headless Chromium paints no native text caret in screenshots (checked, `out/dsf/dsf_table.json:caretPaintCheck`), so native caret x is derived from DOM text metrics. wasm is 21 MB because the runtime `include_bytes!`es the fonts (CJK 16 MB).
