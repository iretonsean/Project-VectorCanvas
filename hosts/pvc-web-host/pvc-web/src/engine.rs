//! The seam between the web host and the runtime.
//!
//! `pvc_runtime` is the single producer of semantics (`accesskit::TreeUpdate`), scene and
//! timings (COMMON §7). This trait is the *only* thing `lib.rs`, `mirror.rs` and `overlay.rs`
//! know about it, so that the host could be built and tested before the runtime existed
//! (scaffold engine), and so that wiring the real runtime touches one file.
use accesskit::{ActionRequest, NodeId, Rect, TreeUpdate};
use vello_gpu::{Resources, Scene};

/// Per-stage timings of the last frame, milliseconds (COMMON §7 instrumentation).
#[derive(Clone, Copy, Debug, Default)]
pub struct StageTimings {
    pub update_ms: f64,
    pub layout_ms: f64,
    pub scene_ms: f64,
    pub a11y_ms: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerKind {
    Move,
    Down,
    Up,
}

/// What the engine hands back for one frame.
pub struct FrameOut {
    /// Incremental semantics, only if anything changed.
    pub tree: Option<TreeUpdate>,
    pub timings: StageTimings,
}

/// What the overlay `<input>` needs to sit over a canvas field (CSS px / UTF-16 units).
#[derive(Clone, Debug)]
pub struct InputInfo {
    pub rect: Rect,
    pub font_family: String,
    pub font_size: f64,
    pub line_height: f64,
    pub selection_utf16: (usize, usize),
    pub value: String,
    pub composing: bool,
}

pub trait Engine {
    /// Logical (CSS) viewport size and device scale factor.
    fn resize(&mut self, css_w: f32, css_h: f32, dpr: f32);
    fn pointer(&mut self, kind: PointerKind, x: f32, y: f32);
    /// Returns true if the engine consumed the key (host then calls `preventDefault`).
    fn key(&mut self, key: &str, shift: bool, ctrl: bool, alt: bool) -> bool;
    /// Committed text from keyboard / paste.
    fn text(&mut self, s: &str);
    /// IME composition: `None` text clears the preedit.
    fn ime_preedit(&mut self, s: &str, cursor: Option<(usize, usize)>);
    fn ime_commit(&mut self, s: &str);
    /// Tab / Shift+Tab. Returns false if focus left the app (host then lets the browser move focus on).
    fn focus_next(&mut self, back: bool) -> bool;
    /// DOM selection of an overlay changed (pointer/selection gesture), UTF-16 units.
    fn set_selection(&mut self, id: NodeId, start: usize, end: usize);
    /// IR id of the focused node, as an AccessKit id.
    fn focused(&self) -> Option<NodeId>;
    /// Called after the host rendered, so `total/render` are whole in the runtime's own timings.
    fn record_render_ms(&mut self, _ms: f64) {}
    /// The DOM overlay received focus (pointer or AT): make that node the focused one.
    fn focus_node(&mut self, id: NodeId);
    fn a11y_action(&mut self, req: ActionRequest);
    /// True iff a frame must be produced (dirty/animating). Idle ⇒ false ⇒ 0 frames.
    fn needs_frame(&self) -> bool;
    /// Run update→layout→scene→a11y and record the scene into `scene`.
    fn frame(&mut self, scene: &mut Scene, resources: &mut Resources) -> FrameOut;
    /// Overlay placement + font + selection for this text node (CSS px).
    fn input_info(&self, id: NodeId) -> Option<InputInfo>;
    /// `[x,y,w,h]` CSS px per alias / IR id, for the harness (`window.__pvcRects`).
    fn named_rects(&self) -> Vec<(String, [f32; 4])>;
    /// Caret rectangle in CSS px as the *canvas* draws it, if the field is focused.
    fn caret_rect(&self, id: NodeId) -> Option<Rect>;
    /// Machine-readable runtime state for test assertions (JSON).
    fn debug_state(&self) -> String;
    /// True if `Node::bounds` are physical pixels (AccessKit's documented convention); false if CSS px.
    fn bounds_are_physical(&self) -> bool;
    /// Hex fingerprint of the last display list (dev-render `DisplayList::fingerprint()`).
    fn fingerprint(&self) -> String;
    /// Focus by author alias / IR id (test hook).
    fn focus_alias(&mut self, alias: &str);
    /// Pixel size of the render target in device pixels.
    fn target_size(&self) -> (u16, u16);
}
