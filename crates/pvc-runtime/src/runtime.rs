//! The runtime facade: load, input, update → layout → scene → a11y, damage, instrumentation.
//! Semantics are produced here once, as `accesskit::TreeUpdate`; hosts only forward them.

use crate::a11y;
use crate::edit::{Field, Motion};
use crate::eval::{Behaviour, EffectRecord};
use crate::geom::{snap, Rect, Rgba};
use crate::layout;
use crate::model::Flat;
use crate::paint::{DisplayList, DrawOp};
use crate::text::TextSystem;
use crate::timing::{now_ms, FrameTimings};
use pvc_ir::ast::{Event, Module, Node, NodeKind, Shape, Value};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Options {
    /// BCP-47 locale for strings and shaping, e.g. "en" or "zh-CN". Must be declared in the manifest.
    pub locale: String,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
}

#[derive(Debug)]
pub enum LoadError {
    Model(String),
    Locale(String),
    Fonts(String),
    Unsigned,
    Ir(String),
    /// A construct outside the A2 scope (L01/F01 subset). The pilot extension (B4-runtime) lifts this.
    Unsupported(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for LoadError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Tab,
    Enter,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Escape,
    /// For shortcuts only (e.g. Ctrl+A). Printable text arrives through `text()`.
    Char(char),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// Geometry and content of a text input for the web overlay / native IME placement.
#[derive(Clone, Debug)]
pub struct TextInputInfo {
    pub rect: Rect,
    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub text_origin_x: f32,
    pub text_origin_y: f32,
    pub value: String,
    pub masked: bool,
    pub selection_utf16: (usize, usize),
    pub composing: bool,
}

#[derive(Default)]
struct Dirty {
    update: bool,
    layout: bool,
    scene: bool,
    a11y: bool,
}

pub struct Runtime {
    pub(crate) module: Module,
    pub(crate) flat: Flat,
    pub(crate) locale: String,
    pub(crate) ts: TextSystem,
    pub(crate) beh: Behaviour,
    pub(crate) fields: HashMap<usize, Field>,
    pub(crate) vis: Vec<bool>,
    pub(crate) texts: Vec<Option<String>>,
    pub(crate) rects: Vec<Rect>,
    pub(crate) vw: f32,
    pub(crate) vh: f32,
    pub(crate) scale: f32,
    pub(crate) focus: Option<usize>,
    pressed: Option<usize>,
    drag_field: Option<usize>,
    last_click: Option<(f64, f32, f32, u8)>,
    dirty: Dirty,
    dl: DisplayList,
    pub(crate) a11y: a11y::A11yState,
    timings: FrameTimings,
    frames: u64,
    pub load_warnings: Vec<String>,
    /// Short host-facing names (see `alias_for`), parallel to the arena.
    pub(crate) aliases: Vec<Option<String>>,
}

/// Short names that hosts and tests use for the L01/F01 elements. Derived by rule from the IR, not
/// stored in it: text-input → its `bind` field; id ending `/toggle` → `toggle`; the checkbox →
/// `checkbox`; a button whose handler raises `submit` → `submit`; a button that emits an effect →
/// `connect_button`; in a module without text inputs, the text node with the longest content
/// → `bio_text` (the card's one-sentence bio in L01).
fn compute_aliases(flat: &Flat) -> Vec<Option<String>> {
    use pvc_ir::ast::{Action, Role};
    let mut out: Vec<Option<String>> = vec![None; flat.len()];
    for (i, n) in flat.nodes.iter().enumerate() {
        out[i] = match &n.kind {
            NodeKind::TextInput(p) => Some(p.bind.clone()),
            _ if n.id.ends_with("/toggle") => Some("toggle".into()),
            _ if n.sem.role == Role::Checkbox => Some("checkbox".into()),
            _ if n.sem.role == Role::Button => {
                let acts: Vec<&Action> = n.on.iter().flat_map(|h| h.actions.iter()).collect();
                if acts.iter().any(|a| matches!(a, Action::Submit(_))) {
                    Some("submit".into())
                } else if acts.iter().any(|a| matches!(a, Action::Emit(_))) {
                    Some("connect_button".into())
                } else {
                    None
                }
            }
            _ => None,
        };
    }
    out
}

fn default_locale_ok(m: &Module, l: &str) -> bool {
    m.manifest.locales.iter().any(|x| x == l)
}

impl Runtime {
    /// Production constructor: the module must come out of `pvc_ir::load_signed` (signature and
    /// canonical-form check). `verify` is typically
    /// `|b| pvc_ir::load_signed(b, &trusted_key)`; the runtime never accepts an unverified module.
    pub fn from_signed<F>(bytes: &[u8], verify: F, opts: Options) -> Result<Runtime, LoadError>
    where
        F: FnOnce(&[u8]) -> Result<Module, pvc_ir::LoadError>,
    {
        let module = verify(bytes).map_err(|e| LoadError::Ir(format!("{e}")))?;
        Runtime::build(module, opts)
    }

    /// TEST-ONLY constructor for an unsigned module. Production hosts use [`Runtime::from_signed`].
    #[cfg(any(test, feature = "test-unsigned"))]
    pub fn new_unsigned_for_test(module: Module, opts: Options) -> Result<Runtime, LoadError> {
        Runtime::build(module, opts)
    }

    pub(crate) fn build(module: Module, opts: Options) -> Result<Runtime, LoadError> {
        crate::support::check(&module)?;
        let flat = Flat::build(&module).map_err(LoadError::Model)?;
        let locale = if default_locale_ok(&module, &opts.locale) {
            opts.locale.clone()
        } else {
            return Err(LoadError::Locale(format!("locale {} not declared in manifest", opts.locale)));
        };
        let mut warnings = vec![];
        for f in &module.manifest.fonts {
            if !crate::fonts::BUNDLE.iter().any(|b| b.sha256 == f.sha256) {
                warnings.push(format!("manifest font {} ({}) is not in the runtime bundle; bundled stack used", f.family, f.sha256));
            }
        }
        let n = flat.len();
        let beh = Behaviour::new(&module);
        let mut rt = Runtime {
            ts: TextSystem::new(&locale),
            beh,
            fields: HashMap::new(),
            vis: vec![true; n],
            texts: vec![None; n],
            rects: vec![Rect::default(); n],
            vw: opts.width,
            vh: opts.height,
            scale: opts.scale.max(0.5),
            focus: None,
            pressed: None,
            drag_field: None,
            last_click: None,
            dirty: Dirty { update: true, layout: true, scene: true, a11y: true },
            dl: DisplayList::default(),
            a11y: a11y::A11yState::default(),
            timings: FrameTimings::default(),
            frames: 0,
            load_warnings: warnings,
            aliases: vec![],
            module,
            flat,
            locale,
        };
        a11y::check_ids(&rt.flat).map_err(LoadError::Model)?;
        rt.aliases = compute_aliases(&rt.flat);
        if !rt.flat.nodes.iter().any(|n| matches!(n.kind, NodeKind::TextInput(_))) {
            let mut best: Option<(usize, usize)> = None;
            for (i, n) in rt.flat.nodes.iter().enumerate() {
                if let NodeKind::Text(t) = &n.kind {
                    let len = rt.module.string(rt.module.default_locale(), &t.key).map(|s| s.chars().count()).unwrap_or(0);
                    if best.map(|(_, l)| len > l).unwrap_or(true) {
                        best = Some((i, len));
                    }
                }
            }
            if let Some((i, _)) = best {
                rt.aliases[i] = Some("bio_text".into());
            }
        }
        rt.update_stage();
        Ok(rt)
    }

    // ------------------------------------------------------------------ queries

    pub fn locale(&self) -> &str {
        &self.locale
    }
    pub fn module(&self) -> &Module {
        &self.module
    }
    pub fn timings(&self) -> FrameTimings {
        self.timings
    }
    pub fn frame_count(&self) -> u64 {
        self.frames
    }
    pub fn effect_log(&self) -> &[EffectRecord] {
        &self.beh.effects
    }
    pub fn state_value(&self, name: &str) -> Option<&Value> {
        self.beh.get(name)
    }
    /// Host calls this after rendering the display list so `timings().render_ms/total_ms` are whole.
    pub fn record_render_ms(&mut self, ms: f64) {
        self.timings.render_ms = ms;
        self.timings.total_ms =
            self.timings.update_ms + self.timings.layout_ms + self.timings.scene_ms + self.timings.a11y_ms + ms;
    }

    pub fn needs_frame(&self) -> bool {
        self.dirty.update || self.dirty.layout || self.dirty.scene || self.dirty.a11y
    }

    pub fn display_list(&self) -> &DisplayList {
        &self.dl
    }

    /// Resolve an IR node id **or** an alias (`name`, `email`, `password`, `toggle`, `checkbox`,
    /// `submit`, `connect_button`, `bio_text`) to an arena index.
    pub fn node_idx(&self, id: &str) -> Option<usize> {
        self.flat.by_id.get(id).copied().or_else(|| self.aliases.iter().position(|a| a.as_deref() == Some(id)))
    }

    pub fn is_visible(&self, id: &str) -> bool {
        self.node_idx(id).map(|i| self.vis[i]).unwrap_or(false)
    }

    /// Absolute rect (logical px) of a visible node by IR id (valid after `frame()`).
    pub fn rect_of(&self, id: &str) -> Option<Rect> {
        let i = self.node_idx(id)?;
        self.vis[i].then(|| self.rects[i])
    }

    /// Every visible node's id and rect `[x, y, w, h]` in logical px, document order.
    pub fn named_rects(&self) -> Vec<(String, [f32; 4])> {
        let mut out = vec![];
        for i in (0..self.flat.len()).filter(|&i| self.vis[i]) {
            let r = [self.rects[i].x, self.rects[i].y, self.rects[i].w, self.rects[i].h];
            out.push((self.flat.nodes[i].id.clone(), r));
            if let Some(a) = &self.aliases[i] {
                out.push((a.clone(), r));
            }
        }
        out
    }

    /// Content box of a text-input node (where the editable text is drawn), logical px. For the web
    /// overlay `<input>` and the native IME area.
    pub fn text_input_rect(&self, id: &str) -> Option<Rect> {
        let i = self.node_idx(id)?;
        if !self.vis[i] || !matches!(self.flat.nodes[i].kind, NodeKind::TextInput(_)) {
            return None;
        }
        Some(self.content_rect(i))
    }

    /// Caret / preedit area (absolute logical px) of the focused text input, for the IME candidate
    /// window (`winit::Window::set_ime_cursor_area`). `None` if no text input has focus.
    pub fn focused_text_input_rect(&self) -> Option<Rect> {
        let i = self.focus?;
        let f = self.fields.get(&i)?;
        let c = self.content_rect(i);
        let a = f.ime_area();
        Some(Rect::new(c.x + a.x, c.y + a.y, a.w.max(1.0), a.h.max(c.h.min(self.line_h(i)))))
    }

    /// Everything the web overlay `<input>` needs to sit pixel-aligned over a canvas field.
    pub fn text_input_info(&self, id: &str) -> Option<TextInputInfo> {
        let i = self.node_idx(id)?;
        let rect = self.text_input_rect(id)?;
        let f = self.fields.get(&i)?;
        let p = match &self.flat.nodes[i].kind {
            NodeKind::TextInput(p) => p,
            _ => return None,
        };
        Some(TextInputInfo {
            rect,
            font_family: crate::fonts::STACK.to_string(),
            font_size: p.size.px(),
            line_height: p.line_height.px(),
            text_origin_x: rect.x - f.scroll_x,
            text_origin_y: rect.y,
            value: f.value().to_string(),
            masked: f.is_masked(),
            selection_utf16: f.selection_utf16(),
            composing: f.is_composing(),
        })
    }

    /// Caret rectangle (absolute logical px) of the text input `id`.
    pub fn caret_rect(&self, id: &str) -> Option<Rect> {
        let i = self.node_idx(id)?;
        let c = self.content_rect(i);
        let r = self.fields.get(&i)?.caret_rect()?;
        Some(Rect::new(c.x + r.x, c.y + r.y, r.w, r.h))
    }

    /// Set the selection of text input `id`. Offsets are UTF-16 code units into the real value
    /// (the unit of DOM `selectionStart/End`).
    pub fn set_selection(&mut self, id: &str, start_utf16: usize, end_utf16: usize) {
        if let Some(i) = self.node_idx(id) {
            if let Some(f) = self.fields.get_mut(&i) {
                f.set_selection_utf16(&mut self.ts, start_utf16, end_utf16);
                self.dirty.scene = true;
            }
        }
    }

    /// Stable accesskit id for an IR node id, and the reverse.
    pub fn node_id(&self, id: &str) -> accesskit::NodeId {
        a11y::node_id(id)
    }
    pub fn ir_id_of(&self, n: accesskit::NodeId) -> Option<&str> {
        let i = self.a11y.index_of(n)?;
        Some(self.flat.nodes[i].id.as_str())
    }

    /// Focus a node by IR id (e.g. the web overlay received DOM focus).
    pub fn focus_id(&mut self, id: &str) {
        if let Some(i) = self.node_idx(id) {
            if self.focusable(i) {
                self.set_focus(Some(i));
            }
        }
    }

    pub fn focused_id(&self) -> Option<&str> {
        self.focus.map(|i| self.flat.nodes[i].id.as_str())
    }

    fn line_h(&self, i: usize) -> f32 {
        match &self.flat.nodes[i].kind {
            NodeKind::TextInput(p) => p.line_height.px(),
            _ => 0.0,
        }
    }

    fn content_rect(&self, i: usize) -> Rect {
        let r = self.rects[i];
        let p = &self.flat.nodes[i].layout.pad;
        Rect::new(r.x + p.left.px(), r.y + p.top.px(), (r.w - p.left.px() - p.right.px()).max(0.0), (r.h - p.top.px() - p.bottom.px()).max(0.0))
    }

    /// Human-readable state for host test logs: field values (incl. committed IME text), toggles.
    pub fn debug_state(&self) -> String {
        let mut keys: Vec<&String> = self.beh.state.keys().collect();
        keys.sort();
        let mut s = String::new();
        for k in keys {
            s.push_str(&format!("{}={:?}\n", k, self.beh.state[k]));
        }
        let mut dk: Vec<&String> = self.beh.derived.keys().collect();
        dk.sort();
        for k in dk {
            s.push_str(&format!("derived {}={:?}\n", k, self.beh.derived[k]));
        }
        if let Some(f) = self.focused_id() {
            s.push_str(&format!("focus={f}\n"));
        }
        for e in &self.beh.effects {
            s.push_str(&format!("effect#{} {} cap={} permitted={}\n", e.seq, e.effect, e.capability, e.permitted));
        }
        s
    }

    /// Text of the current selection in the focused field (for host-side copy), if any and not masked.
    pub fn selected_text(&self) -> Option<String> {
        self.fields.get(&self.focus?)?.selected_text()
    }

    /// Solved geometry as stable text for the D3 test: `id x y w h` in 1/64 px integers, document
    /// order, visible nodes only, plus `lines=<n> <w1>,<w2>..` for text nodes.
    pub fn geometry_dump(&mut self) -> String {
        self.run_update_and_layout();
        let mut out = String::new();
        for i in 0..self.flat.len() {
            if !self.vis[i] {
                continue;
            }
            let r = self.rects[i];
            let q = |v: f32| (v * 64.0).round() as i64;
            out.push_str(&format!("{} {} {} {} {}", self.flat.nodes[i].id, q(r.x), q(r.y), q(r.w), q(r.h)));
            if let NodeKind::Text(t) = &self.flat.nodes[i].kind {
                let p = &self.flat.nodes[i].layout.pad;
                let st = crate::text::TextStyle { size: t.size.px(), weight: t.weight, line_height: t.line_height.px(), align: match t.align { pvc_ir::ast::TextAlign::Start => 0, pvc_ir::ast::TextAlign::Center => 1, pvc_ir::ast::TextAlign::End => 2 } };
                let s = self.texts[i].clone().unwrap_or_default();
                let l = self.ts.layout(&s, &st, Some((r.w - p.left.px() - p.right.px()).max(0.0)));
                let widths: Vec<String> = l.lines().map(|ln| q(ln.metrics().advance).to_string()).collect();
                out.push_str(&format!(" lines={} {}", widths.len(), widths.join(",")));
            }
            out.push('\n');
        }
        out
    }

    // ------------------------------------------------------------------ input

    pub fn resize(&mut self, w: f32, h: f32, scale: f32) {
        if (w, h, scale) != (self.vw, self.vh, self.scale) {
            self.vw = w;
            self.vh = h;
            self.scale = scale.max(0.5);
            self.dirty.layout = true;
            self.dirty.scene = true;
            self.dirty.a11y = true;
        }
    }

    fn enabled(&self, i: usize) -> bool {
        let n = &self.flat.nodes[i];
        !n.sem.disabled.as_ref().map(|e| self.beh.truthy(e)).unwrap_or(false)
    }

    fn focusable(&self, i: usize) -> bool {
        let n = &self.flat.nodes[i];
        self.vis[i] && n.is_interactive() && self.enabled(i)
    }

    fn hit(&self, x: f32, y: f32) -> Option<usize> {
        (0..self.flat.len()).rev().find(|&i| self.vis[i] && self.flat.nodes[i].is_interactive() && self.rects[i].contains(x, y))
    }

    fn set_focus(&mut self, f: Option<usize>) {
        if self.focus != f {
            self.focus = f;
            self.dirty.scene = true;
            self.dirty.a11y = true;
        }
    }

    pub fn pointer_move(&mut self, x: f32, y: f32) {
        if let Some(i) = self.drag_field {
            let c = self.content_rect(i);
            if let Some(f) = self.fields.get_mut(&i) {
                f.click(&mut self.ts, x - c.x, y - c.y, true);
                self.dirty.scene = true;
            }
        }
    }

    pub fn pointer_down(&mut self, x: f32, y: f32) {
        self.run_update_and_layout();
        let h = self.hit(x, y).filter(|&i| self.enabled(i));
        self.pressed = h;
        let Some(i) = h else {
            self.set_focus(None);
            return;
        };
        self.set_focus(Some(i));
        if matches!(self.flat.nodes[i].kind, NodeKind::TextInput(_)) {
            let c = self.content_rect(i);
            let now = now_ms();
            let count = match self.last_click {
                Some((t, lx, ly, n)) if now - t < 500.0 && (lx - x).abs() < 4.0 && (ly - y).abs() < 4.0 => (n % 3) + 1,
                _ => 1,
            };
            self.last_click = Some((now, x, y, count));
            if let Some(f) = self.fields.get_mut(&i) {
                match count {
                    1 => f.click(&mut self.ts, x - c.x, y - c.y, false),
                    2 => f.double_click(&mut self.ts, x - c.x, y - c.y),
                    _ => f.select_all(&mut self.ts),
                }
            }
            self.drag_field = Some(i);
            self.dirty.scene = true;
        }
    }

    pub fn pointer_up(&mut self, x: f32, y: f32) {
        self.drag_field = None;
        let down = self.pressed.take();
        if let (Some(d), Some(u)) = (down, self.hit(x, y)) {
            if d == u && !matches!(self.flat.nodes[d].kind, NodeKind::TextInput(_)) {
                self.activate(d);
            }
        }
    }

    fn activate(&mut self, i: usize) {
        if !self.enabled(i) {
            return;
        }
        self.fire(i, Event::Press);
    }

    /// Run `event` handlers on node `i`, then any `submit` it raises; mark dirty on change.
    fn fire(&mut self, i: usize, event: Event) {
        let handlers = self.flat.nodes[i].on.clone();
        let mut submits = vec![];
        let mut changed = self.beh.run_handlers(&self.module, &handlers, event, &mut submits);
        let mut guard = 0;
        while let Some(id) = submits.pop() {
            guard += 1;
            if guard > 16 {
                break;
            }
            if let Some(&t) = self.flat.by_id.get(&id) {
                let hs = self.flat.nodes[t].on.clone();
                changed |= self.beh.run_handlers(&self.module, &hs, Event::Submit, &mut submits);
            }
        }
        if changed {
            self.dirty.update = true;
            self.dirty.scene = true;
            self.dirty.a11y = true;
        }
    }

    pub fn focus_next(&mut self, back: bool) -> bool {
        self.run_update_and_layout();
        let order: Vec<usize> = (0..self.flat.len()).filter(|&i| self.focusable(i)).collect();
        if order.is_empty() {
            return false;
        }
        let pos = self.focus.and_then(|f| order.iter().position(|&i| i == f));
        let next = match (pos, back) {
            (None, false) => Some(order[0]),
            (None, true) => Some(order[order.len() - 1]),
            (Some(p), false) => order.get(p + 1).copied(),
            (Some(p), true) => p.checked_sub(1).map(|q| order[q]),
        };
        match next {
            Some(n) => {
                self.set_focus(Some(n));
                true
            }
            None => {
                self.set_focus(None);
                false
            }
        }
    }

    pub fn key(&mut self, key: Key, mods: Mods) {
        if key == Key::Tab {
            self.focus_next(mods.shift);
            return;
        }
        let Some(i) = self.focus else { return };
        if !self.vis[i] {
            return;
        }
        let is_input = matches!(self.flat.nodes[i].kind, NodeKind::TextInput(_));
        if !is_input {
            if key == Key::Enter {
                self.activate(i);
            }
            return;
        }
        let word = mods.ctrl || mods.alt;
        let mut changed = false;
        {
            let Some(f) = self.fields.get_mut(&i) else { return };
            let ts = &mut self.ts;
            match key {
                Key::Backspace => changed = f.backspace(ts, word),
                Key::Delete => changed = f.delete(ts, word),
                Key::Left => f.motion(ts, if word { Motion::WordLeft } else { Motion::Left }, mods.shift),
                Key::Right => f.motion(ts, if word { Motion::WordRight } else { Motion::Right }, mods.shift),
                Key::Home | Key::Up => f.motion(ts, Motion::Home, mods.shift),
                Key::End | Key::Down => f.motion(ts, Motion::End, mods.shift),
                Key::Char('a') | Key::Char('A') if mods.ctrl => f.select_all(ts),
                Key::Enter => {
                    self.dirty.scene = true;
                    // Enter in a single-line input raises Submit on the nearest ancestor that has a
                    // Submit handler (dev-ir CHECKPOINT: F01's form).
                    let mut cur = self.flat.parent[i];
                    while let Some(p) = cur {
                        if self.flat.nodes[p].on.iter().any(|h| h.event == Event::Submit) {
                            self.fire(p, Event::Submit);
                            break;
                        }
                        cur = self.flat.parent[p];
                    }
                    return;
                }
                _ => {}
            }
        }
        self.dirty.scene = true;
        if changed {
            self.field_changed(i);
        }
    }

    /// Committed text from the host's keyboard (printable keys). On a focused button/checkbox a
    /// single space activates it.
    pub fn text(&mut self, s: &str) {
        let Some(i) = self.focus else { return };
        if !self.vis[i] {
            return;
        }
        if matches!(self.flat.nodes[i].kind, NodeKind::TextInput(_)) {
            let changed = self.fields.get_mut(&i).map(|f| f.insert(&mut self.ts, s)).unwrap_or(false);
            self.dirty.scene = true;
            if changed {
                self.field_changed(i);
            }
        } else if s == " " {
            self.activate(i);
        }
    }

    pub fn ime_preedit(&mut self, text: &str, cursor: Option<(usize, usize)>) {
        let Some(i) = self.focus else { return };
        if let Some(f) = self.fields.get_mut(&i) {
            f.ime_preedit(&mut self.ts, text, cursor);
            self.dirty.scene = true;
        }
    }

    pub fn ime_commit(&mut self, text: &str) {
        let Some(i) = self.focus else { return };
        let changed = self.fields.get_mut(&i).map(|f| f.ime_commit(&mut self.ts, text)).unwrap_or(false);
        self.dirty.scene = true;
        if changed {
            self.field_changed(i);
        }
    }

    fn field_changed(&mut self, i: usize) {
        let bind = match &self.flat.nodes[i].kind {
            NodeKind::TextInput(p) => p.bind.clone(),
            _ => return,
        };
        let v = self.fields[&i].value().to_string();
        self.beh.state.insert(bind, Value::Text(v));
        self.beh.recompute(&self.module);
        self.fire(i, Event::Change);
        self.dirty.update = true;
        self.dirty.a11y = true;
        self.dirty.scene = true;
    }

    /// Assistive-technology action (native AccessKit `ActionRequest` or the web mirror).
    pub fn a11y_action(&mut self, req: &accesskit::ActionRequest) {
        let Some(i) = self.a11y.index_of(req.target_node) else { return };
        if !self.vis[i] {
            return;
        }
        match req.action {
            accesskit::Action::Click => {
                if self.enabled(i) {
                    self.set_focus(Some(i));
                    self.activate(i);
                }
            }
            accesskit::Action::Focus => {
                if self.focusable(i) {
                    self.set_focus(Some(i));
                }
            }
            accesskit::Action::SetValue => {
                if let Some(accesskit::ActionData::Value(v)) = &req.data {
                    if let Some(f) = self.fields.get_mut(&i) {
                        if f.set_value(&mut self.ts, v) {
                            self.field_changed(i);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------------ frame pipeline

    fn locale_string(&self, key: &str) -> String {
        self.module
            .string(&self.locale, key)
            .or_else(|| self.module.string(self.module.default_locale(), key))
            .unwrap_or(key)
            .to_string()
    }

    pub(crate) fn name_text(&self, i: usize) -> Option<String> {
        use pvc_ir::ast::Name;
        match self.flat.nodes[i].sem.name.as_ref()? {
            Name::Key(k) => Some(self.locale_string(k)),
            Name::FromNode(id) => {
                let j = *self.flat.by_id.get(id)?;
                self.texts[j].clone()
            }
        }
    }

    fn update_stage(&mut self) {
        self.beh.recompute(&self.module);
        // Visibility.
        let n = self.flat.len();
        let mut vis = vec![false; n];
        for i in 0..n {
            let parent_vis = self.flat.parent[i].map(|p| vis[p]).unwrap_or(true);
            let shown = self.flat.nodes[i].show.as_ref().map(|e| self.beh.truthy(e)).unwrap_or(true);
            vis[i] = parent_vis && shown;
        }
        // Text content.
        let mut texts: Vec<Option<String>> = vec![None; n];
        for i in 0..n {
            if let NodeKind::Text(t) = &self.flat.nodes[i].kind {
                texts[i] = Some(self.locale_string(&t.key));
            }
        }
        if vis != self.vis || texts != self.texts {
            self.dirty.layout = true;
            self.dirty.scene = true;
            self.dirty.a11y = true;
        }
        self.vis = vis;
        self.texts = texts;
        // Fields.
        for i in 0..n {
            if !self.vis[i] {
                continue;
            }
            if let NodeKind::TextInput(p) = &self.flat.nodes[i].kind {
                let (size, lh, bind, masked_e) = (p.size.px(), p.line_height.px(), p.bind.clone(), p.masked.clone());
                let loc = self.locale.clone();
                let masked = self.beh.truthy(&masked_e);
                let value = self.beh.text(&bind);
                let f = self.fields.entry(i).or_insert_with(|| Field::new(size, lh, &loc));
                f.set_locale(&loc);
                let a = f.set_masked(&mut self.ts, masked);
                let _ = a;
                if f.set_value(&mut self.ts, &value) {
                    self.dirty.scene = true;
                }
            }
        }
        self.dirty.update = false;
    }

    fn layout_stage(&mut self) {
        let s = layout::solve(&self.flat, &self.vis, &self.texts, &mut self.ts, self.vw, self.vh);
        if s.rects != self.rects {
            self.dirty.scene = true;
            self.dirty.a11y = true;
        }
        self.rects = s.rects;
        self.dirty.layout = false;
    }

    fn run_update_and_layout(&mut self) {
        if self.dirty.update {
            self.update_stage();
        }
        if self.dirty.layout {
            self.layout_stage();
        }
    }

    /// Run the dirty stages and return the display list. Hosts call this only when `needs_frame()`.
    pub fn frame(&mut self) -> &DisplayList {
        let t0 = now_ms();
        if self.dirty.update {
            self.update_stage();
        }
        let t1 = now_ms();
        if self.dirty.layout {
            self.layout_stage();
        }
        let t2 = now_ms();
        if self.dirty.scene {
            self.scene_stage();
        }
        let t3 = now_ms();
        if self.dirty.a11y {
            a11y::refresh(self);
            self.dirty.a11y = false;
        }
        let t4 = now_ms();
        self.timings = FrameTimings {
            update_ms: t1 - t0,
            layout_ms: t2 - t1,
            scene_ms: t3 - t2,
            a11y_ms: t4 - t3,
            render_ms: 0.0,
            total_ms: t4 - t0,
        };
        self.frames += 1;
        &self.dl
    }

    /// Incremental accessibility update produced by the last `frame()` (None if nothing changed).
    pub fn take_tree_update(&mut self) -> Option<accesskit::TreeUpdate> {
        self.a11y.pending.take()
    }

    /// A full tree, for adapter activation (AccessKit `ActivationHandler`) or a freshly attached mirror.
    pub fn full_tree(&mut self) -> accesskit::TreeUpdate {
        self.run_update_and_layout();
        a11y::full(self)
    }

    // ------------------------------------------------------------------ scene

    fn color(&self, c: &pvc_ir::ast::Color) -> Rgba {
        Rgba(self.module.resolve_color(c).unwrap_or([255, 0, 255, 255]))
    }

    fn theme(&self, token: &str) -> Rgba {
        Rgba(self.module.theme.colors.iter().find(|(k, _)| k == token).map(|(_, v)| *v).unwrap_or([255, 0, 255, 255]))
    }

    fn scene_stage(&mut self) {
        let mut dl = DisplayList { width: self.vw, height: self.vh, scale: self.scale, background: self.theme("bg"), ops: Vec::with_capacity(64) };
        if let Some(c) = &self.flat.nodes[0].paint.fill {
            dl.background = self.color(c);
        }
        let mut layer_ends: Vec<usize> = Vec::new();
        for i in 0..self.flat.len() {
            while layer_ends.last().map(|&e| e < i).unwrap_or(false) {
                layer_ends.pop();
                dl.ops.push(DrawOp::PopLayer);
            }
            if !self.vis[i] {
                continue;
            }
            if !self.enabled(i) {
                let mut e = i;
                while let Some(&k) = self.flat.kids[e].last() {
                    e = k;
                }
                dl.ops.push(DrawOp::PushOpacity(0.4));
                layer_ends.push(e);
            }
            let n: &Node = &self.flat.nodes[i];
            let r = self.rects[i];
            let radius = match n.paint.shape {
                Shape::RoundedRect(l) => l.px(),
                Shape::Circle => r.w.min(r.h) / 2.0,
            };
            if i != 0 || n.paint.border.is_some() {
                if let Some(c) = &n.paint.fill {
                    if i != 0 {
                        dl.ops.push(DrawOp::FillRRect { rect: r, radius, color: self.color(c) });
                    }
                }
            }
            if let Some(b) = &n.paint.border {
                dl.ops.push(DrawOp::StrokeRRect { rect: r, radius, width: b.width.px(), color: self.color(&b.color) });
            }
            let pad = &n.layout.pad;
            match &n.kind {
                NodeKind::Text(t) => {
                    let st = crate::text::TextStyle {
                        size: t.size.px(),
                        weight: t.weight,
                        line_height: t.line_height.px(),
                        align: match t.align { pvc_ir::ast::TextAlign::Start => 0, pvc_ir::ast::TextAlign::Center => 1, pvc_ir::ast::TextAlign::End => 2 },
                    };
                    let s = self.texts[i].clone().unwrap_or_default();
                    let w = (r.w - pad.left.px() - pad.right.px()).max(0.0);
                    let l = self.ts.layout(&s, &st, Some(w));
                    dl.push_layout(&l, r.x + pad.left.px(), r.y + pad.top.px(), self.color(&t.color));
                }
                NodeKind::TextInput(p) => {
                    let c = self.content_rect(i);
                    let color = self.color(&p.color);
                    let focused = self.focus == Some(i);
                    if let Some(f) = self.fields.get_mut(&i) {
                        f.layout_and_scroll(&mut self.ts, c.w);
                        let sx = f.scroll_x;
                        dl.ops.push(DrawOp::PushClip { rect: c, radius: 0.0 });
                        if focused {
                            let sel = Rgba([self.module.theme.colors.iter().find(|(k, _)| k == "focus").map(|(_, v)| v[0]).unwrap_or(56), self.module.theme.colors.iter().find(|(k, _)| k == "focus").map(|(_, v)| v[1]).unwrap_or(189), self.module.theme.colors.iter().find(|(k, _)| k == "focus").map(|(_, v)| v[2]).unwrap_or(248), 90]);
                            for (b, _) in f.ed.selection_geometry() {
                                dl.ops.push(DrawOp::FillRRect { rect: Rect::new(c.x + b.x0 as f32 - sx, c.y + b.y0 as f32, (b.x1 - b.x0) as f32, (b.y1 - b.y0) as f32), radius: 0.0, color: sel });
                            }
                        }
                        if let Some(l) = f.ed.try_layout() {
                            dl.push_layout(l, c.x - sx, c.y, color);
                        }
                        // IME preedit underline.
                        if let Some(range) = f.ed.raw_compose().clone() {
                            if let Some(l) = f.ed.try_layout() {
                                use parley::editing::{Cursor, Selection};
                                use parley::Affinity;
                                let a = Cursor::from_byte_index(l, range.start, Affinity::Downstream);
                                let b = Cursor::from_byte_index(l, range.end, Affinity::Upstream);
                                for (bb, _) in Selection::new(a, b).geometry(l) {
                                    dl.ops.push(DrawOp::FillRRect { rect: Rect::new(c.x + bb.x0 as f32 - sx, c.y + bb.y1 as f32 - 1.5, (bb.x1 - bb.x0) as f32, 1.0), radius: 0.0, color });
                                }
                            }
                        }
                        if focused {
                            if let Some(g) = f.ed.cursor_geometry(1.5) {
                                dl.ops.push(DrawOp::FillRRect { rect: Rect::new(c.x + g.x0 as f32 - sx, c.y + g.y0 as f32, (g.x1 - g.x0) as f32, (g.y1 - g.y0) as f32), radius: 0.0, color });
                            }
                        }
                        dl.ops.push(DrawOp::PopClip);
                    }
                }
                _ => {}
            }
            if self.focus == Some(i) {
                let q = Rect::new(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0);
                dl.ops.push(DrawOp::StrokeRRect { rect: q, radius: radius + 2.0, width: 2.0, color: self.theme("focus") });
            }
        }
        while layer_ends.pop().is_some() {
            dl.ops.push(DrawOp::PopLayer);
        }
        self.dl = dl;
        self.dirty.scene = false;
    }
}
