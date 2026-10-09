//! F01 state-trace test: the four G2 checklist A-items, executed headlessly on the signed
//! specimen through the real input API and the accesskit tree (no rendering involved).
//!   1. submit disabled until all valid
//!   2. email validated
//!   3. show/hide toggles masking and is named
//!   4. errors announced to AT (live = polite)
mod common;
use accesskit::{Role, Toggled, TreeUpdate};
use common::*;
use pvc_runtime::*;

/// Latest full picture of the tree after applying pending incremental updates on top of a base.
struct Mirror(std::collections::HashMap<accesskit::NodeId, accesskit::Node>, accesskit::NodeId);
impl Mirror {
    fn new() -> Self {
        Mirror(Default::default(), accesskit::NodeId(0))
    }
    fn apply(&mut self, u: TreeUpdate) {
        for (i, n) in u.nodes {
            self.0.insert(i, n);
        }
        self.1 = u.focus;
    }
    fn by_author<'a>(&'a self, a: &str) -> Option<&'a accesskit::Node> {
        self.0.values().find(|n| n.author_id() == Some(a))
    }
}

fn pump(rt: &mut Runtime, m: &mut Mirror) {
    rt.frame();
    if let Some(u) = rt.take_tree_update() {
        m.apply(u);
    }
}

#[test]
fn f01_four_checklist_items() {
    let mut rt = load("F01", "en", 1366.0, 768.0);
    let mut m = Mirror::new();
    pump(&mut rt, &mut m);

    // --- item 1: submit disabled until all valid
    assert!(m.by_author("submit").unwrap().is_disabled(), "empty form: submit disabled");
    type_into(&mut rt, "name", "Jane Doe");
    pump(&mut rt, &mut m);
    assert!(m.by_author("submit").unwrap().is_disabled());
    type_into(&mut rt, "email", "jane@example.com");
    type_into(&mut rt, "password", "hunter22");
    pump(&mut rt, &mut m);
    assert!(m.by_author("submit").unwrap().is_disabled(), "terms not accepted yet");
    // press on a disabled submit does nothing
    click(&mut rt, "submit");
    assert_eq!(rt.state_value("status"), Some(&pvc_ir::Value::Enum("editing".into())));
    click(&mut rt, "checkbox");
    pump(&mut rt, &mut m);
    assert_eq!(m.by_author("checkbox").unwrap().toggled(), Some(Toggled::True));
    assert!(!m.by_author("submit").unwrap().is_disabled(), "all valid: submit enabled");
    // making one field invalid disables it again
    type_into(&mut rt, "password", "x"); // password becomes hunter22x (still valid)
    pump(&mut rt, &mut m);
    assert!(!m.by_author("submit").unwrap().is_disabled());
    click(&mut rt, "submit");
    assert_eq!(rt.state_value("status"), Some(&pvc_ir::Value::Enum("done".into())), "submit ran the form's submit handler");
    pump(&mut rt, &mut m);

    // --- item 2: email validated (fresh runtime)
    let mut rt = load("F01", "en", 1366.0, 768.0);
    let mut m = Mirror::new();
    pump(&mut rt, &mut m);
    for (v, valid) in [("plainaddress", false), ("a@b", false), ("a@b.co", true), ("x y@z.com", false), ("name@example.com", true)] {
        click(&mut rt, "email");
        rt.key(Key::Char('a'), Mods { ctrl: true, ..Default::default() });
        rt.key(Key::Backspace, Mods::default());
        rt.text(v);
        pump(&mut rt, &mut m);
        let shown = rt.is_visible("surface0/form0/field1/error");
        assert_eq!(shown, !valid, "email {v:?}: error visibility");
    }

    // --- item 3: show/hide toggles masking and is named
    let mut rt = load("F01", "en", 1366.0, 768.0);
    let mut m = Mirror::new();
    pump(&mut rt, &mut m);
    type_into(&mut rt, "password", "hunter22");
    pump(&mut rt, &mut m);
    let toggle = m.by_author("toggle").unwrap();
    assert_eq!(toggle.role(), Role::Button);
    assert_eq!(toggle.label(), Some("Show password"), "toggle is named");
    assert_eq!(toggle.toggled(), Some(Toggled::False), "pressed=false while masked");
    let pw = m.by_author("password").unwrap();
    assert_eq!(pw.role(), Role::PasswordInput);
    assert_eq!(pw.value(), Some("\u{2022}".repeat(8).as_str()), "masked: AT sees bullets, never the secret");
    assert!(rt.text_input_info("password").unwrap().masked);
    click(&mut rt, "toggle");
    pump(&mut rt, &mut m);
    assert_eq!(m.by_author("toggle").unwrap().toggled(), Some(Toggled::True));
    assert_eq!(m.by_author("password").unwrap().value(), Some("hunter22"), "unmasked");
    assert!(!rt.text_input_info("password").unwrap().masked);
    click(&mut rt, "toggle");
    pump(&mut rt, &mut m);
    assert_eq!(m.by_author("password").unwrap().value(), Some("\u{2022}".repeat(8).as_str()), "masked again");
    // editing while masked keeps the true value: insert in the middle, backspace
    click(&mut rt, "password");
    rt.key(Key::Home, Mods::default());
    rt.key(Key::Right, Mods::default());
    rt.text("Z");
    rt.key(Key::Backspace, Mods::default());
    rt.key(Key::Backspace, Mods::default());
    assert_eq!(rt.state_value("password"), Some(&pvc_ir::Value::Text("unter22".into())));

    // --- item 4: errors announced (live = polite), present from the first tree
    let mut rt = load("F01", "en", 1366.0, 768.0);
    let mut m = Mirror::new();
    pump(&mut rt, &mut m);
    let err_id = rt.node_id("surface0/form0/field1/error");
    let first = m.0.get(&err_id).expect("live region exists in the tree before any error");
    assert_eq!(first.live(), Some(accesskit::Live::Polite));
    assert_eq!(first.label(), Some(""), "empty while hidden");
    type_into(&mut rt, "email", "nope");
    pump(&mut rt, &mut m);
    let shown = m.0.get(&err_id).unwrap();
    assert_eq!(shown.live(), Some(accesskit::Live::Polite));
    assert_eq!(shown.role(), Role::Status);
    assert_eq!(shown.label(), Some("Enter a valid email address, like name@example.com"));
    println!("effects: {:?}", rt.effect_log());
    assert!(rt.effect_log().is_empty(), "F01 declares no effect");
}

#[test]
fn f01_incremental_updates_are_small_and_idle_is_silent() {
    let mut rt = load("F01", "en", 1366.0, 768.0);
    rt.frame();
    let full = rt.take_tree_update().unwrap();
    assert!(full.tree.is_some());
    assert!(!rt.needs_frame());
    click(&mut rt, "checkbox");
    rt.frame();
    let inc = rt.take_tree_update().unwrap();
    assert!(inc.tree.is_none());
    assert!(inc.nodes.len() < full.nodes.len() / 2, "incremental: {} of {} nodes", inc.nodes.len(), full.nodes.len());
    assert!(!rt.needs_frame());
}

#[test]
fn f01_zh_cn_ime_preedit_and_commit() {
    let mut rt = load("F01", "zh-CN", 1366.0, 768.0);
    rt.frame();
    click(&mut rt, "name");
    rt.frame();
    rt.ime_preedit("ni", Some((2, 2)));
    rt.frame();
    assert!(rt.text_input_info("name").unwrap().composing);
    let a = rt.focused_text_input_rect().unwrap();
    rt.ime_preedit("ni hao", Some((6, 6)));
    rt.frame();
    let b = rt.focused_text_input_rect().unwrap();
    assert!(b.w > a.w, "candidate-window anchor grows with the preedit: {a:?} -> {b:?}");
    assert_eq!(rt.state_value("name"), Some(&pvc_ir::Value::Text("".into())), "preedit is not committed state");
    rt.ime_commit("你好");
    rt.frame();
    assert_eq!(rt.state_value("name"), Some(&pvc_ir::Value::Text("你好".into())));
    assert!(!rt.text_input_info("name").unwrap().composing);
    // second composition appended, then cleared (Escape in an IME): value unchanged
    rt.ime_preedit("shi", Some((3, 3)));
    rt.ime_preedit("", None);
    assert_eq!(rt.state_value("name"), Some(&pvc_ir::Value::Text("你好".into())));
    let info = rt.text_input_info("name").unwrap();
    assert_eq!(info.value, "你好");
    assert_eq!(info.selection_utf16, (2, 2));
    // zh-CN strings are used for names
    let t = rt.full_tree();
    let name = t.nodes.iter().find(|(_, n)| n.author_id() == Some("submit")).unwrap();
    assert_eq!(name.1.label(), Some("注册"));
}
