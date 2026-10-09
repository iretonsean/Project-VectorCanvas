# CHECKPOINT — dev-render (pvc_runtime)

Crate: `/workspace/dev_work/dev-render/pvc-runtime/` (lib `pvc_runtime`). Depends read-only on `pvc-ir = { path = "/workspace/dev_work/dev-ir/pvc-ir" }`.
**Status 2026-10-08 (later): the facade below is REAL (compiles, loads the signed L01/F01 specimens, renders them on vello_cpu and on vello_gpu/wgpu-GL-llvmpipe).** Layout statement for every measurement file: **general constraint solve excluded** (D-4): direct row/col/pad/gap/align/justify/fill/hug/fixed sizes, 1/64-px snapping.

## Cargo
```toml
pvc-runtime = { path = "/workspace/dev_work/dev-render/pvc-runtime", features = ["cpu"] }          # Tier 0 c, golden reference
pvc-runtime = { path = "...", features = ["gl"] }        # + vello_gpu on wgpu 30.0.1, GL backend ONLY (only wgpu feature `gles`)
pvc-runtime = { path = "...", features = ["webgl"] }     # wasm32: vello_gpu `webgl` (no wgpu); web host owns canvas + WebGlRenderer
# test-only: feature "test-unsigned" enables Runtime::new_unsigned_for_test
```
Parley/Fontique have the `system` feature OFF and `system_fonts:false`: **no system font can be used**.

## Facade (all `pvc_runtime::`)
```rust
Options { locale: String /* "en" | "zh-CN", must be in manifest */, width: f32, height: f32 /*logical px*/, scale: f32 }
Runtime::from_signed(bytes, |b| pvc_ir::load_signed(b, &key), Options) -> Result<Runtime, LoadError>   // the only production constructor
// input (logical px)
rt.resize(w, h, scale)
rt.pointer_down(x,y) / pointer_up(x,y) / pointer_move(x,y)      // click activates press; click in input places caret; drag selects; dbl-click word; triple-click all
rt.key(Key, Mods{shift,ctrl,alt})   // Key::{Tab,Enter,Backspace,Delete,Left,Right,Up,Down,Home,End,Escape,Char(c)}; Tab/Shift-Tab = focus traversal; Enter in an input raises Submit on nearest ancestor with a Submit handler; Enter on button = press; Ctrl+A select all
rt.text(&str)                        // committed printable text (host sends it for every text-producing key). On a focused non-input a single " " = Space activation. Host must NOT also call key() for the same printable char.
rt.ime_preedit(&str, Option<(usize,usize)>)   // winit Ime::Preedit; "" clears; byte range within the preedit; ignored while the field is masked (labelled gap)
rt.ime_commit(&str)                  // winit Ime::Commit (replaces preedit)
rt.focus_next(back: bool) -> bool    // false = focus left the app (host may hand focus to the browser/OS)
rt.focus_id(&str) ; rt.a11y_action(&accesskit::ActionRequest)   // Click / Focus / SetValue
rt.set_selection(id, start_utf16, end_utf16)                    // unit = UTF-16 code units into the real value (= DOM selectionStart/End)
// frame loop
rt.needs_frame() -> bool             // false when idle (no input, no pending change) => host requests NO frame (0 FPS idle)
rt.frame() -> &DisplayList           // runs only dirty stages: update -> layout -> scene -> a11y
rt.take_tree_update() -> Option<accesskit::TreeUpdate>   // incremental; first one carries TreeInfo and all nodes
rt.full_tree() -> TreeUpdate         // for AccessKit ActivationHandler / a freshly attached mirror
rt.timings() -> FrameTimings{update_ms,layout_ms,scene_ms,a11y_ms,render_ms,total_ms}; rt.record_render_ms(ms)  // host calls after drawing
pvc_runtime::timing::set_time_source(fn()->f64)   // REQUIRED on wasm32 (Instant panics there): pass performance.now
// queries
rt.named_rects() -> Vec<(String,[f32;4])>   // [x,y,w,h] logical px, visible nodes: full IR ids AND aliases name,email,password,toggle,checkbox,submit (F01) / connect_button,bio_text (L01)
rt.rect_of(id_or_alias) ; rt.text_input_rect(id) ; rt.focused_text_input_rect() -> Option<Rect> (caret/preedit area, absolute logical px, for set_ime_cursor_area)
rt.text_input_info(id) -> TextInputInfo{rect,font_family,font_size,line_height,text_origin_x,text_origin_y,value,masked,selection_utf16,composing}   // web overlay
rt.caret_rect(id) -> Option<Rect> ; rt.selected_text() ; rt.focused_id()
rt.debug_state() -> String ; rt.effect_log() -> &[EffectRecord{seq,effect,capability,permitted}]   // dispatcher is a LOGGING STUB, no network
rt.geometry_dump() -> String        // D3 text: "id x y w h" in 1/64 px ints + text line widths
rt.node_id(&str)->accesskit::NodeId ; rt.ir_id_of(NodeId)->Option<&str>     // NodeId = FNV-1a-64(IR id)
```
### Accessibility (single semantics model)
`accesskit::TreeUpdate` (0.25.1) per G4 §8.1. **Node `bounds` are LOGICAL px** in surface coordinates (native host at DSF≠1 multiplies by scale; web mirror uses them as CSS px directly). `author_id` = alias (above) else IR id. Decorative nodes and children of button/checkbox/switch/link are not in the tree. **Nodes with `live != off` are present from the first tree even while hidden (empty label, no bounds)** so the error/status live regions exist before content appears. Label role → text in `value`; other roles → `label`. Password input exposes bullets as `value`, never the secret. Checkbox/`pressed` → `toggled`. Error relation → `error_message`.

### Rendering (all three paths replay ONE display list)
`rt.frame()` gives `&DisplayList` (logical px ops: FillRRect, StrokeRRect, FillCircle, PushClip/PopClip, PushOpacity/PopLayer, Glyphs). Disabled controls draw in a 0.4-opacity layer (runtime rule from dev-ir).
- Tier 0 c: `render::render_cpu_rgba(&dl)`, `render_cpu_png(&dl)` (feature `cpu`).
- Native GL (feature `gl`): `render_gl::GlRenderer::new(&device, format, w_px, h_px)` then `.render(&dl, &device, &queue, &target_view)` — host owns the wgpu surface (**request `wgpu::Limits::downlevel_webgl2_defaults()`, GL backend only**; `render_gl::new_headless()` shows how). Recreate on resize (`dl.pixel_size()`). Test helper `render_gl::render_gl_rgba(&Headless,&dl)`.
- Web (feature `webgl`): host creates `vello_gpu::WebGlRenderer`; each frame call `render::replay_gpu(&dl, &mut vello_gpu::Scene, &mut vello_gpu::Resources)` on its Scene/Resources, then its renderer's render. (`replay_gpu` is generated for the `gl` feature today; it is also enabled under `webgl`.)

### Fonts (REAL, checked) — `pvc_runtime::fonts::BUNDLE`, files in `pvc-runtime/fonts/`
| file | bytes | sha256 | note |
|---|---|---|---|
| DejaVuSans.ttf | 759720 | ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280 | matches dev-ir manifest |
| DejaVuSans-Bold.ttf | 708920 | 5c1247acef7f2b8522a31742c76d6adcb5569bacc0be7ceaa4dc39dd252ce895 | matches dev-ir manifest |
| NotoSansCJKsc-Regular.otf | 16437340 | 6ead94f36c0f9423a88356f5a58b2b1b8180a3ff553ecab2f7a816990b306c60 | SC face extracted from NotoSansCJK-Regular.ttc idx 2, glyphs unchanged, **not subset** (user-input face, G6 §3) |
| NotoSansCJKsc-Bold.subset.otf | 78468 | a44805f531bccdd9eb28df4a5446132d88cf93326ed96b38366c45c2e4e3c5f9 | authored-only bold, **subset to the 143 characters** of L01+F01 strings (G6 §3) |
Font stack for authored text: `"DejaVu Sans, Noto Sans CJK SC"`. **dev-ir: the two CJK manifest entries currently point to the full TTCs (hashes b76b0433…, faa5f365…) which are NOT bundled → load_warnings; please re-hash those entries to the two OTF files above (`index=0`).**

### Text-bar entry point (dev-measure A8)
`render::text_display_list(text, locale, size_px, weight, max_width, scale, fg, bg, pad_px, (canvas_w,canvas_h)) -> DisplayList`, then `render_cpu_png`. Layout at scale 1.0 in logical px (integer line heights, quantised), `scale` applied as renderer transform; vertical hinting requested for size ≤ 16 and scale < 1.5 (G6 §4). Use the bundled files via `@font-face` for same-font-file browser comparisons. Latin text uses DejaVu (not a Noto Sans face).

### Known gaps (labelled)
No clipboard (host copies `selected_text()`); no caret blink (would need timer frames and break 0-FPS idle); IME preedit ignored while masked; AccessKit text-run children / text selection not exposed for inputs (value only); `parley/complex-scripts` feature off (CJK line breaking via ICU4X non-complex segmenter); no scroll/overflow beyond single-line horizontal scroll in inputs.

## Update 2026-10-09 — A2/A3/A7 state
- pvc-runtime builds against the LIVE `/workspace/dev_work/dev-ir/pvc-ir` (Part 2 AST). Specimens measured: L01.pvca sha256 `b86a4f7414a26c821f0a821c4291533ae93ef25d9110510af5bb24d936347813`, F01.pvca `88dfb47cfa84a786eeff91149c9036a5e2990c0aafe7e7283707fdb164211139` (manifest fonts = the bundle; `load_warnings` empty).
- `Runtime::from_signed` refuses constructs outside L01/F01 with `LoadError::Unsupported(reason)` (kinds shape/layer/slot/view, grid, breakpoint `at` rules, decimal/list, arithmetic, append/remove/open-url). The pilot extension (B4-runtime) will lift this gate in `src/support.rs`; layout/behaviour stay separate from rendering (`layout.rs`, `eval.rs` do not touch renderer types).
- New: `DisplayList::dump()` / `fingerprint()` (FNV-1a 64 hex) to prove every path replays the same scene.
- The snapshot crate `/workspace/dev_work/dev-render/scaffold/pvc-ir-a1` is only still referenced by dev-native's Cargo.toml; switch to the live path and I delete it.
- Golden results: `/workspace/dev_work/dev-render/measurements/golden/results.md` (L01 PASS on both paths; F01 states FAIL the pre-registered p99.9 bound by ~6 % on both paths; see REPORT).
- Web golden harness (mine, test tooling): `/workspace/dev_work/dev-render/golden-web/` (`run_golden_web.mjs`).
