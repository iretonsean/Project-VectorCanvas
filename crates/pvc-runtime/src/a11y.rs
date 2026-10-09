//! Semantics: IR roles → `accesskit::TreeUpdate` (G4 §8.1). This is the single semantics model:
//! the native host feeds it to `accesskit_winit`; the web host converts it to the hidden DOM/ARIA
//! mirror. Node ids are stable: FNV-1a 64 of the IR node id. Every node carries `author_id` = its
//! alias (`name`, `email`, `submit`, ... see `Runtime::node_idx`) or else the IR node id string.
//!
//! Conventions: `bounds` are LOGICAL px in surface coordinates (multiply by the scale factor for
//! physical px). Updates are incremental (changed/new nodes + changed parents + focus). Nodes with
//! `live != off` stay in the tree even while their `show` is false (empty label, no bounds), so
//! assistive technology sees the live region exist before its content changes.

use crate::model::{node_id_hash, Flat};
use crate::runtime::Runtime;
use accesskit::{Action, Invalid, Node, NodeId, Role, Toggled, TreeInfo, TreeUpdate};
use pvc_ir::ast as ir;
use std::collections::HashMap;

#[derive(Default)]
pub struct A11yState {
    prev: HashMap<NodeId, Node>,
    prev_focus: Option<NodeId>,
    sent_first: bool,
    ids: HashMap<NodeId, usize>,
    pub(crate) pending: Option<TreeUpdate>,
}

impl A11yState {
    pub fn index_of(&self, id: NodeId) -> Option<usize> {
        self.ids.get(&id).copied()
    }
}

pub fn check_ids(flat: &Flat) -> Result<(), String> {
    let mut seen: HashMap<u64, &str> = HashMap::new();
    for n in &flat.nodes {
        if let Some(prev) = seen.insert(node_id_hash(&n.id), &n.id) {
            return Err(format!("accesskit id collision between {} and {}", prev, n.id));
        }
    }
    Ok(())
}

pub fn node_id(id: &str) -> NodeId {
    NodeId(node_id_hash(id))
}

fn ak_role(r: ir::Role, is_root: bool) -> Role {
    if is_root {
        return Role::Window;
    }
    match r {
        ir::Role::Button => Role::Button,
        ir::Role::Link => Role::Link,
        ir::Role::Checkbox => Role::CheckBox,
        ir::Role::Switch => Role::Switch,
        ir::Role::TextField => Role::TextInput,
        ir::Role::PasswordField => Role::PasswordInput,
        ir::Role::EmailField => Role::EmailInput,
        ir::Role::Group => Role::Group,
        ir::Role::Container => Role::GenericContainer,
        ir::Role::Image => Role::Image,
        ir::Role::Label => Role::Label,
        ir::Role::Paragraph => Role::Paragraph,
        ir::Role::Heading => Role::Heading,
        ir::Role::Status => Role::Status,
        ir::Role::Alert => Role::Alert,
        ir::Role::Form => Role::Form,
        ir::Role::Main => Role::Main,
        ir::Role::Navigation => Role::Navigation,
        ir::Role::Region => Role::Region,
        ir::Role::Radio => Role::RadioButton,
        ir::Role::RadioGroup => Role::RadioGroup,
        ir::Role::TextArea => Role::MultilineTextInput,
        ir::Role::SearchField => Role::SearchInput,
        ir::Role::List => Role::List,
        ir::Role::ListItem => Role::ListItem,
        ir::Role::TabList => Role::TabList,
        ir::Role::Tab => Role::Tab,
        ir::Role::TabPanel => Role::TabPanel,
        ir::Role::Dialog => Role::Dialog,
        ir::Role::AlertDialog => Role::AlertDialog,
    }
}

fn is_atomic_control(r: ir::Role) -> bool {
    matches!(r, ir::Role::Button | ir::Role::Checkbox | ir::Role::Switch | ir::Role::Link)
}

struct Built {
    nodes: Vec<(NodeId, Node)>,
    ids: HashMap<NodeId, usize>,
}

/// Build the full node set from current state.
fn build(rt: &Runtime) -> Built {
    let flat = &rt.flat;
    let n = flat.len();
    // Which nodes are in the tree.
    let mut included = vec![false; n];
    let mut hidden_live = vec![false; n];
    for i in 0..n {
        let node = &flat.nodes[i];
        let parent_in = flat.parent[i].map(|p| included[p]).unwrap_or(true);
        if !parent_in {
            continue;
        }
        // Children of atomic controls and decorative subtrees are presentational.
        if let Some(p) = flat.parent[i] {
            let pn = &flat.nodes[p];
            if is_atomic_control(pn.sem.role) && !pn.sem.decorative {
                continue;
            }
        }
        if node.sem.decorative {
            continue;
        }
        if rt.vis[i] {
            included[i] = true;
        } else if node.sem.live != ir::Live::Off {
            included[i] = true;
            hidden_live[i] = true;
        }
    }
    let mut nodes = Vec::new();
    let mut ids = HashMap::new();
    for i in 0..n {
        if !included[i] {
            continue;
        }
        let src = &flat.nodes[i];
        let nid = node_id(&src.id);
        ids.insert(nid, i);
        let mut node = Node::new(ak_role(src.sem.role, i == 0));
        node.set_author_id(rt.aliases[i].clone().unwrap_or_else(|| src.id.clone()));
        if !hidden_live[i] {
            let r = rt.rects[i];
            node.set_bounds(accesskit::Rect { x0: r.x as f64, y0: r.y as f64, x1: r.right() as f64, y1: r.bottom() as f64 });
        }
        let kids: Vec<NodeId> = flat.kids[i].iter().filter(|&&c| included[c]).map(|&c| node_id(&flat.nodes[c].id)).collect();
        if !kids.is_empty() {
            node.set_children(kids);
        }
        // Name / content.
        let own_text = rt.texts[i].clone().filter(|_| !hidden_live[i]);
        let name = if hidden_live[i] { None } else { rt.name_text(i) };
        match src.sem.role {
            ir::Role::Label => {
                if let Some(t) = own_text.clone().or(name.clone()) {
                    node.set_value(t);
                }
            }
            _ => {
                if let Some(t) = name.clone().or(own_text.clone()) {
                    node.set_label(t);
                }
            }
        }
        if hidden_live[i] && !matches!(src.sem.role, ir::Role::Label) {
            node.set_label("");
        }
        if let Some(d) = &src.sem.description {
            node.set_description(rt.module.string(rt.locale(), d).unwrap_or(d.as_str()).to_string());
        }
        if let Some(l) = src.sem.heading_level {
            node.set_level(l as usize);
        }
        match src.sem.live {
            ir::Live::Off => {}
            ir::Live::Polite => node.set_live(accesskit::Live::Polite),
            ir::Live::Assertive => node.set_live(accesskit::Live::Assertive),
        }
        if src.sem.required {
            node.set_required();
        }
        let truthy = |e: &Option<ir::Expr>| e.as_ref().map(|e| rt.beh.truthy(e));
        if truthy(&src.sem.disabled) == Some(true) {
            node.set_disabled();
        }
        if let Some(c) = truthy(&src.sem.checked) {
            node.set_toggled(Toggled::from(c));
        }
        if let Some(p) = truthy(&src.sem.pressed) {
            node.set_toggled(Toggled::from(p));
        }
        if truthy(&src.sem.invalid) == Some(true) {
            node.set_invalid(Invalid::True);
        }
        if let Some(l) = &src.sem.labelled_by {
            if let Some(&j) = flat.by_id.get(l) {
                if included[j] {
                    node.set_labelled_by(vec![node_id(l)]);
                }
            }
        }
        let desc: Vec<NodeId> = src.sem.described_by.iter().filter(|d| flat.by_id.get(*d).map(|&j| included[j]).unwrap_or(false)).map(|d| node_id(d)).collect();
        if !desc.is_empty() {
            node.set_described_by(desc);
        }
        if let Some(e) = &src.sem.error_message {
            if flat.by_id.get(e).map(|&j| included[j]).unwrap_or(false) {
                node.set_error_message(node_id(e));
            }
        }
        // Actions and value.
        if src.is_interactive() {
            node.add_action(Action::Focus);
            if matches!(src.sem.role, ir::Role::Button | ir::Role::Checkbox | ir::Role::Switch | ir::Role::Link) {
                node.add_action(Action::Click);
            }
        }
        if let ir::NodeKind::TextInput(p) = &src.kind {
            node.add_action(Action::SetValue);
            let v = match rt.fields.get(&i) {
                Some(f) if f.is_masked() => f.display_text().to_string(),
                Some(f) => f.value().to_string(),
                None => rt.beh.text(&p.bind),
            };
            node.set_value(v);
        }
        nodes.push((nid, node));
    }
    Built { nodes, ids }
}

fn focus_id(rt: &Runtime) -> NodeId {
    match rt.focus {
        Some(i) => node_id(&rt.flat.nodes[i].id),
        None => node_id(&rt.flat.nodes[0].id),
    }
}

fn tree_info(rt: &Runtime) -> TreeInfo {
    let mut t = TreeInfo::new(node_id(&rt.flat.nodes[0].id));
    t.toolkit_name = Some("pvc-runtime".into());
    t.toolkit_version = Some(env!("CARGO_PKG_VERSION").into());
    t
}

/// Rebuild the semantics and stage an incremental update (called by `Runtime::frame`).
pub fn refresh(rt: &mut Runtime) {
    let b = build(rt);
    let focus = focus_id(rt);
    let first = !rt.a11y.sent_first;
    let mut changed: Vec<(NodeId, Node)> = Vec::new();
    let mut new_map: HashMap<NodeId, Node> = HashMap::with_capacity(b.nodes.len());
    for (id, node) in b.nodes {
        if first || rt.a11y.prev.get(&id) != Some(&node) {
            changed.push((id, node.clone()));
        }
        new_map.insert(id, node);
    }
    let removed = rt.a11y.prev.keys().any(|k| !new_map.contains_key(k));
    if changed.is_empty() && !removed && rt.a11y.prev_focus == Some(focus) && !first {
        rt.a11y.ids = b.ids;
        return;
    }
    rt.a11y.pending = Some(TreeUpdate {
        nodes: changed,
        tree: first.then(|| tree_info(rt)),
        tree_id: accesskit::TreeId::ROOT,
        focus,
    });
    rt.a11y.prev = new_map;
    rt.a11y.prev_focus = Some(focus);
    rt.a11y.sent_first = true;
    rt.a11y.ids = b.ids;
}

/// A complete update (all current nodes).
pub fn full(rt: &mut Runtime) -> TreeUpdate {
    let b = build(rt);
    rt.a11y.ids = b.ids;
    TreeUpdate { nodes: b.nodes, tree: Some(tree_info(rt)), tree_id: accesskit::TreeId::ROOT, focus: focus_id(rt) }
}
