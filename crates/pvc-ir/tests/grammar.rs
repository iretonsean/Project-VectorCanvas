//! Statements made in GRAMMAR.md that are not covered by the specimens.

use pvc_ir::*;

const H: &str = "pvc 0.1 use std@0.1\nfont ui 400 sha256=ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280\n";
fn m(body: &str) -> Module {
    parse_terse(&format!("{}{}", H, body)).unwrap_or_else(|d| panic!("{:?}", d.iter().map(|x| x.render()).collect::<Vec<_>>()))
}
fn first_child(mm: &Module) -> &Node {
    &mm.root.children[0]
}

#[test]
fn lengths_tokens_and_rounding() {
    let a = m("box pad=lg gap=xs w=100.5 h=0.015625\n");
    let n = first_child(&a);
    assert_eq!(n.layout.pad, Edges::all(Len::from_px(24)));
    assert_eq!(n.layout.gap, Len::from_px(4));
    assert_eq!(n.layout.w, Size::Px(Len(64 * 100 + 32)));
    assert_eq!(n.layout.h, Size::Px(Len(1)));
    // 0.01 px rounds to the nearest 1/64: 0.64 -> 1
    assert_eq!(first_child(&m("box w=0.01\n")).layout.w, Size::Px(Len(1)));
    // pad lists: [vertical, horizontal] and [t r b l]
    let n = first_child(&m("box pad=[sm, lg]\n")).layout.pad;
    assert_eq!((n.top, n.right, n.bottom, n.left), (Len::from_px(8), Len::from_px(24), Len::from_px(8), Len::from_px(24)));
    let n = first_child(&m("box pad=[1, 2, 3, 4]\n")).layout.pad;
    assert_eq!((n.top, n.right, n.bottom, n.left), (Len::from_px(1), Len::from_px(2), Len::from_px(3), Len::from_px(4)));
}

#[test]
fn row_col_column_and_implicit_surface() {
    let a = m("row { }\ncolumn { }\ncol { }\n");
    assert_eq!(a.root.id, "surface0");
    assert_eq!(a.root.kind, NodeKind::Surface);
    assert_eq!(a.root.children.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["surface0/row0", "surface0/col0", "surface0/col1"]);
    assert_eq!(a.root.children[0].layout.dir, Dir::Row);
    // an explicit surface is the root and is forced to fill the viewport
    let b = m("surface pad=sm w=10 { text \"x\" }\n");
    assert_eq!(b.root.id, "surface0");
    assert_eq!(b.root.layout.w, Size::Fill);
    assert_eq!(b.root.layout.pad, Edges::all(Len::from_px(8)));
}

#[test]
fn text_styles_and_headings() {
    let a = m("text \"T\" bold size=20 heading=1\ntext \"m\" muted\ntext \"b\" body color=teal\n");
    let t = |i: usize| match &a.root.children[i].kind {
        NodeKind::Text(t) => t.clone(),
        _ => panic!(),
    };
    assert_eq!((t(0).weight, t(0).size), (700, Len::from_px(20)));
    assert_eq!(t(0).line_height, Len(64 * 28));
    assert_eq!(a.root.children[0].sem.role, Role::Heading);
    assert_eq!(a.root.children[0].sem.heading_level, Some(1));
    assert_eq!(t(1).color, Color::Token("muted".into()));
    assert_eq!(t(2).color, Color::Token("teal".into()), "explicit colour wins over `body`");
}

#[test]
fn versions_and_theme_are_checked() {
    assert!(compile_terse("pvc 0.2 use std@0.1\ntext \"x\"\n").is_err());
    assert!(compile_terse("pvc 0.1 use std@0.2\ntext \"x\"\n").is_err());
    let d = compile_terse("pvc 0.1 use std@0.1 theme dak\ntext \"x\"\n").unwrap_err();
    assert_eq!(d[0].code, "PVC-E-UNKNOWN-TOKEN");
}

#[test]
fn text_input_core_kind_and_change_handler() {
    let a = m("state { v: text = \"hi\"; n: int }\ntext-input bind=v variant=email name=@l on.change={ n = 1 }\nstrings en { l \"Label\" }\n");
    let n = first_child(&a);
    assert_eq!(n.sem.role, Role::EmailField);
    assert_eq!(n.on[0].event, Event::Change);
    assert_eq!(a.state[0].init, Value::Text("hi".into()));
}

#[test]
fn submit_button_outside_form_is_schema_error() {
    let d = parse_terse(&format!("{}button \"x\" submit\n", H)).unwrap_err();
    assert_eq!(d[0].message_key, "schema.submit-outside-form");
}

#[test]
fn net_capability_needs_hosts_and_methods() {
    let d = parse_terse(&format!("{}cap net.main\ntext \"x\"\n", H)).unwrap_err();
    assert_eq!(d[0].code, "PVC-E-SCHEMA");
    let d = parse_terse(&format!("{}cap lasers.main\ntext \"x\"\n", H)).unwrap_err();
    assert_eq!(d[0].message_key, "schema.unknown-capability-kind");
}

#[test]
fn lexer_rules() {
    // `a-b` is one word, so this is an unknown state name, not a subtraction
    let d = compile_terse(&format!("{}state {{ a: int; b: int }}\ntext \"x\" show=(a-b == 0)\n", H)).unwrap_err();
    assert!(d.iter().any(|x| x.code == "PVC-E-REF"));
    // string escapes and unicode escapes
    let a = m("text \"q\\\"\\u{e9}\\n\"\n");
    assert_eq!(a.strings.tables[0].entries[0].1, "q\"\u{e9}\n");
    // backslash continuation
    let a = m("box pad=sm \\\n  gap=md\n");
    assert_eq!(first_child(&a).layout.gap, Len::from_px(16));
}
