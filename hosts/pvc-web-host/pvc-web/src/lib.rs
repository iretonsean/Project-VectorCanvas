//! pvc-web: PVC web host (Session 2, A5).
//!
//! wasm32 + WebGL2 (`vello_gpu` `webgl` feature, no wgpu) + hidden DOM/ARIA semantics mirror +
//! `<input>` overlay for text fields. Hard wall: this crate performs no network I/O; the only
//! fetches are the page's own static assets, issued by the JS bootstrap in `www/`.
//!
//! Frame rows are pushed to `window.__pvcFrames` in the dev-measure METHOD.md §3 schema.
mod engine;
mod mirror;
mod overlay;
mod runtime_engine;

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use accesskit::{ActionRequest, NodeId};
use vello_gpu::peniko::color::{AlphaColor, Srgb};
use vello_gpu::{RenderSettings, RenderSize, Resources, Scene, WebGlRenderer, WebGlTextureBindings};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, HtmlElement, WebGl2RenderingContext};

use engine::{Engine, PointerKind};
use mirror::{EventSink, Mirror};

fn now() -> f64 {
    web_sys::window().unwrap().performance().unwrap().now()
}

/// Events raised by DOM listeners (overlay, mirror, canvas) toward the engine.
pub enum HostEvent {
    Pointer(PointerKind, f32, f32),
    Key { key: String, shift: bool, ctrl: bool, alt: bool },
    Text(String),
    Preedit { text: String, cursor: Option<(usize, usize)> },
    Commit(String),
    Focus(NodeId),
    Action(ActionRequest),
    /// Overlay `<input>` value must be re-synchronised with the runtime after the next frame.
    ResyncInput(NodeId),
    Selection(NodeId, usize, usize),
    /// Test hook: focus by alias.
    FocusAlias(String),
}

impl HostEvent {
    /// METHOD.md §2.3 closed event vocabulary.
    fn class(&self) -> &'static str {
        match self {
            HostEvent::Pointer(PointerKind::Move, ..) => "ptr_move",
            HostEvent::Pointer(PointerKind::Down, ..) => "ptr_down",
            HostEvent::Pointer(PointerKind::Up, ..) => "ptr_up",
            HostEvent::Key { key, .. } if key == "Tab" => "key_tab",
            HostEvent::Key { .. } => "key_other",
            HostEvent::Text(_) => "key_char",
            HostEvent::Preedit { .. } => "ime_preedit",
            HostEvent::Commit(_) => "ime_commit",
            HostEvent::Focus(_) | HostEvent::Action(_) | HostEvent::ResyncInput(_) | HostEvent::Selection(..) | HostEvent::FocusAlias(_) => "other",
        }
    }
}

struct App {
    engine: Box<dyn Engine>,
    renderer: WebGlRenderer,
    resources: Resources,
    scene: Scene,
    gl: WebGl2RenderingContext,
    canvas: HtmlCanvasElement,
    mirror: Option<Mirror>,
    mirror_container: HtmlElement,
    dpr: f64,
    css_w: f64,
    css_h: f64,
    sync_variant: bool,
    scenario: String,
    // instrumentation
    frames: js_sys::Array,
    frame_idx: u32,
    events_received: u32,
    pending_handler_ms: f64,
    pending_t_recv: Option<f64>,
    pending_class: &'static str,
    resync: Vec<NodeId>,
    /// author_id -> [x,y,w,h] CSS px, maintained independently of the mirror (works with mirror=off).
    rects: std::collections::BTreeMap<String, [f64; 4]>,
    renderer_str: String,
    perf_caveat_bypass: bool,
    init_ms: f64,
    shader_ms: f64,
    artifact_nodes: Option<u32>,
    golden: Option<(u32, u32, Vec<u8>)>,
}

struct Host {
    app: RefCell<App>,
    queue: RefCell<VecDeque<HostEvent>>,
    raf_pending: Cell<bool>,
    raf_cb: RefCell<Option<Closure<dyn FnMut(f64)>>>,
}

impl EventSink for Host {
    fn push(&self, ev: HostEvent) {
        self.queue.borrow_mut().push_back(ev);
        self.pump();
    }

    /// Synchronous Tab traversal: the caller needs to know whether focus stayed inside the app.
    fn tab(&self, shift: bool) -> bool {
        self.pump();
        let t0 = now();
        let moved = match self.app.try_borrow_mut() {
            Ok(mut app) => {
                app.events_received += 1;
                if app.pending_t_recv.is_none() {
                    app.pending_t_recv = Some(t0);
                }
                app.pending_class = "key_tab";
                let m = app.engine.focus_next(shift);
                app.pending_handler_ms += now() - t0;
                m
            }
            Err(_) => false,
        };
        self.request_frame();
        moved
    }
}

impl Host {
    /// Drain queued events into the engine. Re-entrancy safe: if the app is already borrowed
    /// (an event fired from inside a frame), the events stay queued and the outer caller drains.
    fn pump(&self) {
        let mut any = false;
        loop {
            let ev = match self.queue.borrow_mut().pop_front() {
                Some(e) => e,
                None => break,
            };
            let mut app = match self.app.try_borrow_mut() {
                Ok(a) => a,
                Err(_) => {
                    self.queue.borrow_mut().push_front(ev);
                    return;
                }
            };
            let t0 = now();
            app.handle(ev, t0);
            app.pending_handler_ms += now() - t0;
            any = true;
        }
        if any {
            self.request_frame();
        }
    }

    fn request_frame(&self) {
        if self.raf_pending.get() {
            return;
        }
        self.raf_pending.set(true);
        let cb = self.raf_cb.borrow();
        web_sys::window()
            .unwrap()
            .request_animation_frame(cb.as_ref().unwrap().as_ref().unchecked_ref())
            .unwrap();
    }

    fn on_raf(&self) {
        self.raf_pending.set(false);
        let more = {
            let mut app = self.app.borrow_mut();
            if app.engine.needs_frame() || !app.resync.is_empty() {
                app.run_frame();
            }
            app.engine.needs_frame()
        };
        // Events raised from inside the frame (e.g. focus echo) were queued, not lost.
        self.pump();
        if more {
            self.request_frame();
        }
    }
}

impl App {
    fn handle(&mut self, ev: HostEvent, t_recv: f64) {
        self.events_received += 1;
        if self.pending_t_recv.is_none() {
            self.pending_t_recv = Some(t_recv);
        }
        // bookkeeping events (focus echo, selection, resync) must not relabel the frame's real cause
        if !matches!(ev, HostEvent::Focus(_) | HostEvent::Selection(..) | HostEvent::ResyncInput(_) | HostEvent::FocusAlias(_)) || self.pending_class.is_empty() {
            self.pending_class = ev.class();
        }
        match ev {
            HostEvent::Pointer(k, x, y) => self.engine.pointer(k, x, y),
            HostEvent::Key { key, shift, ctrl, alt } => {
                self.engine.key(&key, shift, ctrl, alt);
            }
            HostEvent::Text(s) => self.engine.text(&s),
            HostEvent::Preedit { text, cursor } => self.engine.ime_preedit(&text, cursor),
            HostEvent::Commit(s) => self.engine.ime_commit(&s),
            HostEvent::Focus(id) => self.engine.focus_node(id),
            HostEvent::Selection(id, a, b) => self.engine.set_selection(id, a, b),
            HostEvent::FocusAlias(a) => self.engine.focus_alias(&a),
            HostEvent::Action(req) => self.engine.a11y_action(req),
            HostEvent::ResyncInput(id) => {
                if !self.resync.contains(&id) {
                    self.resync.push(id);
                }
            }
        }
    }

    fn run_frame(&mut self) {
        self.run_frame_inner(false);
    }

    fn run_frame_inner(&mut self, capture: bool) {
        let t_start = now();
        let t_recv = self.pending_t_recv.take().unwrap_or(t_start);
        let class = if self.frame_idx == 0 { "init" } else if self.pending_class.is_empty() { "idle_redraw" } else { self.pending_class };
        self.pending_class = "";

        // --- runtime: update/layout/scene/a11y
        self.scene.reset();
        let out = self.engine.frame(&mut self.scene, &mut self.resources);

        if out.tree.is_some() {
            // harness rects (author aliases), maintained independently of the mirror so mirror=off works
            self.rects.clear();
            for (k, r) in self.engine.named_rects() {
                self.rects.insert(k, [r[0] as f64, r[1] as f64, r[2] as f64, r[3] as f64]);
            }
        }

        // --- mirror (Q3): DOM mutation + one forced style/layout flush of the mirror subtree
        let t_m0 = now();
        let mut mirror_ms: Option<f64> = None;
        let mut overlay_only_ms: Option<f64> = None;
        let m_full = self.mirror.as_ref().map(|m| m.is_full()).unwrap_or(true);
        let bscale = if self.engine.bounds_are_physical() { 1.0 / self.dpr } else { 1.0 };
        if let Some(m) = self.mirror.as_mut() {
            m.set_bounds_scale(bscale);
            if let Some(tree) = &out.tree {
                let eng = &*self.engine;
                if let Err(e) = m.apply(tree, &|id| eng.input_info(id)) {
                    web_sys::console::error_1(&e);
                }
            }
            for id in self.resync.drain(..) {
                m.resync_input(id);
            }
            if let Some(fid) = self.engine.focused() {
                if let Some(info) = self.engine.input_info(fid) {
                    m.sync_selection(fid, info.selection_utf16);
                }
            }
            let _ = self.mirror_container.get_bounding_client_rect();
            mirror_ms = Some(now() - t_m0);
            if !m_full {
                overlay_only_ms = mirror_ms.take();
            }
        } else {
            self.resync.clear();
        }

        // --- render
        let t_r0 = now();
        let (w, h) = self.engine.target_size();
        let size = RenderSize { width: w, height: h };
        let clear = AlphaColor::<Srgb>::new([1.0, 1.0, 1.0, 1.0]);
        if let Err(e) = self.renderer.render(&self.scene, &mut self.resources, &size, &WebGlTextureBindings::new(), clear) {
            web_sys::console::error_1(&JsValue::from_str(&format!("render error: {e}")));
        }
        let t_r1 = now();
        let render_ms = t_r1 - t_r0;
        self.engine.record_render_ms(render_ms);
        let t_end = t_r1;
        if capture {
            // Read the drawing buffer in the same task as the render (preserveDrawingBuffer is false).
            let (w, h) = (size.width as usize, size.height as usize);
            let mut buf = vec![0u8; w * h * 4];
            let _ = self.gl.read_pixels_with_opt_u8_array(0, 0, w as i32, h as i32, WebGl2RenderingContext::RGBA, WebGl2RenderingContext::UNSIGNED_BYTE, Some(&mut buf));
            // GL origin is bottom-left; flip to top-left rows
            let mut flipped = vec![0u8; buf.len()];
            for y in 0..h {
                flipped[y * w * 4..(y + 1) * w * 4].copy_from_slice(&buf[(h - 1 - y) * w * 4..(h - y) * w * 4]);
            }
            self.golden = Some((w as u32, h as u32, flipped));
        }

        // total = event-handler work + rAF-callback work (METHOD §2.2 web), waiting excluded
        let handler_ms = std::mem::take(&mut self.pending_handler_ms);
        let raf_work = t_end - t_start;
        let total_ms = handler_ms + raf_work;

        // sync variant: force GPU completion with a 1x1 readPixels, after total is sampled
        let completion_ms = if self.sync_variant {
            let mut px = [0u8; 4];
            let _ = self.gl.read_pixels_with_opt_u8_array(0, 0, 1, 1, WebGl2RenderingContext::RGBA, WebGl2RenderingContext::UNSIGNED_BYTE, Some(&mut px));
            Some(total_ms + (now() - t_end))
        } else {
            None
        };

        // row (written after all spans are sampled)
        let row = js_sys::Object::new();
        let set = |k: &str, v: JsValue| {
            let _ = js_sys::Reflect::set(&row, &JsValue::from_str(k), &v);
        };
        let opt = |v: Option<f64>| v.map(JsValue::from_f64).unwrap_or(JsValue::NULL);
        set("frame_idx", JsValue::from_f64(self.frame_idx as f64));
        set("t_ns", JsValue::from_f64((t_recv * 1e6).round()));
        set("scenario", JsValue::from_str(&self.scenario));
        set("event", JsValue::from_str(class));
        set("update_ms", JsValue::from_f64(handler_ms + out.timings.update_ms));
        set("layout_ms", JsValue::from_f64(out.timings.layout_ms));
        set("scene_ms", JsValue::from_f64(out.timings.scene_ms));
        set("a11y_ms", JsValue::from_f64(out.timings.a11y_ms));
        set("mirror_ms", opt(mirror_ms));
        set("overlay_only_ms", opt(overlay_only_ms));
        set("render_ms", JsValue::from_f64(render_ms));
        set("total_ms", JsValue::from_f64(total_ms));
        set("completion_ms", opt(completion_ms));
        // extra columns (allowed after the first twelve)
        if let Some(m) = &self.mirror {
            set("mirror_nodes_in_update", JsValue::from_f64(m.stats.nodes_in_update as f64));
            set("mirror_nodes_changed", JsValue::from_f64(m.stats.nodes_changed as f64));
            set("mirror_dom_writes", JsValue::from_f64(m.stats.dom_writes as f64));
            set("mirror_elements", JsValue::from_f64(m.stats.total_elements as f64));
        }
        self.frames.push(&row);
        self.frame_idx += 1;
        if self.frame_idx == 1 {
            let win = web_sys::window().unwrap();
            let _ = js_sys::Reflect::set(&win, &"__pvcReady".into(), &JsValue::TRUE);
        }
        self.publish_meta();
    }

    fn publish_meta(&self) {
        let win = web_sys::window().unwrap();
        // Reuse the page's object so fields the JS bootstrap added (navigator_gpu, ...) survive.
        let existing = js_sys::Reflect::get(&win, &"__pvcMeta".into()).unwrap_or(JsValue::UNDEFINED);
        let m: js_sys::Object = if existing.is_object() { existing.unchecked_into() } else { js_sys::Object::new() };
        let set = |k: &str, v: JsValue| {
            let _ = js_sys::Reflect::set(&m, &JsValue::from_str(k), &v);
        };
        set("frames_presented", JsValue::from_f64(self.frame_idx as f64));
        set("events_received", JsValue::from_f64(self.events_received as f64));
        set("renderer", JsValue::from_str(&self.renderer_str));
        set("backend", JsValue::from_str("webgl2 (vello_gpu webgl feature, no wgpu)"));
        set("limits", JsValue::from_str("webgl2"));
        set("mirror", JsValue::from_str(if self.mirror.as_ref().map(|m| m.is_full()).unwrap_or(false) { "on" } else { "off (overlay inputs only)" }));
        set("variant", JsValue::from_str(if self.sync_variant { "sync" } else { "raf" }));
        set("perf_caveat_bypass", JsValue::from_bool(self.perf_caveat_bypass));
        set("init_ms", JsValue::from_f64(self.init_ms));
        set("shader_compile_ms", JsValue::from_f64(self.shader_ms));
        set("dpr", JsValue::from_f64(self.dpr));
        set("artifact_loaded_via_load_signed", JsValue::TRUE);
        if let Some(mir) = &self.mirror {
            set("mirror_elements", JsValue::from_f64(mir.element_count() as f64));
        }
        let _ = js_sys::Reflect::set(&win, &"__pvcMeta".into(), &m);
    }
}

/// Create (or reuse) the WebGL2 context. First attempt is vello_gpu's own (it asks for
/// `failIfMajorPerformanceCaveat: true`, which a software GL such as SwiftShader may refuse).
/// If that fails, the page creates the context itself with the caveat check off and retries;
/// the bypass is recorded in `__pvcMeta.perf_caveat_bypass`.
fn make_renderer(canvas: &HtmlCanvasElement, settings: RenderSettings) -> Result<(WebGlRenderer, Resources, bool), String> {
    match WebGlRenderer::new_with(canvas, settings, true) {
        Ok((r, res)) => Ok((r, res, false)),
        Err(first) => {
            let opts = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&opts, &"antialias".into(), &JsValue::FALSE);
            let _ = js_sys::Reflect::set(&opts, &"depth".into(), &JsValue::TRUE);
            let _ = js_sys::Reflect::set(&opts, &"failIfMajorPerformanceCaveat".into(), &JsValue::FALSE);
            let ctx = canvas.get_context_with_context_options("webgl2", &opts).map_err(|e| format!("{first}; fallback getContext threw {e:?}"))?;
            if ctx.is_none() {
                return Err(format!("{first}; fallback getContext('webgl2') returned null"));
            }
            WebGlRenderer::new_with(canvas, settings, true)
                .map(|(r, res)| (r, res, true))
                .map_err(|e| format!("first: {first}; after fallback: {e}"))
        }
    }
}

#[wasm_bindgen]
pub struct PvcWeb {
    host: Rc<Host>,
}

#[wasm_bindgen]
impl PvcWeb {
    /// `mirror_on`: G4 §8.3 default true; false only as a measurement control.
    /// `artifact`: signed canonical artifact (loaded through `pvc_ir::load_signed`, TEST key trust root;
    /// empty = none).
    #[wasm_bindgen(constructor)]
    pub fn new(
        canvas_id: &str,
        mirror_id: &str,
        artifact: &[u8],
        scenario: &str,
        locale: &str,
        sync_variant: bool,
        mirror_on: bool,
    ) -> Result<PvcWeb, JsValue> {
        pvc_runtime::timing::set_time_source(now);
        let t_init0 = now();
        let win = web_sys::window().ok_or("no window")?;
        let doc = win.document().ok_or("no document")?;
        let canvas: HtmlCanvasElement = doc.get_element_by_id(canvas_id).ok_or("canvas not found")?.unchecked_into();
        let mirror_container: HtmlElement = doc.get_element_by_id(mirror_id).ok_or("mirror container not found")?.unchecked_into();
        let dpr = win.device_pixel_ratio();
        let css_w = win.inner_width()?.as_f64().unwrap_or(1366.0);
        let css_h = win.inner_height()?.as_f64().unwrap_or(768.0);
        // Drawing buffer = the runtime's pixel size (ceil(css * dpr)); must equal RenderSize at render time.
        canvas.set_width((css_w * dpr).ceil() as u32);
        canvas.set_height((css_h * dpr).ceil() as u32);

        let settings = RenderSettings::default();
        let t_s0 = now();
        let (renderer, resources, bypass) = make_renderer(&canvas, settings).map_err(|e| JsValue::from_str(&e))?;
        let shader_ms = now() - t_s0;
        let gl = renderer.gl_context().clone();
        let renderer_str = match gl.get_extension("WEBGL_debug_renderer_info") {
            Ok(Some(_)) => gl.get_parameter(0x9246).ok().and_then(|v| v.as_string()).unwrap_or_default(),
            _ => gl.get_parameter(WebGl2RenderingContext::RENDERER).ok().and_then(|v| v.as_string()).unwrap_or_default(),
        };

        // Signed artifact -> Runtime through pvc_ir::load_signed (refuses bad signature / other signer).
        // Trust root in this session: the published TEST key (pvc_ir::sign::testkey), not a production key.
        let mut engine: Box<dyn Engine> = Box::new(
            runtime_engine::RuntimeEngine::new(artifact, locale, css_w as f32, css_h as f32, dpr as f32).map_err(|e| JsValue::from_str(&e))?,
        );
        engine.resize(css_w as f32, css_h as f32, dpr as f32);
        let (w, h) = engine.target_size();
        let scene = Scene::new_with(w, h, settings.level);

        let frames = js_sys::Array::new();
        let _ = js_sys::Reflect::set(&win, &"__pvcFrames".into(), &frames);

        let app = App {
            engine,
            renderer,
            resources,
            scene,
            gl,
            canvas: canvas.clone(),
            mirror: None,
            mirror_container: mirror_container.clone(),
            dpr,
            css_w,
            css_h,
            sync_variant,
            scenario: scenario.to_string(),
            frames,
            frame_idx: 0,
            events_received: 0,
            pending_handler_ms: 0.0,
            pending_t_recv: None,
            pending_class: "",
            resync: Vec::new(),
            rects: Default::default(),
            renderer_str,
            perf_caveat_bypass: bypass,
            init_ms: 0.0,
            shader_ms,
            artifact_nodes: None,
            golden: None,
        };
        let host = Rc::new(Host {
            app: RefCell::new(app),
            queue: RefCell::new(VecDeque::new()),
            raf_pending: Cell::new(false),
            raf_cb: RefCell::new(None),
        });
        {
            let h2 = host.clone();
            *host.raf_cb.borrow_mut() = Some(Closure::new(move |_t: f64| h2.on_raf()));
        }
        {
            let sink: Rc<dyn EventSink> = host.clone();
            let m = Mirror::new(doc.clone(), mirror_container, sink, 1.0, mirror_on);
            host.app.borrow_mut().mirror = Some(m);
        }
        host.app.borrow_mut().init_ms = now() - t_init0;
        host.app.borrow().publish_meta();
        let pvc = PvcWeb { host };
        pvc.host.request_frame(); // init frame
        Ok(pvc)
    }

    pub fn renderer(&self) -> String {
        self.host.app.borrow().renderer_str.clone()
    }

    pub fn needs_frame(&self) -> bool {
        self.host.app.borrow().engine.needs_frame()
    }

    pub fn pointer(&self, kind: &str, x: f32, y: f32) {
        let k = match kind {
            "down" => PointerKind::Down,
            "up" => PointerKind::Up,
            _ => PointerKind::Move,
        };
        self.host.push(HostEvent::Pointer(k, x, y));
    }

    /// Key typed while the canvas (not an overlay input) has focus. Returns true to preventDefault.
    pub fn key(&self, key: &str, shift: bool, ctrl: bool, alt: bool) -> bool {
        let handled = matches!(key, "Enter" | " " | "Escape");
        self.host.push(HostEvent::Key { key: key.to_string(), shift, ctrl, alt });
        handled
    }

    /// Golden-image hook (dev-render A7): force a frame, read the canvas pixels with `readPixels` in the same
    /// task as the render (not a screenshot) and return `{width, height, rgba: Uint8Array, dl_fingerprint}`.
    pub fn golden(&self) -> Result<JsValue, JsValue> {
        self.host.pump();
        let mut app = self.host.app.try_borrow_mut().map_err(|_| JsValue::from_str("busy"))?;
        app.run_frame_inner(true);
        let (w, h, px) = app.golden.take().ok_or("no pixels")?;
        let o = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&o, &"width".into(), &JsValue::from_f64(w as f64));
        let _ = js_sys::Reflect::set(&o, &"height".into(), &JsValue::from_f64(h as f64));
        let _ = js_sys::Reflect::set(&o, &"rgba".into(), &js_sys::Uint8Array::from(px.as_slice()));
        let _ = js_sys::Reflect::set(&o, &"dl_fingerprint".into(), &JsValue::from_str(&app.engine.fingerprint()));
        Ok(o.into())
    }

    /// Input hooks that drive the runtime directly (no DOM overlay), for golden states and tests.
    pub fn text(&self, s: &str) {
        self.host.push(HostEvent::Text(s.to_string()));
    }
    /// `cursor_start/end < 0` = no cursor (byte offsets into the preedit, as the runtime expects).
    pub fn ime_preedit(&self, s: &str, cursor_start: i32, cursor_end: i32) {
        let cursor = if cursor_start < 0 || cursor_end < 0 { None } else { Some((cursor_start as usize, cursor_end as usize)) };
        self.host.push(HostEvent::Preedit { text: s.to_string(), cursor });
    }
    pub fn ime_commit(&self, s: &str) {
        self.host.push(HostEvent::Commit(s.to_string()));
    }
    /// Focus a node by alias (`name`, `email`, `password`, `toggle`, `checkbox`, `submit`) or IR id.
    pub fn focus(&self, alias: &str) {
        self.host.push(HostEvent::FocusAlias(alias.to_string()));
    }

    /// Tab / Shift+Tab from the page (no overlay input focused). True = focus stayed inside the app.
    pub fn tab(&self, shift: bool) -> bool {
        self.host.tab(shift)
    }

    pub fn resize(&self, css_w: f64, css_h: f64, dpr: f64) {
        {
            let mut app = self.host.app.borrow_mut();
            app.dpr = dpr;
            app.css_w = css_w;
            app.css_h = css_h;
            app.canvas.set_width((css_w * dpr).ceil() as u32);
            app.canvas.set_height((css_h * dpr).ceil() as u32);
            app.engine.resize(css_w as f32, css_h as f32, dpr as f32);
            let (w, h) = app.engine.target_size();
            app.scene.reset_and_resize(w, h);
            app.pending_class = "resize";
        }
        self.host.request_frame();
    }

    /// Runtime state as JSON, for test assertions.
    pub fn debug_state(&self) -> String {
        self.host.app.borrow().engine.debug_state()
    }

    /// Caret rect (CSS px) as the canvas draws it for the focused text node, JSON or "".
    pub fn caret_rect(&self, node_id: &str) -> String {
        // NodeId is a u64 hash: passed as a decimal string (f64 would lose bits).
        let id: u64 = match node_id.parse() {
            Ok(v) => v,
            Err(_) => return String::new(),
        };
        match self.host.app.borrow().engine.caret_rect(NodeId(id)) {
            Some(r) => format!("{{\"x0\":{},\"y0\":{},\"x1\":{},\"y1\":{}}}", r.x0, r.y0, r.x1, r.y1),
            None => String::new(),
        }
    }

    /// Semantic-id -> CSS rect map for the harness (`window.__pvcRects`).
    pub fn rects_json(&self) -> String {
        let app = self.host.app.borrow();
        let parts: Vec<String> = app.rects.iter().map(|(k, r)| format!("\"{}\":[{},{},{},{}]", k, r[0], r[1], r[2], r[3])).collect();
        format!("{{{}}}", parts.join(","))
    }
}


