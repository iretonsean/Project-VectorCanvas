//! winit 0.30 application: ControlFlow::Wait, redraw only when `Runtime::needs_frame()`, AccessKit, IME, tracing.
use crate::gpu::{Gpu, Renderer};
use crate::trace::{self, Row, Trace};
use pvc_runtime::{Key, Mods, Runtime};
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key as WKey, NamedKey};
use winit::window::{Window, WindowId};

pub enum UserEvent {
    Access(accesskit_winit::Event),
    Term,
}
impl From<accesskit_winit::Event> for UserEvent {
    fn from(e: accesskit_winit::Event) -> Self {
        UserEvent::Access(e)
    }
}

pub struct HostInfo {
    pub artifact_sha256: String,
    pub artifact: String,
    pub locale: String,
    pub profile: &'static str,
    pub renderer: Renderer,
}

struct Pending {
    t0: Instant,
    rt_ns: u64,
    class: &'static str,
    n: u32,
    handler_ms: f64,
}

struct Running {
    window: Arc<Window>,
    gpu: Gpu,
    adapter: accesskit_winit::Adapter,
}

pub struct App {
    rt: Runtime,
    info: HostInfo,
    proxy: EventLoopProxy<UserEvent>,
    run: Option<Running>,
    trace: Trace,
    t_start: Instant,
    rt_start_ns: u64,
    pending: Option<Pending>,
    frames: u64,
    events_received: u64,
    ime_allowed: bool,
    ime_rect: Option<[f32; 4]>,
    ime_set_allowed_calls: u32,
    ime_set_area_calls: u32,
    ime_events: Vec<String>,
    a11y_active_seen: bool,
    a11y_updates_sent: u64,
    a11y_actions: u32,
    mods: Mods,
    cursor: (f32, f32),
    scale: f64,
    rects_written: bool,
    first_frame_ms: f64,
    error: Option<String>,
    gpu_info: Option<[String; 4]>,
}

pub fn run(rt: Runtime, info: HostInfo, t_start: Instant, rt_start_ns: u64) -> Result<(), String> {
    trace::sig::install();
    let el = EventLoop::<UserEvent>::with_user_event().build().map_err(|e| format!("event loop: {e}"))?;
    el.set_control_flow(ControlFlow::Wait);
    let proxy = el.create_proxy();
    // watcher: turn SIGTERM/SIGINT into a user event (the loop is in Wait and would not notice a flag)
    let p2 = proxy.clone();
    std::thread::spawn(move || loop {
        if trace::sig::requested() {
            let _ = p2.send_event(UserEvent::Term);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    });
    let mut app = App {
        rt,
        info,
        proxy,
        run: None,
        trace: Trace::from_env(),
        t_start,
        rt_start_ns,
        pending: None,
        frames: 0,
        events_received: 0,
        ime_allowed: false,
        ime_rect: None,
        ime_set_allowed_calls: 0,
        ime_set_area_calls: 0,
        ime_events: vec![],
        a11y_active_seen: false,
        a11y_updates_sent: 0,
        a11y_actions: 0,
        mods: Mods::default(),
        cursor: (0.0, 0.0),
        scale: 1.0,
        rects_written: false,
        first_frame_ms: 0.0,
        error: None,
        gpu_info: None,
    };
    el.run_app(&mut app).map_err(|e| format!("run_app: {e}"))?;
    app.write_meta();
    match app.error.take() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

impl App {
    fn note(&mut self, class: &'static str) {
        self.events_received += 1;
        match self.pending.as_mut() {
            Some(p) => {
                p.class = class;
                p.n += 1;
            }
            None => self.pending = Some(Pending { t0: Instant::now(), rt_ns: trace::realtime_ns(), class, n: 1, handler_ms: 0.0 }),
        }
    }

    /// After handling an input: sync IME state, then request a redraw only if the runtime has damage.
    fn after_input(&mut self, handler_ms: f64) {
        if let Some(p) = self.pending.as_mut() {
            p.handler_ms += handler_ms;
        }
        self.sync_ime();
        if self.rt.needs_frame() {
            if let Some(r) = &self.run {
                r.window.request_redraw();
            }
        } else {
            self.pending = None; // input produced no damage -> no frame (not an interaction frame)
        }
    }

    /// `set_ime_allowed` follows "a text input has focus"; `set_ime_cursor_area` follows the runtime's caret rect.
    fn sync_ime(&mut self) {
        let Some(r) = &self.run else { return };
        let rect = self.rt.focused_text_input_rect().map(|q| [q.x, q.y, q.w, q.h]);
        let want = rect.is_some();
        if want != self.ime_allowed {
            r.window.set_ime_allowed(want);
            self.ime_allowed = want;
            self.ime_set_allowed_calls += 1;
        }
        if let (Some(q), true) = (rect, rect != self.ime_rect) {
            r.window.set_ime_cursor_area(LogicalPosition::new(q[0] as f64, q[1] as f64), LogicalSize::new(q[2] as f64, q[3] as f64));
            self.ime_set_area_calls += 1;
        }
        self.ime_rect = rect;
    }

    fn frame(&mut self, first: bool) {
        let Some(run) = self.run.as_mut() else { return };
        let (t0, rt_ns, class, n, handler_ms) = match self.pending.take() {
            Some(p) => (p.t0, p.rt_ns, p.class, p.n, p.handler_ms),
            None if first => (self.t_start, self.rt_start_ns, "init", 0, 0.0),
            None => (Instant::now(), trace::realtime_ns(), "idle_redraw", 0, 0.0),
        };
        // runtime stages (only the dirty ones run): update -> layout -> scene -> a11y build
        self.rt.frame();
        let st = self.rt.timings();
        let render_ms = match run.gpu.frame(self.rt.display_list()) {
            Ok(ms) => ms,
            Err(e) => {
                eprintln!("pvc-native: frame error: {e}");
                return;
            }
        };
        self.rt.record_render_ms(render_ms);
        // hand-off to accesskit_winit (the closure runs only when an AT-SPI client activated the adapter)
        let ta = Instant::now();
        if let Some(tu) = self.rt.take_tree_update() {
            let sent = &mut self.a11y_updates_sent;
            run.adapter.update_if_active(|| {
                *sent += 1;
                tu
            });
        }
        let a11y_handoff_ms = ta.elapsed().as_secs_f64() * 1e3;
        let total_ms = t0.elapsed().as_secs_f64() * 1e3;
        // forced GPU completion, after total_ms is sampled
        run.gpu.wait_idle();
        let completion_ms = t0.elapsed().as_secs_f64() * 1e3;
        let row = Row {
            frame_idx: self.frames,
            t_ns: rt_ns,
            event: class,
            update_ms: handler_ms + st.update_ms,
            layout_ms: st.layout_ms,
            scene_ms: st.scene_ms,
            a11y_ms: st.a11y_ms + a11y_handoff_ms,
            render_ms,
            total_ms,
            completion_ms,
            n_events: n,
        };
        if first {
            self.first_frame_ms = (trace::realtime_ns().saturating_sub(self.rt_start_ns)) as f64 / 1e6;
        }
        self.trace.write(&row);
        self.frames += 1;
        self.sync_ime(); // caret moved with the text
        if !self.rects_written {
            self.rects_written = true;
            self.write_rects();
        }
        if std::env::var_os("PVC_STATE").is_some() {
            trace::write_file("PVC_STATE", &self.rt.debug_state());
        }
    }

    fn write_rects(&self) {
        // pvc_runtime::named_rects() already carries the harness aliases (name, email, ..., connect_button, bio_text)
        let mut seen = std::collections::HashSet::new();
        let parts: Vec<String> = self
            .rt
            .named_rects()
            .iter()
            .filter(|(k, _)| seen.insert(k.clone()))
            .map(|(k, r)| format!("\"{}\":[{},{},{},{}]", trace::json_escape(k), r[0], r[1], r[2], r[3]))
            .collect();
        trace::write_file("PVC_RECTS", &format!("{{{}}}", parts.join(",")));
    }

    fn write_meta(&self) {
        let (renderer, backend, limits, format, pm) = match &self.gpu_info {
            Some([a, b, f, p]) => (a.clone(), b.clone(), "downlevel_webgl2_defaults", f.clone(), p.clone()),
            None => (String::new(), String::new(), "", String::new(), String::new()),
        };
        let kb = |k| trace::proc_status_kb(k).map(|v| v.to_string()).unwrap_or_else(|| "null".into());
        let ime = self.ime_events.iter().map(|s| format!("\"{}\"", trace::json_escape(s))).collect::<Vec<_>>().join(",");
        let body = format!(
            "{{\"label\":\"host=container-xeon4 path={}\",\"renderer_mode\":\"{}\",\"frames_presented\":{},\"events_received\":{},\"renderer\":\"{}\",\"backend\":\"{}\",\
\"vmhwm_kb\":{},\"vmrss_kb\":{},\"rssanon_kb\":{},\"rssfile_kb\":{},\"rssshmem_kb\":{},\
\"build\":{{\"git_or_sha256\":\"{}\",\"profile\":\"{}\"}},\"limits\":\"{}\",\"surface_format\":\"{}\",\"present_mode\":\"{}\",\
\"first_frame_ms_from_main_entry\":{:.3},\"locale\":\"{}\",\"runtime\":\"pvc_runtime\",\"artifact\":\"{}\",\"artifact_canonical_sha256\":\"{}\",\
\"a11y_activated\":{},\"a11y_updates_sent\":{},\"a11y_actions\":{},\"ime_set_allowed_calls\":{},\"ime_set_cursor_area_calls\":{},\"ime_events\":[{}],\
\"general_constraint_solve_excluded_D4\":true,\"not_a_tier1_device_result\":true}}",
            match self.info.renderer { Renderer::Gl => "gl-llvmpipe", Renderer::Cpu => "tier0c-vello_cpu" },
            match self.info.renderer { Renderer::Gl => "vello_gpu on wgpu-GL (llvmpipe), WebGL2 downlevel limits imposed", Renderer::Cpu => "vello_cpu windowed: CPU raster then texture upload+present via wgpu-GL; mode=windowed" },
            self.frames,
            self.events_received,
            trace::json_escape(&renderer),
            backend,
            kb("VmHWM"),
            kb("VmRSS"),
            kb("RssAnon"),
            kb("RssFile"),
            kb("RssShmem"),
            self.info.artifact_sha256,
            self.info.profile,
            limits,
            format,
            pm,
            self.first_frame_ms,
            self.info.locale,
            trace::json_escape(&self.info.artifact),
            self.info.artifact_sha256,
            self.a11y_active_seen,
            self.a11y_updates_sent,
            self.a11y_actions,
            self.ime_set_allowed_calls,
            self.ime_set_area_calls,
            ime
        );
        trace::write_file("PVC_META", &body);
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.run.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("PVC")
            .with_inner_size(PhysicalSize::new(1366u32, 768u32))
            .with_resizable(false)
            .with_visible(false);
        let window = match el.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.error = Some(format!("create_window: {e}"));
                el.exit();
                return;
            }
        };
        // AccessKit adapter must exist before the window is shown (accesskit_winit docs)
        let adapter = accesskit_winit::Adapter::with_event_loop_proxy(el, &window, self.proxy.clone());
        let gpu = match Gpu::new(el.owned_display_handle(), window.clone(), self.info.renderer) {
            Ok(g) => g,
            Err(e) => {
                self.error = Some(format!("gpu init: {e}"));
                el.exit();
                return;
            }
        };
        self.scale = window.scale_factor();
        let sz = window.inner_size();
        self.rt.resize((sz.width as f64 / self.scale) as f32, (sz.height as f64 / self.scale) as f32, self.scale as f32);
        window.set_visible(true);
        self.gpu_info = Some([gpu.adapter_desc.clone(), gpu.backend.clone(), gpu.format.clone(), gpu.present_mode.clone()]);
        self.run = Some(Running { window, gpu, adapter });
        self.frame(true);
    }

    fn user_event(&mut self, el: &ActiveEventLoop, ev: UserEvent) {
        match ev {
            UserEvent::Term => {
                // meta first: winit 0.30.13 can abort in EventProcessor::drop if a preedit callback fires
                // during teardown (REPORT finding W1), so nothing may depend on a clean return.
                self.write_meta();
                if let Some(r) = &self.run {
                    r.window.set_ime_allowed(false);
                }
                self.run = None;
                el.exit();
            }
            UserEvent::Access(e) => match e.window_event {
                accesskit_winit::WindowEvent::InitialTreeRequested => {
                    self.a11y_active_seen = true;
                    let tu = self.rt.full_tree();
                    if let Some(r) = self.run.as_mut() {
                        r.adapter.update_if_active(|| tu);
                    }
                }
                accesskit_winit::WindowEvent::ActionRequested(req) => {
                    self.a11y_actions += 1;
                    self.note("other");
                    let t = Instant::now();
                    self.rt.a11y_action(&req);
                    self.after_input(t.elapsed().as_secs_f64() * 1e3);
                }
                accesskit_winit::WindowEvent::AccessibilityDeactivated => {
                    self.a11y_active_seen = false;
                }
            },
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let t_in = Instant::now();
        if let Some(r) = self.run.as_mut() {
            r.adapter.process_event(&r.window, &event);
        }
        match event {
            WindowEvent::CloseRequested => {
                self.write_meta();
                el.exit()
            }
            WindowEvent::RedrawRequested => {
                if self.rt.needs_frame() {
                    self.frame(false);
                }
            }
            WindowEvent::Resized(sz) => {
                if let Some(r) = self.run.as_mut() {
                    if sz.width != r.gpu.width() || sz.height != r.gpu.height() {
                        r.gpu.resize(sz.width, sz.height);
                    }
                }
                self.note("resize");
                self.rt.resize((sz.width as f64 / self.scale) as f32, (sz.height as f64 / self.scale) as f32, self.scale as f32);
                self.after_input(t_in.elapsed().as_secs_f64() * 1e3);
            }
            WindowEvent::ModifiersChanged(m) => {
                let s = m.state();
                self.mods = Mods { shift: s.shift_key(), ctrl: s.control_key(), alt: s.alt_key() };
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = ((position.x / self.scale) as f32, (position.y / self.scale) as f32);
                self.note("ptr_move");
                self.rt.pointer_move(self.cursor.0, self.cursor.1);
                self.after_input(t_in.elapsed().as_secs_f64() * 1e3);
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                let pressed = state == ElementState::Pressed;
                self.note(if pressed { "ptr_down" } else { "ptr_up" });
                if pressed {
                    self.rt.pointer_down(self.cursor.0, self.cursor.1);
                } else {
                    self.rt.pointer_up(self.cursor.0, self.cursor.1);
                }
                self.after_input(t_in.elapsed().as_secs_f64() * 1e3);
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let named = match &event.logical_key {
                    WKey::Named(NamedKey::Tab) => Some(Key::Tab),
                    WKey::Named(NamedKey::Enter) => Some(Key::Enter),
                    WKey::Named(NamedKey::Backspace) => Some(Key::Backspace),
                    WKey::Named(NamedKey::Delete) => Some(Key::Delete),
                    WKey::Named(NamedKey::ArrowLeft) => Some(Key::Left),
                    WKey::Named(NamedKey::ArrowRight) => Some(Key::Right),
                    WKey::Named(NamedKey::ArrowUp) => Some(Key::Up),
                    WKey::Named(NamedKey::ArrowDown) => Some(Key::Down),
                    WKey::Named(NamedKey::Home) => Some(Key::Home),
                    WKey::Named(NamedKey::End) => Some(Key::End),
                    WKey::Named(NamedKey::Escape) => Some(Key::Escape),
                    _ => None,
                };
                let shortcut = match &event.logical_key {
                    WKey::Character(c) if self.mods.ctrl => c.chars().next().map(Key::Char),
                    _ => None,
                };
                let text = event.text.as_ref().map(|t| t.to_string()).filter(|t| !t.is_empty() && !t.chars().any(|c| c.is_control()));
                if let Some(k) = named.or(shortcut) {
                    self.note(if k == Key::Tab { "key_tab" } else { "key_other" });
                    self.rt.key(k, self.mods);
                } else if let (Some(t), false) = (text, self.mods.ctrl) {
                    self.note("key_char");
                    self.rt.text(&t);
                } else {
                    return;
                }
                self.after_input(t_in.elapsed().as_secs_f64() * 1e3);
            }
            WindowEvent::Ime(ime) => {
                let (class, label) = match &ime {
                    Ime::Enabled => ("ime_preedit", "Enabled".to_string()),
                    Ime::Disabled => ("ime_preedit", "Disabled".to_string()),
                    Ime::Preedit(s, c) => ("ime_preedit", format!("Preedit({s:?},{c:?})")),
                    Ime::Commit(s) => ("ime_commit", format!("Commit({s:?})")),
                };
                eprintln!("pvc-native: winit Ime::{label}");
                if self.ime_events.len() < 400 {
                    self.ime_events.push(label);
                }
                self.note(class);
                match ime {
                    Ime::Enabled | Ime::Disabled => {}
                    Ime::Preedit(s, c) => self.rt.ime_preedit(&s, c),
                    Ime::Commit(s) => self.rt.ime_commit(&s),
                }
                self.after_input(t_in.elapsed().as_secs_f64() * 1e3);
            }
            _ => {}
        }
    }
}
