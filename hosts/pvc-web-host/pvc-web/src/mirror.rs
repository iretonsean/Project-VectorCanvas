//! Hidden DOM/ARIA semantics mirror (G4 §8.3, Flutter-web precedent G7 S.3).
//!
//! Input: `accesskit::TreeUpdate` produced by the runtime (the only semantics model).
//! Output: an absolutely positioned DOM tree inside `container`, one element per AccessKit
//! node, ARIA roles/names/states per the G4 §8.1 ARIA column, applied *incrementally*:
//! only nodes present in the update are touched, and only attributes that differ from the
//! node's previous version are written to the DOM.
//!
//! Text-input nodes (TextInput/EmailInput/PasswordInput/SearchInput) are realised as a real
//! `<input>` (G6 §6): that element is the overlay that receives keyboard/IME *and* the
//! accessible text field the tree points at (see `overlay.rs`).
use std::collections::HashMap;
use std::rc::Rc;

use accesskit::{Action, ActionRequest, Live, Node, NodeId, Rect, Role, TreeId, TreeUpdate, Toggled};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement};

use crate::engine::InputInfo;
use crate::overlay::{self, OverlayHandle};
use crate::HostEvent;

/// Sink for events the mirror/overlay raise toward the runtime. The host implements it.
pub trait EventSink {
    fn push(&self, ev: HostEvent);
    /// Synchronous Tab traversal; true = focus stayed in the app (caller must preventDefault).
    fn tab(&self, shift: bool) -> bool;
}

struct Entry {
    el: HtmlElement,
    node: Node,
    children: Vec<NodeId>,
    parent: Option<NodeId>,
    /// Absolute rect in CSS px (container space).
    abs: Rect,
    kind: ElKind,
    /// Kept alive as long as the element lives.
    _listeners: Vec<Closure<dyn FnMut(web_sys::Event)>>,
    overlay: Option<OverlayHandle>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ElKind {
    Div,
    Span,
    Input,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct MirrorStats {
    pub nodes_in_update: u32,
    pub nodes_changed: u32,
    pub dom_writes: u32,
    pub elements_created: u32,
    pub elements_removed: u32,
    pub total_elements: u32,
}

pub struct Mirror {
    doc: Document,
    container: HtmlElement,
    sink: Rc<dyn EventSink>,
    entries: HashMap<NodeId, Entry>,
    root: Option<NodeId>,
    /// Multiply AccessKit bounds by this to get CSS px (1/dpr if the runtime emits physical px).
    bounds_scale: f64,
    /// false = control mode (`?mirror=off`): only the `<input>` overlays exist (needed for text entry
    /// and IME regardless of the mirror, G6 §6); every other ARIA element is omitted.
    full: bool,
    /// True while we call `.focus()` ourselves, so the focus listener does not echo back.
    pub programmatic_focus: Rc<std::cell::Cell<bool>>,
    pub stats: MirrorStats,
}

fn is_text_input(role: Role) -> bool {
    matches!(
        role,
        Role::TextInput | Role::EmailInput | Role::PasswordInput | Role::SearchInput | Role::NumberInput
    )
}

fn input_type(role: Role) -> &'static str {
    match role {
        Role::EmailInput => "email",
        Role::PasswordInput => "password",
        Role::SearchInput => "search",
        _ => "text",
    }
}

/// G4 §8.1 "ARIA (web mirror)" column. `None` = no explicit role (generic/static text).
fn aria_role(role: Role) -> Option<&'static str> {
    Some(match role {
        Role::Button | Role::DefaultButton => "button",
        Role::CheckBox => "checkbox",
        Role::Switch => "switch",
        Role::RadioButton => "radio",
        Role::RadioGroup => "radiogroup",
        Role::Slider => "slider",
        Role::SpinButton => "spinbutton",
        Role::ComboBox | Role::EditableComboBox => "combobox",
        Role::ListBox => "listbox",
        Role::ListBoxOption => "option",
        Role::List => "list",
        Role::ListItem => "listitem",
        Role::Table => "table",
        Role::Row => "row",
        Role::Cell => "cell",
        Role::ColumnHeader => "columnheader",
        Role::RowHeader => "rowheader",
        Role::Grid => "grid",
        Role::GridCell => "gridcell",
        Role::Tab => "tab",
        Role::TabList => "tablist",
        Role::TabPanel => "tabpanel",
        Role::Dialog => "dialog",
        Role::AlertDialog => "alertdialog",
        Role::Alert => "alert",
        Role::Status => "status",
        Role::Heading => "heading",
        Role::Paragraph => "paragraph",
        Role::Image | Role::Canvas => "img",
        Role::ProgressIndicator => "progressbar",
        Role::Meter => "meter",
        Role::Menu => "menu",
        Role::MenuItem => "menuitem",
        Role::Toolbar => "toolbar",
        Role::Tooltip => "tooltip",
        Role::Navigation => "navigation",
        Role::Main => "main",
        Role::Region => "region",
        Role::Form => "form",
        Role::Group => "group",
        Role::Tree => "tree",
        Role::TreeItem => "treeitem",
        Role::Link => "link",
        Role::MultilineTextInput => "textbox",
        _ => return None,
    })
}

/// Roles whose accessible text is the node's `value` (AccessKit convention for Label/TextRun).
fn is_text_role(role: Role) -> bool {
    matches!(role, Role::Label | Role::TextRun | Role::Heading | Role::Paragraph | Role::Link)
}

fn el_kind(role: Role) -> ElKind {
    if is_text_input(role) {
        ElKind::Input
    } else if matches!(role, Role::Label | Role::TextRun) {
        ElKind::Span
    } else {
        ElKind::Div
    }
}

fn dom_id(id: NodeId) -> String {
    format!("pvc-n{}", id.0)
}

fn id_list(ids: &[NodeId]) -> Option<String> {
    if ids.is_empty() {
        None
    } else {
        Some(ids.iter().map(|i| dom_id(*i)).collect::<Vec<_>>().join(" "))
    }
}

type Attrs = Vec<(&'static str, Option<String>)>;

/// Everything except geometry and text content, as (attribute, value) pairs.
fn attrs(node: &Node) -> Attrs {
    let role = node.role();
    let mut a: Attrs = Vec::with_capacity(16);
    let text_role = is_text_role(role) && node.value().is_some();
    let input = is_text_input(role);
    a.push(("role", if input { None } else { aria_role(role).map(String::from) }));
    // Name. Text-run roles are named by their content (value), not by aria-label.
    a.push(("aria-label", if text_role { None } else { node.label().map(String::from) }));
    a.push(("aria-description", node.description().map(String::from)));
    a.push(("aria-roledescription", node.role_description().map(String::from)));
    a.push(("lang", node.language().map(String::from)));
    a.push(("aria-labelledby", id_list(node.labelled_by())));
    a.push(("aria-describedby", id_list(node.described_by())));
    a.push(("aria-controls", id_list(node.controls())));
    a.push(("aria-errormessage", node.error_message().map(dom_id)));
    // States.
    let toggled = node.toggled().map(|t| {
        match t {
            Toggled::False => "false",
            Toggled::True => "true",
            Toggled::Mixed => "mixed",
        }
        .to_string()
    });
    if matches!(role, Role::CheckBox | Role::Switch | Role::RadioButton | Role::MenuItemCheckBox | Role::MenuItemRadio) {
        a.push(("aria-checked", toggled));
        a.push(("aria-pressed", None));
    } else {
        a.push(("aria-pressed", toggled));
        a.push(("aria-checked", None));
    }
    let b = |v: bool| if v { Some("true".to_string()) } else { None };
    a.push(("aria-disabled", if input { None } else { b(node.is_disabled()) }));
    a.push(("disabled", if input { b(node.is_disabled()).map(|_| String::new()) } else { None }));
    a.push(("aria-required", b(node.is_required())));
    a.push(("aria-readonly", b(node.is_read_only())));
    a.push(("aria-invalid", node.invalid().map(|_| "true".to_string())));
    a.push(("aria-expanded", node.is_expanded().map(|v| v.to_string())));
    a.push(("aria-selected", node.is_selected().map(|v| v.to_string())));
    a.push(("aria-busy", b(node.is_busy())));
    a.push(("aria-modal", b(node.is_modal())));
    a.push(("aria-hidden", b(node.is_hidden())));
    a.push((
        "aria-live",
        node.live().map(|l| {
            match l {
                Live::Off => "off",
                Live::Polite => "polite",
                Live::Assertive => "assertive",
            }
            .to_string()
        }),
    ));
    a.push(("aria-level", node.level().map(|l| l.to_string())));
    a.push(("aria-valuenow", node.numeric_value().map(|v| v.to_string())));
    a.push(("aria-valuemin", node.min_numeric_value().map(|v| v.to_string())));
    a.push(("aria-valuemax", node.max_numeric_value().map(|v| v.to_string())));
    // Focusability: programmatic focus only (the runtime owns Tab order). Inputs are natively focusable.
    a.push((
        "tabindex",
        if !input && node.supports_action(Action::Focus) { Some("-1".to_string()) } else { None },
    ));
    if input {
        a.push(("type", Some(input_type(role).to_string())));
    }
    a
}

fn px(v: f64) -> String {
    // 1/64 px quantisation matches the solver's snap; trims noise in the style string.
    let q = (v * 64.0).round() / 64.0;
    format!("{q}")
}

impl Mirror {
    pub fn new(doc: Document, container: HtmlElement, sink: Rc<dyn EventSink>, bounds_scale: f64, full: bool) -> Self {
        Mirror {
            full,
            doc,
            container,
            sink,
            entries: HashMap::new(),
            root: None,
            bounds_scale,
            programmatic_focus: Rc::new(std::cell::Cell::new(false)),
            stats: MirrorStats::default(),
        }
    }

    pub fn is_full(&self) -> bool {
        self.full
    }

    pub fn set_bounds_scale(&mut self, s: f64) {
        self.bounds_scale = s;
    }

    pub fn element_count(&self) -> usize {
        self.entries.len()
    }

    /// Keep the DOM selection of the focused overlay equal to the runtime's (arrow keys, select-all).
    pub fn sync_selection(&self, id: NodeId, sel: (usize, usize)) {
        if let Some(ov) = self.entries.get(&id).and_then(|e| e.overlay.as_ref()) {
            if !ov.composing.get() {
                let cur = (ov.input.selection_start().ok().flatten().unwrap_or(0) as usize, ov.input.selection_end().ok().flatten().unwrap_or(0) as usize);
                if cur != sel {
                    let _ = ov.input.set_selection_range(sel.0 as u32, sel.1 as u32);
                }
            }
        }
    }

    pub fn resync_input(&self, id: NodeId) {
        if let Some(e) = self.entries.get(&id) {
            if let Some(ov) = &e.overlay {
                if !ov.composing.get() {
                    let v = e.node.value().unwrap_or("");
                    if ov.input.value() != v {
                        ov.input.set_value(v);
                        let n = v.encode_utf16().count() as u32;
                        let _ = ov.input.set_selection_range(n, n);
                    }
                }
            }
        }
    }

    fn scaled(&self, r: Rect) -> Rect {
        let s = self.bounds_scale;
        Rect { x0: r.x0 * s, y0: r.y0 * s, x1: r.x1 * s, y1: r.y1 * s }
    }

    fn create_element(&mut self, id: NodeId, role: Role) -> Result<(HtmlElement, ElKind, Vec<Closure<dyn FnMut(web_sys::Event)>>, Option<OverlayHandle>), JsValue> {
        let kind = el_kind(role);
        let tag = match kind {
            ElKind::Div => "div",
            ElKind::Span => "span",
            ElKind::Input => "input",
        };
        let el: HtmlElement = self.doc.create_element(tag)?.unchecked_into();
        el.set_id(&dom_id(id));
        el.set_class_name(if kind == ElKind::Input { "pvcm pvci" } else { "pvcm" });
        self.stats.elements_created += 1;
        let mut listeners = Vec::new();
        let mut ov = None;
        if kind == ElKind::Input {
            let input: HtmlInputElement = el.clone().unchecked_into();
            ov = Some(overlay::attach(&input, id, self.sink.clone(), self.programmatic_focus.clone())?);
        } else {
            // AT "click" (e.g. screen-reader activate) arrives as a DOM click on the mirror element.
            let sink = self.sink.clone();
            let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |e: web_sys::Event| {
                // `click` bubbles: only the element that was itself activated counts, not its ancestors.
                if e.target() != e.current_target() {
                    return;
                }
                sink.push(HostEvent::Action(ActionRequest {
                    action: Action::Click,
                    target_tree: TreeId::ROOT,
                    target_node: id,
                    data: None,
                }));
            });
            el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
            listeners.push(cb);
            // AT / programmatic focus on a mirror element is reported to the runtime.
            let sink = self.sink.clone();
            let echo = self.programmatic_focus.clone();
            let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |_e: web_sys::Event| {
                if !echo.get() {
                    sink.push(HostEvent::Focus(id));
                }
            });
            el.add_event_listener_with_callback("focus", cb.as_ref().unchecked_ref())?;
            listeners.push(cb);
        }
        Ok((el, kind, listeners, ov))
    }

    fn set_attr(el: &Element, name: &str, v: &Option<String>, writes: &mut u32) {
        *writes += 1;
        match v {
            Some(s) => {
                let _ = el.set_attribute(name, s);
            }
            None => {
                let _ = el.remove_attribute(name);
            }
        }
    }

    fn remove_subtree(&mut self, id: NodeId) {
        if let Some(e) = self.entries.remove(&id) {
            self.stats.elements_removed += 1;
            e.el.remove();
            for c in e.children {
                self.remove_subtree(c);
            }
        }
    }

    /// Apply one incremental update. `input_rect(id)` returns the runtime's `text_input_rect`.
    pub fn apply(
        &mut self,
        upd: &TreeUpdate,
        input_info: &dyn Fn(NodeId) -> Option<InputInfo>,
    ) -> Result<(), JsValue> {
        self.stats = MirrorStats::default();
        self.stats.nodes_in_update = upd.nodes.len() as u32;
        if let Some(t) = &upd.tree {
            if self.root != Some(t.root) {
                // New tree: drop everything.
                let ids: Vec<NodeId> = self.entries.keys().copied().collect();
                for id in ids {
                    if let Some(e) = self.entries.remove(&id) {
                        e.el.remove();
                    }
                }
                self.root = Some(t.root);
            }
        }
        let mut writes = 0u32;

        // Pass 1: create/update node elements and their own attributes.
        let mut touched: Vec<NodeId> = Vec::with_capacity(upd.nodes.len());
        for (id, node) in &upd.nodes {
            let id = *id;
            let is_root = self.root == Some(id);
            if !self.full && !is_root && !is_text_input(node.role()) {
                continue;
            }
            let role_changed = self
                .entries
                .get(&id)
                .map(|e| el_kind(e.node.role()) != el_kind(node.role()))
                .unwrap_or(false);
            if role_changed {
                // Rare: swap element kind. Drop the old element, keep child ids, recreate.
                if let Some(old) = self.entries.remove(&id) {
                    old.el.remove();
                    self.stats.elements_removed += 1;
                }
            }
            if !self.entries.contains_key(&id) {
                let (el, kind, listeners, ov) = if is_root {
                    (self.container.clone(), ElKind::Div, Vec::new(), None)
                } else {
                    self.create_element(id, node.role())?
                };
                self.entries.insert(
                    id,
                    Entry {
                        el,
                        node: Node::new(Role::Unknown),
                        children: Vec::new(),
                        parent: None,
                        abs: Rect { x0: 0.0, y0: 0.0, x1: 0.0, y1: 0.0 },
                        kind,
                        _listeners: listeners,
                        overlay: ov,
                    },
                );
            }
            let e = self.entries.get_mut(&id).unwrap();
            if e.node != *node {
                self.stats.nodes_changed += 1;
                let old = attrs(&e.node);
                let new = attrs(node);
                let fresh = e.node.role() == Role::Unknown && e.node.children().is_empty();
                for (i, (name, v)) in new.iter().enumerate() {
                    if fresh || old.get(i).map(|o| o.0 != *name || o.1 != *v).unwrap_or(true) {
                        if is_root && *name == "tabindex" {
                            continue;
                        }
                        if e.kind == ElKind::Input && *name == "type" && !fresh {
                            // changing `type` on a live input is legal but rare; write only when differing
                        }
                        Self::set_attr(&e.el, name, v, &mut writes);
                    }
                }
                // Text content for text roles.
                if e.kind != ElKind::Input && is_text_role(node.role()) && node.children().is_empty() {
                    let new_t = node.value().unwrap_or("");
                    let old_t = if e.node.children().is_empty() { e.node.value().unwrap_or("") } else { "" };
                    if fresh || old_t != new_t {
                        e.el.set_text_content(Some(new_t));
                        writes += 1;
                    }
                }
                // Input value (not while an IME composition is in flight).
                if let Some(ov) = &e.overlay {
                    if let Some(v) = node.value() {
                        if !ov.composing.get() && ov.input.value() != v {
                            ov.input.set_value(v);
                            let n = v.encode_utf16().count() as u32;
                            let _ = ov.input.set_selection_range(n, n);
                            writes += 1;
                        }
                    } else if !ov.composing.get() && !ov.input.value().is_empty() {
                        ov.input.set_value("");
                        writes += 1;
                    }
                }
                e.node = node.clone();
            }
            touched.push(id);
        }

        // Pass 2: child lists (ordering + removals).
        if !self.full {
            for (id, node) in &upd.nodes {
                if is_text_input(node.role()) {
                    if let Some(e) = self.entries.get(id) {
                        if e.el.parent_node().is_none() {
                            let _ = self.container.append_child(&e.el);
                        }
                    }
                }
            }
        }
        let full = self.full;
        for (id, node) in upd.nodes.iter().filter(|_| full) {
            let new_children: Vec<NodeId> = node.children().to_vec();
            let (old_children, el) = {
                let e = self.entries.get(id).unwrap();
                (e.children.clone(), e.el.clone())
            };
            if old_children != new_children {
                for c in &old_children {
                    if !new_children.contains(c) {
                        // Only drop if the child is not re-parented elsewhere in this same update.
                        let reparented = upd.nodes.iter().any(|(pid, pn)| pid != id && pn.children().contains(c));
                        if !reparented {
                            self.remove_subtree(*c);
                        }
                    }
                }
                for c in &new_children {
                    if let Some(ce) = self.entries.get_mut(c) {
                        ce.parent = Some(*id);
                        // appendChild moves an existing node, so this also establishes order.
                        let _ = el.append_child(&ce.el);
                        writes += 1;
                    }
                }
                self.entries.get_mut(id).unwrap().children = new_children;
            }
        }

        // Pass 3: geometry. Absolute rect per touched node, then relative style; children of a
        // moved node are restyled because their style is relative to the parent.
        let mut to_style: Vec<NodeId> = Vec::new();
        for id in &touched {
            let (abs_new, changed) = {
                let e = self.entries.get(id).unwrap();
                let abs = if is_text_input(e.node.role()) {
                    input_info(*id).map(|i| i.rect).or_else(|| e.node.bounds().map(|b| self.scaled(b)))
                } else {
                    e.node.bounds().map(|b| self.scaled(b))
                };
                match abs {
                    Some(r) => (r, r != e.abs),
                    None => (e.abs, false),
                }
            };
            if changed {
                self.entries.get_mut(id).unwrap().abs = abs_new;
                to_style.push(*id);
                let kids = self.entries.get(id).unwrap().children.clone();
                to_style.extend(kids);
            } else if self.entries.get(id).map(|e| e.parent.is_some() && e.el.get_attribute("style").is_none()).unwrap_or(false) {
                to_style.push(*id);
            }
        }
        to_style.sort_by_key(|n| n.0);
        to_style.dedup();
        for id in to_style {
            let (abs, parent_abs, is_root) = {
                let e = self.entries.get(&id).unwrap();
                let pa = e.parent.and_then(|p| self.entries.get(&p)).map(|p| p.abs);
                (e.abs, pa, self.root == Some(id))
            };
            let (ox, oy) = match (is_root, parent_abs) {
                (false, Some(p)) => (p.x0, p.y0),
                _ => (0.0, 0.0),
            };
            let mut style = format!(
                "left:{}px;top:{}px;width:{}px;height:{}px",
                px(abs.x0 - ox),
                px(abs.y0 - oy),
                px(abs.x1 - abs.x0),
                px(abs.y1 - abs.y0)
            );
            // Overlay inputs must lay text out exactly like the canvas: same face/size, no padding,
            // line box = field height (G6 risk 5). Font comes from the runtime's node.
            if let Some(e) = self.entries.get(&id) {
                if e.kind == ElKind::Input {
                    let (fs, fam, lh) = match input_info(id) {
                        Some(i) => (i.font_size, i.font_family, i.line_height),
                        None => (16.0, "sans-serif".to_string(), abs.y1 - abs.y0),
                    };
                    style.push_str(&format!(
                        ";font:{}px {};line-height:{}px;padding:0;text-indent:0",
                        px(fs),
                        fam,
                        px(lh)
                    ));
                }
            }
            if !is_root {
                let e = self.entries.get(&id).unwrap();
                let _ = e.el.set_attribute("style", &style);
                writes += 1;
            }
        }

        // Focus: the runtime's focused node becomes the DOM-focused element.
        if self.root != Some(upd.focus) {
            if let Some(e) = self.entries.get(&upd.focus) {
                let active = self.doc.active_element();
                let already = active.map(|a| a.is_same_node(Some(e.el.as_ref()))).unwrap_or(false);
                if !already {
                    self.programmatic_focus.set(true);
                    let opts = web_sys::FocusOptions::new();
                    opts.set_prevent_scroll(true);
                    let _ = e.el.focus_with_options(&opts);
                    self.programmatic_focus.set(false);
                    writes += 1;
                }
            }
        }
        self.stats.dom_writes = writes;
        self.stats.total_elements = self.entries.len() as u32;
        Ok(())
    }
}
