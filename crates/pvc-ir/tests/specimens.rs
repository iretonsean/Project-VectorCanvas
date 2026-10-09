//! G2 checklists for L01 and F01, checked structurally / by reference evaluation on the *loaded signed
//! artifact* (not the terse source), plus U1 (names), U3 (contrast of the colours the specimens use).
//! Layout checklist items are statements about the explicit layout fields; the solved-geometry
//! versions of these checks belong to the runtime (dev-render).

use pvc_ir::eval::*;
use pvc_ir::sign::testkey;
use pvc_ir::theme::contrast;
use pvc_ir::*;

fn load(name: &str) -> Module {
    let b = std::fs::read(format!("{}/../specimens/{}.pvca", env!("CARGO_MANIFEST_DIR"), name)).unwrap();
    load_signed(&b, &testkey::verifying_key()).unwrap()
}
fn node<'a>(m: &'a Module, id: &str) -> &'a Node {
    m.find_node(id).unwrap_or_else(|| panic!("no node {id}"))
}
fn text_of<'a>(m: &'a Module, n: &Node, loc: &str) -> &'a str {
    match &n.kind {
        NodeKind::Text(t) => m.string(loc, &t.key).unwrap(),
        _ => panic!("not text"),
    }
}
fn rgb(m: &Module, c: &Color) -> [u8; 4] {
    m.resolve_color(c).unwrap()
}

// ---------------------------------------------------------------- L01
#[test]
fn l01_checklist() {
    let m = load("L01");
    let card = node(&m, "surface0/card0");
    // button spans card width: Fill in a column whose cross-axis alignment is start/stretch-by-fill
    let button = node(&m, "surface0/card0/button0");
    assert_eq!(card.layout.dir, Dir::Col);
    assert_eq!(button.layout.w, Size::Fill);
    assert_eq!(card.layout.w, Size::Px(Len::from_px(320)));
    // exact strings present (A)
    let en = "en";
    assert_eq!(text_of(&m, node(&m, "surface0/card0/row0/col0/text0"), en), "Jane Doe");
    assert_eq!(text_of(&m, node(&m, "surface0/card0/row0/col0/text1"), en), "Product Designer");
    assert_eq!(
        text_of(&m, node(&m, "surface0/card0/text0"), en),
        "Building the future of spatial computing via GPU pipelines."
    );
    assert_eq!(text_of(&m, node(&m, "surface0/card0/button0/label"), en), "Connect");
    assert_eq!(text_of(&m, node(&m, "surface0/card0/row0/avatar0/label"), en), "JD");
    // name bold and larger than title (A)
    let (name, title) = match (&node(&m, "surface0/card0/row0/col0/text0").kind, &node(&m, "surface0/card0/row0/col0/text1").kind) {
        (NodeKind::Text(a), NodeKind::Text(b)) => (a, b),
        _ => panic!(),
    };
    assert!(name.weight >= 700 && title.weight < 700);
    assert!(name.size > title.size);
    assert_eq!(name.size, Len::from_px(14));
    assert_eq!(title.size, Len::from_px(12));
    // avatar circular, left of name (A): circle shape, square, first child of a row that precedes the name column
    let row = node(&m, "surface0/card0/row0");
    assert_eq!(row.layout.dir, Dir::Row);
    assert_eq!(row.children[0].id, "surface0/card0/row0/avatar0");
    assert_eq!(row.children[1].id, "surface0/card0/row0/col0");
    let av = &row.children[0];
    assert_eq!(av.paint.shape, Shape::Circle);
    assert_eq!(av.layout.w, av.layout.h);
    // the button triggers connect_user through the declared net capability, nothing else does
    assert_eq!(button.on[0].actions, vec![Action::Emit("connect_user".into())]);
    assert_eq!(m.effects, vec![Effect { name: "connect_user".into(), cap: "net.main".into() }]);
    assert_eq!(m.manifest.caps.len(), 1);
    assert_eq!(m.manifest.caps[0].kind, "net");
    assert_eq!(m.manifest.caps[0].hosts, vec!["connect.example.invalid".to_string()]);
}

// ---------------------------------------------------------------- F01
fn setv(st: &mut State, k: &str, v: Value) {
    assert!(st.set(k, v));
}
fn t(s: &str) -> Value {
    Value::Text(s.into())
}

fn submit_disabled(m: &Module, st: &State) -> bool {
    let b = node(m, "surface0/form0/button0");
    eval_bool(m, st, b.sem.disabled.as_ref().unwrap()).unwrap()
}

#[test]
fn f01_submit_disabled_until_all_valid() {
    let m = load("F01");
    let mut st = State::init(&m);
    assert!(submit_disabled(&m, &st), "initially invalid");
    setv(&mut st, "name", t("Ada Lovelace"));
    assert!(submit_disabled(&m, &st));
    setv(&mut st, "email", t("ada@example"));
    assert!(submit_disabled(&m, &st));
    setv(&mut st, "email", t("ada@example.com"));
    assert!(submit_disabled(&m, &st), "no password yet");
    setv(&mut st, "password", t("1234567"));
    assert!(submit_disabled(&m, &st), "7 chars");
    setv(&mut st, "password", t("12345678"));
    assert!(submit_disabled(&m, &st), "terms not accepted");
    // press the checkbox: its handler toggles `terms`
    let cb = node(&m, "surface0/form0/checkbox0");
    assert_eq!(eval_bool(&m, &st, cb.sem.checked.as_ref().unwrap()), Ok(false));
    run_handler(&m, &mut st, &cb.on[0]).unwrap();
    assert_eq!(eval_bool(&m, &st, cb.sem.checked.as_ref().unwrap()), Ok(true));
    assert!(!submit_disabled(&m, &st), "all valid -> enabled");
    // each validity condition individually breaks it again
    for (k, bad) in [("name", t("")), ("email", t("nope")), ("password", t("short"))] {
        let mut s2 = st.clone();
        setv(&mut s2, k, bad);
        assert!(submit_disabled(&m, &s2), "{k}");
    }
    let mut s2 = st.clone();
    run_handler(&m, &mut s2, &cb.on[0]).unwrap();
    assert!(submit_disabled(&m, &s2), "unchecked again");
}

#[test]
fn f01_submit_handler_is_guarded_and_sets_done() {
    let m = load("F01");
    let form = node(&m, "surface0/form0");
    let sub = &form.on[0];
    assert_eq!(sub.event, Event::Submit);
    // Enter in a field with invalid state does nothing
    let mut st = State::init(&m);
    let o = run_handler(&m, &mut st, sub).unwrap();
    assert!(!o.ran);
    assert_eq!(st.get("status"), Some(&Value::Enum("editing".into())));
    // valid -> runs
    setv(&mut st, "name", t("Ada"));
    setv(&mut st, "email", t("a@b.co"));
    setv(&mut st, "password", t("pass1234"));
    setv(&mut st, "terms", Value::Bool(true));
    let o = run_handler(&m, &mut st, sub).unwrap();
    assert!(o.ran);
    assert_eq!(st.get("status"), Some(&Value::Enum("done".into())));
    // the submit button's press raises submit on the form
    let btn = node(&m, "surface0/form0/button0");
    assert_eq!(btn.on[0].actions, vec![Action::Submit("surface0/form0".into())]);
    // F01 has no effects and no capabilities: nothing can leave the device
    assert!(m.effects.is_empty() && m.manifest.caps.is_empty());
}

#[test]
fn f01_email_validated_and_errors_announced() {
    let m = load("F01");
    let input = node(&m, "surface0/form0/field1/input");
    let err = node(&m, "surface0/form0/field1/error");
    // semantic wiring: invalid state + error-message relation + polite live region
    assert_eq!(input.sem.role, Role::EmailField);
    assert_eq!(input.sem.error_message.as_deref(), Some(err.id.as_str()));
    assert_eq!(err.sem.live, Live::Polite);
    assert_eq!(err.sem.role, Role::Status);
    assert_eq!(err.show, input.sem.invalid, "error is shown exactly while the input is invalid");
    let mut st = State::init(&m);
    let shown = |st: &State| eval_bool(&m, st, err.show.as_ref().unwrap()).unwrap();
    assert!(!shown(&st), "no error on an untouched field");
    for (v, show) in [("a", true), ("a@b", true), ("a@b.c", true), ("a@b.co", false), ("", false), ("x y@b.co", true)] {
        setv(&mut st, "email", t(v));
        assert_eq!(shown(&st), show, "email={v:?}");
    }
    // password error likewise, and masking
    let pe = node(&m, "surface0/form0/password-field0/error");
    assert_eq!(pe.sem.live, Live::Polite);
    setv(&mut st, "password", t("abc"));
    assert!(eval_bool(&m, &st, pe.show.as_ref().unwrap()).unwrap());
    setv(&mut st, "password", t("abcdefgh"));
    assert!(!eval_bool(&m, &st, pe.show.as_ref().unwrap()).unwrap());
}

#[test]
fn f01_show_hide_toggles_masking_and_is_named() {
    let m = load("F01");
    let mut ids = Vec::new();
    m.walk(&mut |n, _| ids.push(n.id.clone()));
    let input_id = ids.iter().find(|i| i.ends_with("/input") && i.contains("password-field")).unwrap().clone();
    let toggle_id = ids.iter().find(|i| i.ends_with("/toggle")).unwrap().clone();
    let input = node(&m, &input_id);
    let toggle = node(&m, &toggle_id);
    let masked = match &input.kind {
        NodeKind::TextInput(t) => {
            assert_eq!(t.variant, InputVariant::Password);
            t.masked.clone()
        }
        _ => panic!(),
    };
    let mut st = State::init(&m);
    assert_eq!(eval_bool(&m, &st, &masked), Ok(true), "masked by default");
    assert_eq!(eval_bool(&m, &st, toggle.sem.pressed.as_ref().unwrap()), Ok(false));
    run_handler(&m, &mut st, &toggle.on[0]).unwrap();
    assert_eq!(eval_bool(&m, &st, &masked), Ok(false), "unmasked after toggle");
    assert_eq!(eval_bool(&m, &st, toggle.sem.pressed.as_ref().unwrap()), Ok(true));
    run_handler(&m, &mut st, &toggle.on[0]).unwrap();
    assert_eq!(eval_bool(&m, &st, &masked), Ok(true), "masked again");
    // named, in both locales, and it is a button
    assert_eq!(toggle.sem.role, Role::Button);
    let k = match &toggle.sem.name {
        Some(Name::Key(k)) => k.clone(),
        o => panic!("{o:?}"),
    };
    assert_eq!(m.string("en", &k), Some("Show password"));
    assert_eq!(m.string("zh-CN", &k), Some("显示密码"));
}

#[test]
fn f01_u1_every_interactive_element_is_named_in_every_locale() {
    for name in ["L01", "F01"] {
        let m = load(name);
        let mut checked = 0;
        m.walk(&mut |n, _| {
            if n.is_interactive() && n.sem.role.needs_name() {
                let key = match n.sem.name.as_ref().expect("interactive node without name") {
                    Name::Key(k) => k.clone(),
                    Name::FromNode(id) => match &node(&m, id).kind {
                        NodeKind::Text(t) => t.key.clone(),
                        _ => panic!(),
                    },
                };
                for l in &m.manifest.locales {
                    assert!(!m.string(l, &key).expect("missing string").is_empty(), "{name} {} {l}", n.id);
                }
                checked += 1;
            }
        });
        assert!(checked >= 1, "{name}: no interactive nodes?");
    }
}

#[test]
fn f01_every_string_key_exists_in_both_locales_and_zh_is_actually_chinese() {
    let m = load("F01");
    assert_eq!(m.manifest.locales, vec!["en".to_string(), "zh-CN".to_string()]);
    let en = &m.strings.tables[0];
    let zh = &m.strings.tables[1];
    assert_eq!(en.entries.len(), zh.entries.len());
    for ((ke, ve), (kz, vz)) in en.entries.iter().zip(zh.entries.iter()) {
        assert_eq!(ke, kz);
        assert!(!ve.is_empty() && !vz.is_empty());
        assert!(vz.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)), "{kz}: {vz}");
    }
}

// ---------------------------------------------------------------- U3 contrast on the colours the specimens use
#[test]
fn u3_text_contrast_meets_aa_for_specimen_colour_pairs() {
    for name in ["L01", "F01"] {
        let m = load(name);
        // effective background of a node = nearest ancestor-or-self fill
        fn walk(m: &Module, n: &Node, bg: [u8; 4], out: &mut Vec<(String, f64, bool)>) {
            let bg = n.paint.fill.as_ref().map(|c| m.resolve_color(c).unwrap()).unwrap_or(bg);
            if let NodeKind::Text(t) = &n.kind {
                let fg = m.resolve_color(&t.color).unwrap();
                let large = t.size.px() >= 24.0 || (t.size.px() >= 18.66 && t.weight >= 700);
                out.push((n.id.clone(), contrast(fg, bg), large));
            }
            for c in &n.children {
                walk(m, c, bg, out);
            }
        }
        let mut rows = Vec::new();
        walk(&m, &m.root, [0, 0, 0, 255], &mut rows);
        assert!(!rows.is_empty());
        for (id, ratio, large) in rows {
            let need = if large { 3.0 } else { 4.5 };
            assert!(ratio >= need, "{name} {id}: contrast {ratio:.2} < {need}");
        }
        // non-text UI component contrast (borders of inputs vs their surroundings) >= 3:1 in F01
        if name == "F01" {
            let bg = rgb(&m, &Color::Token("bg".into()));
            let border = rgb(&m, &Color::Token("border".into()));
            assert!(contrast(border, bg) >= 3.0);
        }
    }
}

#[test]
fn p5_every_node_with_a_role_that_needs_a_name_has_one_after_expansion() {
    for name in ["L01", "F01"] {
        let m = load(name);
        assert!(validate(&m).iter().all(|d| !d.is_error()), "{name}");
    }
}
