//! `Engine` implementation over the real `pvc_runtime::Runtime` (dev-render CHECKPOINT facade).
use accesskit::{ActionRequest, NodeId, Rect as AkRect};
use pvc_runtime::render::replay_gpu;
use pvc_runtime::{Key, Mods, Options, Runtime};
use vello_gpu::{Resources, Scene};

use crate::engine::{Engine, FrameOut, InputInfo, PointerKind, StageTimings};

pub struct RuntimeEngine {
    rt: Runtime,
    first: bool,
    w: f32,
    h: f32,
    dpr: f32,
}

impl RuntimeEngine {
    pub fn new(artifact: &[u8], locale: &str, w: f32, h: f32, dpr: f32) -> Result<Self, String> {
        // Trust root in this session: the published TEST key (pvc_ir::sign::testkey).
        let key = pvc_ir::sign::testkey::verifying_key();
        let rt = Runtime::from_signed(artifact, |b| pvc_ir::load_signed(b, &key), Options { locale: locale.to_string(), width: w, height: h, scale: dpr })
            .map_err(|e| format!("Runtime::from_signed: {e}"))?;
        for w in &rt.load_warnings {
            web_sys::console::warn_1(&format!("pvc_runtime load warning: {w}").into());
        }
        Ok(RuntimeEngine { rt, first: true, w, h, dpr })
    }

    fn ir_id(&self, id: NodeId) -> Option<String> {
        self.rt.ir_id_of(id).map(String::from)
    }
}

impl Engine for RuntimeEngine {
    fn resize(&mut self, w: f32, h: f32, dpr: f32) {
        self.w = w;
        self.h = h;
        self.dpr = dpr;
        self.rt.resize(w, h, dpr);
    }
    fn pointer(&mut self, kind: PointerKind, x: f32, y: f32) {
        match kind {
            PointerKind::Move => self.rt.pointer_move(x, y),
            PointerKind::Down => self.rt.pointer_down(x, y),
            PointerKind::Up => self.rt.pointer_up(x, y),
        }
    }
    fn key(&mut self, key: &str, shift: bool, ctrl: bool, alt: bool) -> bool {
        let mods = Mods { shift, ctrl, alt };
        let k = match key {
            "Enter" => Key::Enter,
            "Backspace" => Key::Backspace,
            "Delete" => Key::Delete,
            "ArrowLeft" => Key::Left,
            "ArrowRight" => Key::Right,
            "ArrowUp" => Key::Up,
            "ArrowDown" => Key::Down,
            "Home" => Key::Home,
            "End" => Key::End,
            "Escape" => Key::Escape,
            s if s.chars().count() == 1 => {
                let c = s.chars().next().unwrap();
                if ctrl {
                    Key::Char(c)
                } else {
                    // printable key with no overlay focus (e.g. Space on a button): committed text path
                    self.rt.text(s);
                    return true;
                }
            }
            _ => return false,
        };
        self.rt.key(k, mods);
        true
    }
    fn focus_next(&mut self, back: bool) -> bool {
        self.rt.focus_next(back)
    }
    fn set_selection(&mut self, id: NodeId, start: usize, end: usize) {
        if let Some(s) = self.ir_id(id) {
            self.rt.set_selection(&s, start, end);
        }
    }
    fn focused(&self) -> Option<NodeId> {
        self.rt.focused_id().map(|s| self.rt.node_id(s))
    }
    fn text(&mut self, s: &str) {
        self.rt.text(s);
    }
    fn ime_preedit(&mut self, s: &str, cursor: Option<(usize, usize)>) {
        self.rt.ime_preedit(s, cursor);
    }
    fn ime_commit(&mut self, s: &str) {
        self.rt.ime_commit(s);
    }
    fn focus_node(&mut self, id: NodeId) {
        if let Some(s) = self.ir_id(id) {
            self.rt.focus_id(&s);
        }
    }
    fn a11y_action(&mut self, req: ActionRequest) {
        self.rt.a11y_action(&req);
    }
    fn needs_frame(&self) -> bool {
        self.rt.needs_frame()
    }
    fn frame(&mut self, scene: &mut Scene, resources: &mut Resources) -> FrameOut {
        {
            let dl = self.rt.frame();
            replay_gpu(dl, scene, resources);
        }
        let tree = if self.first {
            self.first = false;
            Some(self.rt.take_tree_update().unwrap_or_else(|| self.rt.full_tree()))
        } else {
            self.rt.take_tree_update()
        };
        let t = self.rt.timings();
        FrameOut { tree, timings: StageTimings { update_ms: t.update_ms, layout_ms: t.layout_ms, scene_ms: t.scene_ms, a11y_ms: t.a11y_ms } }
    }
    fn record_render_ms(&mut self, ms: f64) {
        self.rt.record_render_ms(ms);
    }
    fn input_info(&self, id: NodeId) -> Option<InputInfo> {
        let s = self.ir_id(id)?;
        let i = self.rt.text_input_info(&s)?;
        Some(InputInfo {
            rect: AkRect { x0: i.rect.x as f64, y0: i.rect.y as f64, x1: (i.rect.x + i.rect.w) as f64, y1: (i.rect.y + i.rect.h) as f64 },
            font_family: i.font_family,
            font_size: i.font_size as f64,
            line_height: i.line_height as f64,
            selection_utf16: i.selection_utf16,
            value: i.value,
            composing: i.composing,
        })
    }
    fn named_rects(&self) -> Vec<(String, [f32; 4])> {
        self.rt.named_rects()
    }
    fn caret_rect(&self, id: NodeId) -> Option<AkRect> {
        let s = self.ir_id(id)?;
        let r = self.rt.caret_rect(&s)?;
        Some(AkRect { x0: r.x as f64, y0: r.y as f64, x1: (r.x + r.w) as f64, y1: (r.y + r.h) as f64 })
    }
    fn debug_state(&self) -> String {
        // runtime state is line-oriented text; wrap as JSON with the parsed focus/name/etc. left to the test
        let st = self.rt.debug_state();
        let esc = st.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
        let fi = self.focused().and_then(|id| self.input_info(id));
        let fi = match fi {
            Some(i) => {
                let v = i.value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
                format!("{{\"value\":\"{v}\",\"composing\":{},\"selection\":[{},{}]}}", i.composing, i.selection_utf16.0, i.selection_utf16.1)
            }
            None => "null".to_string(),
        };
        format!("{{\"engine\":\"pvc_runtime\",\"raw\":\"{esc}\",\"focused_input\":{fi}}}")
    }
    fn fingerprint(&self) -> String {
        self.rt.display_list().fingerprint()
    }
    fn focus_alias(&mut self, alias: &str) {
        self.rt.focus_id(alias);
    }
    fn bounds_are_physical(&self) -> bool {
        false // dev-render CHECKPOINT: bounds are LOGICAL px
    }
    fn target_size(&self) -> (u16, u16) {
        ((self.w * self.dpr).ceil() as u16, (self.h * self.dpr).ceil() as u16)
    }
}
