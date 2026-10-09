//! The 15 hand-written pilot reference programs (pilot-refs/*.pvc): they are the proof that the
//! language can express L01–L10 and G01–G05. Reference only: never shown to the agent.
//! Behaviour is exercised with the reference evaluator (`pvc_ir::eval`), the G2 checklist A-items are
//! restated at IR/state level (geometry-level checks belong to the runtime probe).

use pvc_ir::eval::*;
use pvc_ir::*;

const IDS: &[&str] = &["L01", "L02", "L03", "L04", "L05", "L06", "L07", "L08", "L09", "L10", "G01", "G02", "G03", "G04", "G05"];

fn path(id: &str) -> String {
    format!("{}/../pilot-refs/{}.pvc", env!("CARGO_MANIFEST_DIR"), id)
}
fn src(id: &str) -> String {
    std::fs::read_to_string(path(id)).unwrap()
}
fn load(id: &str) -> Module {
    match compile_terse(&src(id)) {
        Ok(c) => c.module,
        Err(d) => panic!("{id}: {:?}", d.iter().map(|x| x.render()).collect::<Vec<_>>()),
    }
}
fn all(m: &Module) -> Vec<&Node> {
    let mut v = Vec::new();
    m.walk(&mut |n, _| v.push(n));
    v
}
fn name_of(m: &Module, n: &Node) -> String {
    match &n.sem.name {
        Some(Name::Key(k)) => m.string("en", k).unwrap_or("").to_string(),
        Some(Name::FromNode(id)) => match &m.find_node(id).unwrap().kind {
            NodeKind::Text(t) => m.string("en", &t.key).unwrap().to_string(),
            _ => String::new(),
        },
        None => match &n.kind {
            NodeKind::Text(t) => m.string("en", &t.key).unwrap_or("").to_string(),
            _ => String::new(),
        },
    }
}
fn with_role<'a>(m: &'a Module, role: Role) -> Vec<&'a Node> {
    all(m).into_iter().filter(|n| n.sem.role == role && !n.sem.decorative).collect()
}
fn named<'a>(m: &'a Module, role: Role, name: &str) -> Vec<&'a Node> {
    with_role(m, role).into_iter().filter(|n| name_of(m, n) == name).collect()
}
fn one<'a>(m: &'a Module, role: Role, name: &str) -> &'a Node {
    let v = named(m, role, name);
    assert_eq!(v.len(), 1, "{role:?} {name:?}: {} matches", v.len());
    v[0]
}
fn press(m: &Module, st: &mut State, n: &Node, sc: &Scope) {
    for h in n.on.iter().filter(|h| h.event == Event::Press) {
        run_handler_in(m, st, sc, h).unwrap();
    }
}
fn shown(m: &Module, st: &State, n: &Node, sc: &Scope) -> bool {
    n.show.as_ref().map(|e| eval_bool_in(m, st, sc, e).unwrap()).unwrap_or(true)
}
fn en(m: &Module, st: &State, sc: &Scope, n: &Node) -> String {
    match &n.kind {
        NodeKind::Text(t) => text_content(m, st, sc, "en", t).unwrap(),
        _ => panic!(),
    }
}
fn rows_shown(m: &Module, st: &State, v: &Node) -> Vec<usize> {
    let vp = match &v.kind {
        NodeKind::View(p) => p,
        _ => panic!(),
    };
    let n = st.rows(&vp.list).unwrap().len();
    (0..n)
        .filter(|i| match &vp.filter {
            None => true,
            Some(f) => eval_bool_in(m, st, &Scope::with(&vp.alias, &vp.list, *i), f).unwrap(),
        })
        .collect()
}
fn view(m: &Module) -> &Node {
    all(m).into_iter().find(|n| matches!(n.kind, NodeKind::View(_))).unwrap()
}
fn descendants<'a>(n: &'a Node, out: &mut Vec<&'a Node>) {
    for c in &n.children {
        out.push(c);
        descendants(c, out);
    }
}
fn in_view<'a>(m: &'a Module, role: Role, name: &str) -> &'a Node {
    let mut d = Vec::new();
    descendants(view(m), &mut d);
    let v: Vec<&Node> = d.into_iter().filter(|n| n.sem.role == role && name_of(m, n) == name).collect();
    assert_eq!(v.len(), 1, "{role:?} {name:?} in view");
    v[0]
}
fn vp(m: &Module) -> (String, String) {
    match &view(m).kind {
        NodeKind::View(p) => (p.alias.clone(), p.list.clone()),
        _ => unreachable!(),
    }
}
fn set(st: &mut State, k: &str, v: Value) {
    assert!(st.set(k, v));
}

// ---------------------------------------------------------------- all 15
#[test]
fn all_fifteen_refs_validate_clean_and_are_deterministic() {
    for id in IDS {
        let a = compile_terse(&src(id)).unwrap_or_else(|d| panic!("{id}: {:?}", d.iter().map(|x| x.render()).collect::<Vec<_>>()));
        assert!(a.diagnostics.is_empty(), "{id}: {:?}", a.diagnostics.iter().map(|x| x.render()).collect::<Vec<_>>());
        let b = compile_terse(&src(id)).unwrap();
        let (ca, cb) = (canonicalize(&a.module), canonicalize(&b.module));
        assert_eq!(ca, cb, "{id}: D1");
        // exact canonical round trip, incl. the new constructs
        let back = from_canonical(&ca).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(canonicalize(&back), ca, "{id}: round trip");
        // sign + load
        let art = sign(&a.module, &sign::testkey::signing_key());
        assert!(load_signed(&art, &sign::testkey::verifying_key()).is_ok(), "{id}");
    }
}

// ---------------------------------------------------------------- layout-heavy: IR-level checklist statements
fn grid_nodes(m: &Module) -> Vec<&Node> {
    all(m).into_iter().filter(|n| n.layout.dir == Dir::Grid).collect()
}
fn md_cols(n: &Node) -> Option<u16> {
    n.at.iter().find(|a| a.bp == Bp::Md).and_then(|a| a.patch.cols)
}

#[test]
fn l02_three_columns_one_column_on_phones_four_features_each() {
    let m = load("L02");
    let g = grid_nodes(&m);
    assert_eq!(g.len(), 1);
    assert_eq!((g[0].layout.cols, md_cols(g[0])), (3, Some(1)));
    assert_eq!(g[0].children.len(), 3);
    for plan in &g[0].children {
        let mut d = Vec::new();
        descendants(plan, &mut d);
        assert_eq!(d.iter().filter(|n| n.sem.role == Role::ListItem).count(), 4);
        assert_eq!(d.iter().filter(|n| n.sem.role == Role::Button).count(), 1);
    }
}
#[test]
fn l03_row_stacks_on_md_and_headline_is_the_single_h1() {
    let m = load("L03");
    let rows: Vec<&Node> = all(&m).into_iter().filter(|n| n.at.iter().any(|a| a.patch.dir == Some(Dir::Col))).collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].layout.dir, Dir::Row);
    let h1: Vec<&Node> = with_role(&m, Role::Heading).into_iter().filter(|n| n.sem.heading_level == Some(1)).collect();
    assert_eq!(h1.len(), 1);
    assert_eq!(with_role(&m, Role::Button).len(), 2);
    let ill = all(&m).into_iter().find(|n| n.kind == NodeKind::Shape).unwrap();
    assert!(matches!(ill.layout.w, Size::Px(l) if l.px() >= 160.0));
}
#[test]
fn l04_five_landmarks_as_regions_and_stacking_at_md() {
    let m = load("L04");
    let names: Vec<String> = with_role(&m, Role::Region).iter().map(|n| name_of(&m, n)).collect();
    assert_eq!(names, ["Header", "Related links", "Footer"]);
    assert_eq!(with_role(&m, Role::Navigation).len(), 1);
    assert_eq!(with_role(&m, Role::Main).len(), 1);
    assert!(with_role(&m, Role::Link).len() >= 5);
    let row = all(&m).into_iter().find(|n| n.layout.dir == Dir::Row && n.at.iter().any(|a| a.patch.dir == Some(Dir::Col))).unwrap();
    assert_eq!(row.children.len(), 3);
    assert_eq!(row.layout.h, Size::Fill, "middle band grows: footer sits at the bottom of a short page");
}
#[test]
fn l05_article_structure() {
    let m = load("L05");
    let h = with_role(&m, Role::Heading);
    assert_eq!(h.iter().filter(|n| n.sem.heading_level == Some(1)).count(), 1);
    assert_eq!(with_role(&m, Role::Paragraph).len(), 6);
    let col = all(&m).into_iter().find(|n| n.layout.w == Size::Px(Len::from_px(640))).unwrap();
    assert_eq!(col.children.len(), 9);
}
#[test]
fn l06_rows_label_left_control_right_and_group_headings() {
    let m = load("L06");
    let hs: Vec<_> = with_role(&m, Role::Heading).into_iter().filter(|n| n.sem.heading_level == Some(2)).collect();
    assert_eq!(hs.len(), 3);
    let controls: Vec<&Node> = all(&m).into_iter().filter(|n| matches!(n.sem.role, Role::Switch | Role::TextField)).collect();
    assert_eq!(controls.len(), 6);
    for c in controls {
        // each control's name equals the text of the label sibling in its row
        let row = all(&m).into_iter().find(|r| r.children.iter().any(|x| x.id == c.id)).unwrap();
        assert_eq!(row.layout.dir, Dir::Row);
        assert_eq!(row.children.len(), 2);
        assert_eq!(row.children[1].id, c.id, "control is the right-hand child");
        assert_eq!(name_of(&m, &row.children[0]), name_of(&m, c));
        assert_eq!(row.children[0].layout.w, Size::Fill, "label takes the slack so controls end at the right edge");
    }
}
#[test]
fn l07_tiles_panels_search_avatar() {
    let m = load("L07");
    let g = grid_nodes(&m);
    assert_eq!(g.len(), 2);
    assert_eq!((g[0].layout.cols, md_cols(g[0]), g[0].children.len()), (4, Some(2), 4));
    assert_eq!((g[1].layout.cols, md_cols(g[1]), g[1].children.len()), (2, Some(1), 2));
    assert_eq!(g[1].children[0].layout.h, g[1].children[1].layout.h);
    let s = with_role(&m, Role::SearchField);
    assert_eq!(s.len(), 1);
    assert!(name_of(&m, s[0]).contains("Search"));
    assert_eq!(with_role(&m, Role::Image).len(), 1, "avatar named by alt");
    assert_eq!(name_of(&m, with_role(&m, Role::Image)[0]), "Account menu");
}
#[test]
fn l08_date_block_left_icon_decorative_link_role() {
    let m = load("L08");
    let row = all(&m).into_iter().find(|n| n.layout.dir == Dir::Row && n.children.len() == 2 && n.children[0].layout.w == Size::Px(Len::from_px(64))).unwrap();
    assert_eq!(row.children[0].children.len(), 2, "month and day as separate lines");
    assert!(all(&m).iter().any(|n| n.kind == NodeKind::Shape && n.sem.decorative));
    assert_eq!(with_role(&m, Role::Link).len(), 1);
    assert_eq!(name_of(&m, with_role(&m, Role::Link)[0]), "Add to calendar");
    assert_eq!(with_role(&m, Role::Heading).len(), 1);
}
#[test]
fn l09_four_columns_collapse_icons_named_email_labelled() {
    let m = load("L09");
    let g = grid_nodes(&m);
    assert_eq!((g[0].layout.cols, md_cols(g[0]), g[0].children.len()), (4, Some(1), 4));
    let titles: Vec<String> = g[0].children.iter().map(|c| name_of(&m, &c.children[0])).collect();
    assert_eq!(titles, ["Product", "Company", "Resources", "Legal"]);
    for c in &g[0].children {
        assert_eq!(c.children.iter().filter(|x| x.sem.role == Role::Link).count(), 3);
    }
    let icon_links: Vec<&Node> = with_role(&m, Role::Link).into_iter().filter(|l| l.children.iter().all(|c| c.kind == NodeKind::Shape)).collect();
    assert_eq!(icon_links.len(), 2);
    for l in icon_links {
        assert!(!name_of(&m, l).is_empty());
        assert!(l.children[0].sem.decorative);
    }
    let e = with_role(&m, Role::TextField).into_iter().chain(with_role(&m, Role::EmailField)).collect::<Vec<_>>();
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].sem.role, Role::EmailField);
    assert_eq!(name_of(&m, e[0]), "Email");
    assert!(e[0].sem.labelled_by.is_some());
}
#[test]
fn l10_grid_3x2_to_one_column_equal_gaps_icons_decorative() {
    let m = load("L10");
    let g = &grid_nodes(&m)[0].clone();
    assert_eq!((g.layout.cols, md_cols(g), g.children.len()), (3, Some(1), 6));
    let h2 = with_role(&m, Role::Heading).into_iter().filter(|n| n.sem.heading_level == Some(2)).count();
    assert_eq!(h2, 6);
    for cell in &g.children {
        let icons: Vec<&Node> = cell.children.iter().filter(|c| c.kind == NodeKind::Shape).collect();
        assert_eq!(icons.len(), 1);
        assert!(icons[0].sem.decorative);
    }
    assert_eq!(g.layout.gap, Len::from_px(24), "one gap value for rows and columns");
}

// ---------------------------------------------------------------- logic-heavy: behaviour
#[test]
fn g01_hint_dismissal_and_new_session() {
    let m = load("G01");
    let hint = one(&m, Role::Button, "Dismiss");
    let hint_box = all(&m).into_iter().find(|n| n.children.iter().any(|c| c.id == hint.id)).unwrap();
    let add = one(&m, Role::Button, "Add note");
    let none = Scope::default();
    let mut st = State::init(&m);
    // G01.1
    assert!(shown(&m, &st, hint_box, &none), "fresh: hint shown");
    press(&m, &mut st, add, &none);
    assert!(!shown(&m, &st, hint_box, &none));
    let rm = in_view(&m, Role::Button, "Remove");
    let (alias, list) = vp(&m);
    press(&m, &mut st, rm, &Scope::with(&alias, &list, 0));
    assert!(st.rows("notes").unwrap().is_empty());
    assert!(shown(&m, &st, hint_box, &none), "empty again, not dismissed: hint returns");
    // G01.2
    press(&m, &mut st, hint, &none);
    assert!(!shown(&m, &st, hint_box, &none));
    press(&m, &mut st, add, &none);
    press(&m, &mut st, add, &none);
    assert_eq!(rows_shown(&m, &st, view(&m)).len(), 2);
    press(&m, &mut st, rm, &Scope::with(&alias, &list, 1));
    press(&m, &mut st, rm, &Scope::with(&alias, &list, 0));
    assert!(st.rows("notes").unwrap().is_empty());
    assert!(!shown(&m, &st, hint_box, &none), "dismissed: never again in the session");
    press(&m, &mut st, add, &none);
    assert!(!shown(&m, &st, hint_box, &none));
    // G01.3: a new session is a fresh State (the module declares no storage capability)
    let st2 = State::init(&m);
    assert!(shown(&m, &st2, hint_box, &none));
    assert!(m.manifest.caps.is_empty() && m.effects.is_empty(), "dismissal is not persisted");
    // notes read "Note 1", "Note 2": the counter is in the data
    let mut st3 = State::init(&m);
    press(&m, &mut st3, add, &none);
    press(&m, &mut st3, add, &none);
    let title = all(&m).into_iter().find(|n| matches!(&n.kind, NodeKind::Text(t) if t.key == "note.title")).unwrap();
    assert_eq!(en(&m, &st3, &Scope::with(&alias, &list, 1), title), "Note 2");
}

#[test]
fn g02_twenty_operations_match_a_reference_model() {
    let m = load("G02");
    let none = Scope::default();
    let mut st = State::init(&m);
    let add = one(&m, Role::Button, "Add");
    let del = in_view(&m, Role::Button, "Delete");
    let chk = {
        let mut d = Vec::new();
        descendants(view(&m), &mut d);
        d.into_iter().find(|n| n.sem.role == Role::Checkbox).unwrap()
    };
    let (alias, list) = vp(&m);
    let titles_t = {
        let mut d = Vec::new();
        descendants(view(&m), &mut d);
        d.into_iter().find(|n| matches!(&n.kind, NodeKind::Text(t) if t.value.is_some())).unwrap()
    };
    let left_many = all(&m).into_iter().find(|n| matches!(&n.kind, NodeKind::Text(t) if t.key == "left.many")).unwrap();
    let left_one = all(&m).into_iter().find(|n| matches!(&n.kind, NodeKind::Text(t) if m.string("en", &t.key) == Some("1 item left"))).unwrap();
    let f = |name: &str| one(&m, Role::Button, name);

    // reference model
    #[derive(Clone)]
    struct T {
        title: String,
        done: bool,
    }
    let mut model: Vec<T> = Vec::new();
    let mut filter = "all";
    #[derive(Clone)]
    enum Op {
        Add(&'static str),
        Toggle(usize),
        Delete(usize),
        Filter(&'static str),
    }
    use Op::*;
    let script = vec![
        Add("milk"), Add("eggs"), Add("bread"), Toggle(1), Filter("active"), Add("tea"), Filter("done"), Toggle(0),
        Filter("all"), Delete(1), Toggle(2), Filter("active"), Delete(0), Filter("done"), Add("jam"), Filter("all"),
        Toggle(0), Delete(0), Add("rice"), Delete(1),
    ];
    assert_eq!(script.len(), 20);
    for (step, op) in script.into_iter().enumerate() {
        match op {
            Add(t) => {
                set(&mut st, "draft", Value::Text(t.into()));
                press(&m, &mut st, add, &none);
                model.push(T { title: t.into(), done: false });
            }
            Toggle(i) => {
                press(&m, &mut st, chk, &Scope::with(&alias, &list, i));
                model[i].done = !model[i].done;
            }
            Delete(i) => {
                press(&m, &mut st, del, &Scope::with(&alias, &list, i));
                model.remove(i);
            }
            Filter(n) => {
                press(&m, &mut st, f(match n { "all" => "All", "active" => "Active", _ => "Done" }), &none);
                filter = n;
            }
        }
        // (i) visible task set for the filter, (ii) the count is of ALL not-done tasks
        let visible: Vec<String> =
            rows_shown(&m, &st, view(&m)).into_iter().map(|i| en(&m, &st, &Scope::with(&alias, &list, i), titles_t)).collect();
        let expect: Vec<String> = model
            .iter()
            .filter(|t| match filter { "all" => true, "active" => !t.done, _ => t.done })
            .map(|t| t.title.clone())
            .collect();
        assert_eq!(visible, expect, "step {step}: visible tasks");
        let left = model.iter().filter(|t| !t.done).count();
        let text = if shown(&m, &st, left_one, &none) { en(&m, &st, &none, left_one) } else { assert!(shown(&m, &st, left_many, &none)); en(&m, &st, &none, left_many) };
        let want = if left == 1 { "1 item left".to_string() } else { format!("{left} items left") };
        assert_eq!(text, want, "step {step}: count");
        assert!(!(shown(&m, &st, left_one, &none) && shown(&m, &st, left_many, &none)), "exactly one count line");
    }
    // empty drafts are not added
    let n0 = st.rows("tasks").unwrap().len();
    set(&mut st, "draft", Value::Text("".into()));
    press(&m, &mut st, add, &none);
    assert_eq!(st.rows("tasks").unwrap().len(), n0);
}

#[test]
fn g03_prompt_only_when_dirty_cancel_keeps_discard_drops() {
    let m = load("G03");
    let none = Scope::default();
    let tab_p = one(&m, Role::Tab, "Profile");
    let tab_n = one(&m, Role::Tab, "Notes");
    let discard = one(&m, Role::Button, "Discard");
    let cancel = one(&m, Role::Button, "Cancel");
    let dialog = one(&m, Role::AlertDialog, "Discard changes?");
    let areas: Vec<&Node> = with_role(&m, Role::TextArea);
    assert_eq!(areas.len(), 2);
    let area_p = areas.iter().find(|a| name_of(&m, a) == "Profile text").unwrap();
    let selected = |st: &State, t: &Node| eval_bool(&m, st, t.sem.selected.as_ref().unwrap()).unwrap();
    let prompt = |st: &State| shown(&m, st, dialog, &none);
    let mut st = State::init(&m);
    // G03.1 clean switches never prompt
    assert!(selected(&st, tab_p) && !selected(&st, tab_n));
    press(&m, &mut st, tab_n, &none);
    assert!(!prompt(&st) && selected(&st, tab_n) && !selected(&st, tab_p));
    press(&m, &mut st, tab_p, &none);
    assert!(!prompt(&st) && selected(&st, tab_p));
    // typing in the current tab's field makes it dirty (runtime writes bind, then runs Change)
    set(&mut st, "profile_text", Value::Text("x".into()));
    for h in area_p.on.iter().filter(|h| h.event == Event::Change) {
        run_handler(&m, &mut st, h).unwrap();
    }
    press(&m, &mut st, tab_n, &none);
    assert!(prompt(&st), "dirty: prompt");
    assert!(selected(&st, tab_p), "still on the first tab while the prompt is open");
    // G03.2 cancel keeps tab and text
    press(&m, &mut st, cancel, &none);
    assert!(!prompt(&st) && selected(&st, tab_p));
    assert_eq!(st.get("profile_text"), Some(&Value::Text("x".into())));
    // discard switches and drops the text
    press(&m, &mut st, tab_n, &none);
    assert!(prompt(&st));
    press(&m, &mut st, discard, &none);
    assert!(!prompt(&st) && selected(&st, tab_n));
    assert_eq!(st.get("profile_text"), Some(&Value::Text("".into())));
    assert_eq!(st.get("dirty"), Some(&Value::Bool(false)));
    // after a discard the other tab is clean again: switching back does not prompt
    press(&m, &mut st, tab_p, &none);
    assert!(!prompt(&st) && selected(&st, tab_p));
}

fn cents_round_half_up(num: i128, den: i128) -> i128 {
    (2 * num + den) / (2 * den)
}

#[test]
fn g04_exact_decimal_arithmetic_and_invalid_code() {
    let m = load("G04");
    let none = Scope::default();
    let (alias, list) = vp(&m);
    let inc = in_view(&m, Role::Button, "Increase quantity");
    let rm = in_view(&m, Role::Button, "Remove");
    let apply = one(&m, Role::Button, "Apply");
    let money = all(&m).into_iter().find(|n| matches!(&n.kind, NodeKind::Text(t) if t.key == "money" && matches!(&t.args[0].1, Expr::Var(v) if v == "subtotal"))).unwrap();
    let amount = |st: &State, derived: &str| -> String {
        let mut t = match &money.kind { NodeKind::Text(t) => t.clone(), _ => unreachable!() };
        t.args[0].1 = Expr::Var(derived.into());
        text_content(&m, st, &none, "en", &t).unwrap()
    };
    // independent integer reference: cents, half-up
    let reference = |qty: &[i128], prices: &[i128], pct: i128| -> (i128, i128, i128, i128) {
        let sub: i128 = qty.iter().zip(prices).map(|(q, p)| q * p).sum();
        let disc = cents_round_half_up(sub * pct, 100);
        let tax = cents_round_half_up((sub - disc) * 825, 10000);
        (sub, disc, tax, sub - disc + tax)
    };
    let fmt = |c: i128| format!("${}.{:02}", c / 100, c % 100);
    let check = |st: &State, qty: &[i128], prices: &[i128], pct: i128, label: &str| {
        let (s, d, t, tot) = reference(qty, prices, pct);
        assert_eq!(amount(st, "subtotal"), fmt(s), "{label} subtotal");
        assert_eq!(amount(st, "discount"), fmt(d), "{label} discount");
        assert_eq!(amount(st, "tax"), fmt(t), "{label} tax");
        assert_eq!(amount(st, "total"), fmt(tot), "{label} total");
    };
    let mut st = State::init(&m);
    let prices = [499, 125, 1990];
    check(&st, &[1, 1, 1], &prices, 0, "initial");
    assert_eq!(amount(&st, "subtotal"), "$26.14");
    assert_eq!(amount(&st, "tax"), "$2.16", "8.25% of 26.14 = 2.1566 -> 2.16");
    press(&m, &mut st, inc, &Scope::with(&alias, &list, 2));
    check(&st, &[1, 1, 2], &prices, 0, "backpack x2");
    // G04.2: wrong code shows the error and changes nothing
    set(&mut st, "promo", Value::Text("WRONG".into()));
    press(&m, &mut st, apply, &none);
    let err = all(&m).into_iter().find(|n| n.sem.role == Role::Alert).unwrap();
    assert!(shown(&m, &st, err, &none));
    assert!(m.string("en", match &err.kind { NodeKind::Text(t) => &t.key, _ => unreachable!() }).unwrap().contains("Invalid"));
    check(&st, &[1, 1, 2], &prices, 0, "after wrong code");
    // G04.1 (3): SAVE10
    set(&mut st, "promo", Value::Text("SAVE10".into()));
    press(&m, &mut st, apply, &none);
    assert!(!shown(&m, &st, err, &none), "error clears on a valid code");
    check(&st, &[1, 1, 2], &prices, 10, "SAVE10");
    // (4) remove Pen (row 1)
    press(&m, &mut st, rm, &Scope::with(&alias, &list, 1));
    check(&st, &[1, 2], &[499, 1990], 10, "pen removed");
    // quantity never below 1
    let dec = in_view(&m, Role::Button, "Decrease quantity");
    for _ in 0..3 {
        press(&m, &mut st, dec, &Scope::with(&alias, &list, 0));
    }
    check(&st, &[1, 2], &[499, 1990], 10, "decrease floor");
    // a few half-cent cases against the reference: every quantity pair 1..=7
    let mut st2 = State::init(&m);
    set(&mut st2, "promo", Value::Text("SAVE10".into()));
    press(&m, &mut st2, apply, &none);
    for q0 in 1..=7i128 {
        for q1 in 1..=7i128 {
            set(&mut st2, "lines", Value::List(vec![
                vec![("name".into(), Value::Text("a".into())), ("price".into(), Value::Decimal(499)), ("qty".into(), Value::Int(q0 as i64))],
                vec![("name".into(), Value::Text("b".into())), ("price".into(), Value::Decimal(125)), ("qty".into(), Value::Int(q1 as i64))],
            ]));
            check(&st2, &[q0, q1], &[499, 125], 10, &format!("{q0},{q1}"));
        }
    }
}

#[test]
fn g05_skip_logic_and_back_returns_to_where_the_user_came_from() {
    let m = load("G05");
    let none = Scope::default();
    let slot = all(&m).into_iter().find(|n| matches!(n.kind, NodeKind::Slot(_))).unwrap();
    let cases = match &slot.kind { NodeKind::Slot(p) => p.cases.clone(), _ => unreachable!() };
    assert_eq!(cases, ["s1", "s2", "s3", "s4"]);
    let current = |st: &State| -> usize {
        let p = match &slot.kind { NodeKind::Slot(p) => p, _ => unreachable!() };
        let v = match eval(&m, st, &p.on).unwrap() { Value::Enum(e) => e, o => panic!("{o:?}") };
        cases.iter().position(|c| *c == v).unwrap() + 1
    };
    // buttons live inside the case that is showing
    let in_case = |step: usize, role: Role, name: &str| -> &Node {
        let c = &slot.children[step - 1];
        let mut d = Vec::new();
        descendants(c, &mut d);
        let v: Vec<&Node> = d.into_iter().filter(|n| n.sem.role == role && name_of(&m, n) == name).collect();
        assert_eq!(v.len(), 1, "step {step} {name}");
        v[0]
    };
    let headings = |step: usize| -> Vec<String> {
        let mut d = Vec::new();
        descendants(&slot.children[step - 1], &mut d);
        d.into_iter().filter(|n| n.sem.role == Role::Heading).map(|n| name_of(&m, n)).collect()
    };
    for s in 1..=4 {
        let h = headings(s);
        assert_eq!(h.len(), 1, "exactly one heading per step");
        assert!(h[0].starts_with(&format!("Step {s}")), "{h:?}");
        assert_eq!(named_in(&m, &slot.children[s - 1], Role::Button, "Next").len() + named_in(&m, &slot.children[s - 1], Role::Button, "Finish").len(), 1);
        assert_eq!(named_in(&m, &slot.children[s - 1], Role::Button, "Back").len(), if s == 1 { 0 } else { 1 });
    }
    for answer in ["No", "Yes"] {
        let mut st = State::init(&m);
        assert_eq!(current(&st), 1);
        press(&m, &mut st, in_case(1, Role::Button, "Next"), &none);
        assert_eq!(current(&st), 2);
        press(&m, &mut st, in_case(2, Role::Radio, answer), &none);
        let radio_yes = in_case(2, Role::Radio, "Yes");
        assert_eq!(eval_bool(&m, &st, radio_yes.sem.checked.as_ref().unwrap()), Ok(answer == "Yes"));
        press(&m, &mut st, in_case(2, Role::Button, "Next"), &none);
        if answer == "Yes" {
            assert_eq!(current(&st), 3, "yes: step 3 shown");
            set(&mut st, "pet_name", Value::Text("Rex".into()));
            press(&m, &mut st, in_case(3, Role::Button, "Next"), &none);
        } else {
            assert_eq!(current(&st), 4, "no: step 3 skipped");
        }
        assert_eq!(current(&st), 4);
        press(&m, &mut st, in_case(4, Role::Button, "Back"), &none);
        assert_eq!(current(&st), if answer == "Yes" { 3 } else { 2 }, "back from 4 after {answer}");
    }
    // summary shows the name only when there is a pet
    let mut st = State::init(&m);
    set(&mut st, "pet", Value::Enum("yes".into()));
    set(&mut st, "pet_name", Value::Text("Rex".into()));
    let s = all(&m).into_iter().find(|n| matches!(&n.kind, NodeKind::Text(t) if t.key == "pet.summary")).unwrap();
    assert_eq!(en(&m, &st, &none, s), "Your pet is called Rex.");
}

fn named_in<'a>(m: &Module, n: &'a Node, role: Role, name: &str) -> Vec<&'a Node> {
    let mut d = Vec::new();
    descendants(n, &mut d);
    d.into_iter().filter(|x| x.sem.role == role && name_of(m, x) == name).collect()
}
