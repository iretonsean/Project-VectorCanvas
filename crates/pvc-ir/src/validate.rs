//! Validator over the canonical AST (G4 §9). Pure and deterministic: no clock, no randomness, no I/O,
//! no network (P2, D1). Diagnostics are produced in a fixed order (global checks, then nodes in
//! document order, then strings).

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::*;
use crate::diag::*;
use crate::eval::template_holes;
use crate::theme::contrast;

pub const E_A11Y_IMG: &str = "PVC-E-A11Y-IMG";
pub const E_A11Y_ACTION: &str = "PVC-E-A11Y-ACTION";
pub const E_A11Y_CONTRAST: &str = "PVC-E-A11Y-CONTRAST";
pub const E_CAP_SCOPE: &str = "PVC-E-CAP-SCOPE";

#[derive(Clone, Debug, PartialEq, Eq)]
enum T {
    Bool,
    Int,
    Dec,
    Text,
    Enum(Vec<String>),
    List(Vec<(String, T)>),
}

struct Env<'a> {
    m: &'a Module,
    vars: Vec<(String, T, bool)>, // name, type, assignable
    out: Vec<Diagnostic>,
}

fn ty_of_decl(t: &Ty) -> T {
    match t {
        Ty::Bool => T::Bool,
        Ty::Int => T::Int,
        Ty::Text => T::Text,
        Ty::Decimal => T::Dec,
        Ty::Enum(v) => T::Enum(v.clone()),
        Ty::List { fields, .. } => T::List(fields.iter().map(|(n, t)| (n.clone(), ty_of_decl(t))).collect()),
    }
}

fn tname(t: &T) -> String {
    match t {
        T::Bool => "bool".into(),
        T::Int => "int".into(),
        T::Dec => "decimal".into(),
        T::Text => "text".into(),
        T::Enum(_) => "enum".into(),
        T::List(_) => "list".into(),
    }
}

/// `(alias, list name)` bindings in force.
type Scope = Vec<(String, String)>;

fn same(a: &T, b: &T) -> bool {
    match (a, b) {
        (T::Enum(_), T::Enum(_)) => true,
        (T::List(_), T::List(_)) => true,
        (x, y) => x == y,
    }
}

fn numeric(t: &T) -> bool {
    matches!(t, T::Int | T::Dec)
}

impl<'a> Env<'a> {
    fn push(&mut self, d: Diagnostic, node: &str, path: &str) {
        let mut d = d.at_node(node, path);
        if let Some(sp) = self.m.span_of(node) {
            d = d.at_span(sp);
        }
        self.out.push(d);
    }

    fn mismatch(&mut self, want: &T, got: &T, node: &str, path: &str) {
        self.push(
            Diagnostic::new(E_TYPE, "type.mismatch").param("expected", tname(want)).param("found", tname(got)),
            node,
            path,
        );
    }

    fn var_names(&self, sc: &Scope) -> Vec<String> {
        let mut v: Vec<String> = self.vars.iter().map(|(n, _, _)| n.clone()).collect();
        for s in &self.m.state {
            if let Ty::Enum(vs) = &s.ty {
                v.extend(vs.iter().cloned());
            }
        }
        for (a, l) in sc {
            if let Some(StateDecl { ty: Ty::List { fields, .. }, .. }) = self.m.state.iter().find(|d| &d.name == l) {
                v.extend(fields.iter().map(|(f, _)| format!("{}.{}", a, f)));
            }
        }
        v
    }

    fn list_fields(&self, name: &str) -> Option<Vec<(String, T)>> {
        match self.vars.iter().find(|(v, _, _)| v == name) {
            Some((_, T::List(f), _)) => Some(f.clone()),
            _ => None,
        }
    }

    fn infer(&mut self, e: &Expr, sc: &Scope, node: &str, path: &str) -> Option<T> {
        match e {
            Expr::Bool(_) => Some(T::Bool),
            Expr::Int(_) => Some(T::Int),
            Expr::Dec(_) => Some(T::Dec),
            Expr::Str(_) => Some(T::Text),
            Expr::Var(n) => {
                if let Some((alias, field)) = n.split_once('.') {
                    return match sc.iter().rev().find(|(a, _)| a == alias) {
                        Some((_, list)) => match self.list_fields(list).and_then(|f| f.into_iter().find(|(x, _)| x == field)) {
                            Some((_, t)) => Some(t),
                            None => {
                                let names = self.var_names(sc);
                                let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                                self.push(
                                    Diagnostic::new(E_REF, "ref.field")
                                        .param("name", n.clone())
                                        .fix("choose", Edit::Choose { candidates: nearest(n, &refs, 3) }),
                                    node,
                                    path,
                                );
                                None
                            }
                        },
                        None => {
                            self.push(Diagnostic::new(E_REF, "ref.alias").param("name", alias), node, path);
                            None
                        }
                    };
                }
                match self.vars.iter().find(|(v, _, _)| v == n) {
                    Some((_, t, _)) => Some(t.clone()),
                    None => {
                        let names = self.var_names(sc);
                        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                        let cands = nearest(n, &refs, 3);
                        self.push(
                            Diagnostic::new(E_REF, "ref.state").param("name", n.clone()).fix("choose", Edit::Choose { candidates: cands }),
                            node,
                            path,
                        );
                        None
                    }
                }
            }
            Expr::Variant(v) => {
                let found = self.m.state.iter().find_map(|s| match &s.ty {
                    Ty::Enum(vs) if vs.contains(v) => Some(vs.clone()),
                    _ => None,
                });
                match found {
                    Some(vs) => Some(T::Enum(vs)),
                    None => {
                        self.push(Diagnostic::new(E_REF, "ref.variant").param("name", v.clone()), node, path);
                        None
                    }
                }
            }
            Expr::Not(a) => {
                let t = self.infer(a, sc, node, path)?;
                if t != T::Bool {
                    self.mismatch(&T::Bool, &t, node, path);
                }
                Some(T::Bool)
            }
            Expr::And(a, b) | Expr::Or(a, b) => {
                for x in [a, b] {
                    if let Some(t) = self.infer(x, sc, node, path) {
                        if t != T::Bool {
                            self.mismatch(&T::Bool, &t, node, path);
                        }
                    }
                }
                Some(T::Bool)
            }
            Expr::Add(a, b) | Expr::Sub(a, b) => {
                let (ta, tb) = (self.infer(a, sc, node, path), self.infer(b, sc, node, path));
                match (ta, tb) {
                    (Some(x), Some(y)) if numeric(&x) && numeric(&y) => Some(if x == T::Dec || y == T::Dec { T::Dec } else { T::Int }),
                    (Some(x), Some(y)) => {
                        let bad = if !numeric(&x) { x } else { y };
                        self.mismatch(&T::Int, &bad, node, path);
                        None
                    }
                    _ => None,
                }
            }
            Expr::Mul(a, b) => {
                let (ta, tb) = (self.infer(a, sc, node, path), self.infer(b, sc, node, path));
                match (ta, tb) {
                    (Some(T::Int), Some(T::Int)) => Some(T::Int),
                    (Some(T::Int), Some(T::Dec)) | (Some(T::Dec), Some(T::Int)) => Some(T::Dec),
                    (Some(T::Dec), Some(T::Dec)) => {
                        self.push(Diagnostic::new(E_TYPE, "type.decimal-times-decimal"), node, path);
                        None
                    }
                    (Some(x), Some(y)) => {
                        let bad = if !numeric(&x) { x } else { y };
                        self.mismatch(&T::Int, &bad, node, path);
                        None
                    }
                    _ => None,
                }
            }
            Expr::Cmp(op, a, b) => {
                let (ta, tb) = (self.infer(a, sc, node, path), self.infer(b, sc, node, path));
                if let (Some(ta), Some(tb)) = (ta, tb) {
                    let ordered = matches!(op, CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge);
                    let ok = if numeric(&ta) && numeric(&tb) { true } else { same(&ta, &tb) };
                    if !ok {
                        self.mismatch(&ta, &tb, node, path);
                    } else if ordered && !numeric(&ta) {
                        self.mismatch(&T::Int, &ta, node, path);
                    } else if matches!(ta, T::List(_)) {
                        self.mismatch(&T::Int, &ta, node, path);
                    }
                }
                Some(T::Bool)
            }
            Expr::Call(f, args) => self.infer_call(*f, args, sc, node, path),
        }
    }

    fn infer_call(&mut self, f: Func, args: &[Expr], sc: &Scope, node: &str, path: &str) -> Option<T> {
        let arity_ok = match f {
            Func::Count => args.len() == 1 || args.len() == 2,
            Func::IsEmpty | Func::Length | Func::IsEmail => args.len() == 1,
            Func::Contains | Func::Sum | Func::Percent => args.len() == 2,
        };
        if !arity_ok {
            self.push(
                Diagnostic::new(E_TYPE, "type.arity").param("func", f.as_str()).param("found", args.len().to_string()),
                node,
                path,
            );
            return None;
        }
        match f {
            Func::Count | Func::Sum => {
                let list = match &args[0] {
                    Expr::Var(l) => l.clone(),
                    _ => {
                        self.push(Diagnostic::new(E_TYPE, "type.expected-list-name").param("func", f.as_str()), node, path);
                        return None;
                    }
                };
                if self.list_fields(&list).is_none() {
                    let lists: Vec<String> = self.vars.iter().filter(|(_, t, _)| matches!(t, T::List(_))).map(|(n, _, _)| n.clone()).collect();
                    let refs: Vec<&str> = lists.iter().map(|s| s.as_str()).collect();
                    self.push(
                        Diagnostic::new(E_REF, "ref.list").param("name", list.clone()).fix("choose", Edit::Choose { candidates: nearest(&list, &refs, 3) }),
                        node,
                        path,
                    );
                    return None;
                }
                if args.len() == 2 {
                    let mut s2 = sc.clone();
                    s2.push(("it".to_string(), list));
                    let t = self.infer(&args[1], &s2, node, path);
                    return match (f, t) {
                        (Func::Count, Some(T::Bool)) => Some(T::Int),
                        (Func::Count, Some(t)) => {
                            self.mismatch(&T::Bool, &t, node, path);
                            Some(T::Int)
                        }
                        (_, Some(t)) if numeric(&t) => Some(t),
                        (_, Some(t)) => {
                            self.mismatch(&T::Int, &t, node, path);
                            None
                        }
                        (_, None) => None,
                    };
                }
                Some(T::Int)
            }
            Func::Percent => {
                let a = self.infer(&args[0], sc, node, path);
                let b = self.infer(&args[1], sc, node, path);
                if let Some(t) = a {
                    if t != T::Dec {
                        self.mismatch(&T::Dec, &t, node, path);
                    }
                }
                if let Some(t) = b {
                    if !numeric(&t) {
                        self.mismatch(&T::Dec, &t, node, path);
                    }
                }
                Some(T::Dec)
            }
            Func::IsEmpty | Func::Length => {
                let t = self.infer(&args[0], sc, node, path);
                if let Some(t) = &t {
                    if !matches!(t, T::Text | T::List(_)) {
                        self.mismatch(&T::Text, t, node, path);
                    }
                }
                Some(if f == Func::IsEmpty { T::Bool } else { T::Int })
            }
            Func::Contains | Func::IsEmail => {
                for a in args {
                    if let Some(t) = self.infer(a, sc, node, path) {
                        if t != T::Text {
                            self.mismatch(&T::Text, &t, node, path);
                        }
                    }
                }
                Some(T::Bool)
            }
        }
    }

    fn check_bool(&mut self, e: &Expr, sc: &Scope, node: &str, path: &str) {
        if let Some(t) = self.infer(e, sc, node, path) {
            if t != T::Bool {
                self.mismatch(&T::Bool, &t, node, path);
            }
        }
    }
}

fn collect_keys(n: &Node, out: &mut Vec<(String, String, String)>) {
    // (key, node id, path)
    if let NodeKind::Text(t) = &n.kind {
        if t.value.is_none() {
            out.push((t.key.clone(), n.id.clone(), "data.key".into()));
        }
    }
    if let Some(Name::Key(k)) = &n.sem.name {
        out.push((k.clone(), n.id.clone(), "sem.name".into()));
    }
    if let Some(k) = &n.sem.description {
        out.push((k.clone(), n.id.clone(), "sem.desc".into()));
    }
    for c in &n.children {
        collect_keys(c, out);
    }
}

fn descendant_text_keys(n: &Node, out: &mut Vec<String>) {
    if let NodeKind::Text(t) = &n.kind {
        if t.value.is_none() {
            out.push(t.key.clone());
        }
    }
    for c in &n.children {
        descendant_text_keys(c, out);
    }
}

/// Alias scopes: node id -> bindings in force for that node's expressions and handlers.
fn scopes(n: &Node, sc: &Scope, out: &mut BTreeMap<String, Scope>) {
    out.insert(n.id.clone(), sc.clone());
    let mut inner = sc.clone();
    if let NodeKind::View(v) = &n.kind {
        inner.push((v.alias.clone(), v.list.clone()));
    }
    for c in &n.children {
        scopes(c, &inner, out);
    }
}

fn url_prefix(u: &str) -> String {
    match u.find("://") {
        Some(i) => {
            let rest = &u[i + 3..];
            let end = rest.find('/').map(|j| i + 3 + j + 1).unwrap_or(u.len());
            let mut p = u[..end].to_string();
            if !p.ends_with('/') {
                p.push('/');
            }
            p
        }
        None => u.to_string(),
    }
}

fn contrast_walk(m: &Module, n: &Node, bg: [u8; 4], env: &mut Env) {
    let mut bg = bg;
    if let Some(f) = &n.paint.fill {
        if let Some(c) = m.resolve_color(f) {
            // composite a translucent fill over the backdrop
            let a = c[3] as u32;
            bg = [
                ((c[0] as u32 * a + bg[0] as u32 * (255 - a)) / 255) as u8,
                ((c[1] as u32 * a + bg[1] as u32 * (255 - a)) / 255) as u8,
                ((c[2] as u32 * a + bg[2] as u32 * (255 - a)) / 255) as u8,
                255,
            ];
        }
    }
    if let NodeKind::Text(t) = &n.kind {
        if let Some(fg) = m.resolve_color(&t.color) {
            let large = t.size.px() >= 24.0 || (t.size.px() >= 18.66 && t.weight >= 700);
            let need = if large { 3.0 } else { 4.5 };
            let r = contrast(fg, bg);
            if r < need {
                let cands: Vec<String> = ["fg", "body", "muted", "ink-dark", "ink-light"]
                    .iter()
                    .filter(|tok| m.theme.colors.iter().find(|(k, _)| k == *tok).map(|(_, c)| contrast(*c, bg) >= need).unwrap_or(false))
                    .map(|s| s.to_string())
                    .take(3)
                    .collect();
                let mut d = Diagnostic::new(E_A11Y_CONTRAST, "a11y.contrast")
                    .param("ratio", format!("{:.2}", r))
                    .param("required", format!("{:.1}", need));
                if !cands.is_empty() {
                    d = d.fix("set-color", Edit::Choose { candidates: cands });
                }
                env.push(d, &n.id, "data.color");
            }
        }
    }
    for c in &n.children {
        contrast_walk(m, c, bg, env);
    }
}

/// Validate a canonical module. Returns diagnostics in a fixed order.
pub fn validate(m: &Module) -> Vec<Diagnostic> {
    let mut env = Env { m, vars: Vec::new(), out: Vec::new() };
    for s in &m.state {
        env.vars.push((s.name.clone(), ty_of_decl(&s.ty), true));
    }
    let none: Scope = Vec::new();
    // derived values: typed in declaration order, may only use earlier names
    for d in &m.derived {
        let t = env.infer(&d.expr, &none, "", &format!("derived.{}", d.name));
        env.vars.push((d.name.clone(), t.unwrap_or(T::Bool), false));
    }

    // ---- manifest-level
    if m.manifest.fonts.is_empty() {
        env.out.push(Diagnostic::new("PVC-W-SYSTEM-FONT", "font.system").at_path("manifest.fonts"));
    }
    let mut seen_caps = BTreeSet::new();
    for c in &m.manifest.caps {
        if !seen_caps.insert(c.id.clone()) {
            env.out.push(Diagnostic::new(E_ID_COLLISION, "id.duplicate-capability").at_path("manifest.caps").param("id", c.id.clone()));
        }
    }
    for e in &m.effects {
        if !m.manifest.caps.iter().any(|c| c.id == e.cap) {
            let line = if e.cap == "net" || e.cap.starts_with("net.") {
                format!("cap {} hosts=[\"<host>\"] methods=[\"<METHOD>\"]", e.cap)
            } else {
                format!("cap {}", e.cap)
            };
            env.out.push(
                Diagnostic::new(E_CAP_UNDECLARED, "cap.undeclared")
                    .at_path(&format!("effects.{}", e.name))
                    .param("effect", e.name.clone())
                    .param("cap", e.cap.clone())
                    .fix("declare-capability", Edit::AddLine { line }),
            );
        }
    }

    // ---- nodes
    let mut sc_map = BTreeMap::new();
    scopes(&m.root, &none, &mut sc_map);
    let mut ids = BTreeSet::new();
    let mut all: Vec<&Node> = Vec::new();
    m.walk(&mut |n, _| all.push(n));
    let id_set: BTreeSet<&str> = all.iter().map(|n| n.id.as_str()).collect();
    let mut key_uses: Vec<(String, String, String)> = Vec::new();
    collect_keys(&m.root, &mut key_uses);
    let default_locale = m.default_locale().to_string();
    let non_empty = |k: &str| -> bool { m.string(&default_locale, k).map(|s| !s.is_empty()).unwrap_or(false) };

    for n in &all {
        let id = n.id.as_str();
        let sc: Scope = sc_map.get(id).cloned().unwrap_or_default();
        if !ids.insert(id.to_string()) {
            env.push(
                Diagnostic::new(E_ID_COLLISION, "id.duplicate-node").param("id", id).fix("choose", Edit::Choose { candidates: vec![format!("{}-2", id)] }),
                id,
                "id",
            );
        }
        // theme tokens
        let mut colors: Vec<(&Color, &str)> = Vec::new();
        if let Some(c) = &n.paint.fill {
            colors.push((c, "paint.fill"));
        }
        if let Some(b) = &n.paint.border {
            colors.push((&b.color, "paint.border.color"));
        }
        match &n.kind {
            NodeKind::Text(t) => colors.push((&t.color, "data.color")),
            NodeKind::TextInput(t) => colors.push((&t.color, "data.color")),
            _ => {}
        }
        for (c, path) in colors {
            if let Color::Token(t) = c {
                if !m.theme.colors.iter().any(|(n, _)| n == t) {
                    let names: Vec<&str> = m.theme.colors.iter().map(|(n, _)| n.as_str()).collect();
                    env.push(
                        Diagnostic::new(E_UNKNOWN_TOKEN, "token.unknown-color").param("found", t.clone()).fix("choose", Edit::Choose { candidates: nearest(t, &names, 3) }),
                        id,
                        path,
                    );
                }
            }
        }
        // fonts
        let family = match &n.kind {
            NodeKind::Text(t) => Some(&t.family),
            NodeKind::TextInput(t) => Some(&t.family),
            _ => None,
        };
        if let Some(f) = family {
            if !m.manifest.fonts.is_empty() && !m.manifest.fonts.iter().any(|x| &x.family == f) {
                env.push(Diagnostic::new(E_REF, "ref.font").param("family", f.clone()), id, "data.family");
            }
        }
        // expressions
        if let Some(e) = &n.show {
            env.check_bool(e, &sc, id, "show");
        }
        for (name, e) in [
            ("sem.disabled", &n.sem.disabled),
            ("sem.checked", &n.sem.checked),
            ("sem.pressed", &n.sem.pressed),
            ("sem.invalid", &n.sem.invalid),
            ("sem.selected", &n.sem.selected),
        ] {
            if let Some(e) = e {
                env.check_bool(e, &sc, id, name);
            }
        }
        match &n.kind {
            NodeKind::TextInput(t) => {
                env.check_bool(&t.masked, &sc, id, "data.masked");
                let target = if t.bind.contains('.') {
                    env.infer(&Expr::Var(t.bind.clone()), &sc, id, "data.bind")
                } else {
                    match env.vars.iter().find(|(v, _, _)| *v == t.bind).cloned() {
                        None => {
                            let names: Vec<&str> = env.vars.iter().map(|(n, _, _)| n.as_str()).collect();
                            env.push(
                                Diagnostic::new(E_REF, "ref.state").param("name", t.bind.clone()).fix("choose", Edit::Choose { candidates: nearest(&t.bind, &names, 3) }),
                                id,
                                "data.bind",
                            );
                            None
                        }
                        Some((_, ty, assignable)) => {
                            if !assignable {
                                env.push(Diagnostic::new(E_TYPE, "type.not-assignable").param("name", t.bind.clone()), id, "data.bind");
                            }
                            Some(ty)
                        }
                    }
                };
                if let Some(ty) = target {
                    if ty != T::Text {
                        env.mismatch(&T::Text, &ty, id, "data.bind");
                    }
                }
            }
            NodeKind::Text(t) => {
                if let Some(v) = &t.value {
                    env.infer(v, &sc, id, "data.value");
                }
                for (an, ae) in &t.args {
                    env.infer(ae, &sc, id, &format!("data.args.{}", an));
                }
                if t.value.is_none() {
                    // every hole of every locale's string is supplied; every supplied arg is a hole somewhere
                    let mut holes_seen: BTreeSet<String> = BTreeSet::new();
                    for tab in &m.strings.tables {
                        if let Some((_, txt)) = tab.entries.iter().find(|(k, _)| *k == t.key) {
                            match template_holes(txt) {
                                Err(()) => env.push(
                                    Diagnostic::new(E_SCHEMA, "schema.bad-template").param("key", t.key.clone()).param("locale", tab.locale.clone()),
                                    id,
                                    "data.key",
                                ),
                                Ok(hs) => {
                                    for h in hs {
                                        if !t.args.iter().any(|(a, _)| *a == h) {
                                            env.push(
                                                Diagnostic::new(E_REF, "ref.text-hole")
                                                    .param("hole", h.clone())
                                                    .param("key", t.key.clone())
                                                    .fix("add-prop", Edit::SetProp { prop: h.clone(), value: "(<expr>)".into() }),
                                                id,
                                                "data.args",
                                            );
                                        }
                                        holes_seen.insert(h);
                                    }
                                }
                            }
                        }
                    }
                    for (a, _) in &t.args {
                        if !holes_seen.contains(a) && m.strings.tables.iter().any(|tab| tab.entries.iter().any(|(k, _)| *k == t.key)) {
                            let hs: Vec<&str> = holes_seen.iter().map(|s| s.as_str()).collect();
                            env.push(
                                Diagnostic::new(E_REF, "ref.text-arg")
                                    .param("arg", a.clone())
                                    .param("key", t.key.clone())
                                    .fix("choose", Edit::Choose { candidates: nearest(a, &hs, 3) }),
                                id,
                                "data.args",
                            );
                        }
                    }
                }
            }
            NodeKind::Slot(p) => {
                let ty = env.infer(&p.on, &sc, id, "data.on");
                let variants: Option<Vec<String>> = match &ty {
                    Some(T::Enum(v)) => Some(v.clone()),
                    Some(T::Bool) => Some(vec!["true".to_string(), "false".to_string()]),
                    Some(other) => {
                        let o = other.clone();
                        env.mismatch(&T::Enum(vec![]), &o, id, "data.on");
                        None
                    }
                    None => None,
                };
                if let Some(vs) = variants {
                    for c in &p.cases {
                        if !vs.contains(c) {
                            let refs: Vec<&str> = vs.iter().map(|s| s.as_str()).collect();
                            env.push(
                                Diagnostic::new(E_REF, "ref.variant").param("name", c.clone()).fix("choose", Edit::Choose { candidates: nearest(c, &refs, 3) }),
                                id,
                                "data.cases",
                            );
                        }
                    }
                    for v in &vs {
                        if !p.cases.contains(v) {
                            env.push(
                                Diagnostic::new("PVC-W-SLOT-INCOMPLETE", "slot.missing-case").param("case", v.clone()).fix(
                                    "add-line",
                                    Edit::AddLine { line: format!("case {} {{ }}", v) },
                                ),
                                id,
                                "data.cases",
                            );
                        }
                    }
                }
            }
            NodeKind::View(v) => {
                if env.list_fields(&v.list).is_none() {
                    let lists: Vec<String> = env.vars.iter().filter(|(_, t, _)| matches!(t, T::List(_))).map(|(n, _, _)| n.clone()).collect();
                    let refs: Vec<&str> = lists.iter().map(|s| s.as_str()).collect();
                    env.push(
                        Diagnostic::new(E_REF, "ref.list").param("name", v.list.clone()).fix("choose", Edit::Choose { candidates: nearest(&v.list, &refs, 3) }),
                        id,
                        "data.list",
                    );
                }
                if let Some(f) = &v.filter {
                    let mut s2 = sc.clone();
                    s2.push((v.alias.clone(), v.list.clone()));
                    env.check_bool(f, &s2, id, "data.filter");
                }
            }
            _ => {}
        }
        if n.layout.dir == Dir::Grid && n.layout.cols == 0 {
            env.push(Diagnostic::new(E_SCHEMA, "schema.bad-cols"), id, "layout.cols");
        }
        for (hi, h) in n.on.iter().enumerate() {
            let hp = format!("on[{}]", hi);
            if let Some(g) = &h.guard {
                env.check_bool(g, &sc, id, &format!("{}.guard", hp));
            }
            for (ai, a) in h.actions.iter().enumerate() {
                let ap = format!("{}.do[{}]", hp, ai);
                match a {
                    Action::Assign(target, e) => {
                        let te = env.infer(e, &sc, id, &ap);
                        let (ty, assignable) = if target.contains('.') {
                            (env.infer(&Expr::Var(target.clone()), &sc, id, &ap), true)
                        } else {
                            match env.vars.iter().find(|(v, _, _)| v == target).cloned() {
                                None => {
                                    let names: Vec<&str> = env.vars.iter().filter(|v| v.2).map(|(n, _, _)| n.as_str()).collect();
                                    env.push(
                                        Diagnostic::new(E_REF, "ref.state").param("name", target.clone()).fix("choose", Edit::Choose { candidates: nearest(target, &names, 3) }),
                                        id,
                                        &ap,
                                    );
                                    (None, true)
                                }
                                Some((_, ty, assignable)) => (Some(ty), assignable),
                            }
                        };
                        if !assignable {
                            env.push(Diagnostic::new(E_TYPE, "type.not-assignable").param("name", target.clone()), id, &ap);
                        } else if let (Some(ty), Some(te)) = (ty, te) {
                            let ok = if numeric(&ty) && numeric(&te) { ty == te || (ty == T::Dec && te == T::Int) } else { same(&ty, &te) };
                            if !ok || matches!(ty, T::List(_)) {
                                env.mismatch(&ty, &te, id, &ap);
                            }
                        }
                    }
                    Action::Append(list, fields) => match env.list_fields(list) {
                        None => {
                            let lists: Vec<String> = env.vars.iter().filter(|(_, t, _)| matches!(t, T::List(_))).map(|(n, _, _)| n.clone()).collect();
                            let refs: Vec<&str> = lists.iter().map(|s| s.as_str()).collect();
                            env.push(
                                Diagnostic::new(E_REF, "ref.list").param("name", list.clone()).fix("choose", Edit::Choose { candidates: nearest(list, &refs, 3) }),
                                id,
                                &ap,
                            );
                        }
                        Some(decl) => {
                            for (f, e) in fields {
                                let te = env.infer(e, &sc, id, &ap);
                                match decl.iter().find(|(n, _)| n == f) {
                                    None => {
                                        let names: Vec<&str> = decl.iter().map(|(n, _)| n.as_str()).collect();
                                        env.push(
                                            Diagnostic::new(E_REF, "ref.field").param("name", f.clone()).fix("choose", Edit::Choose { candidates: nearest(f, &names, 3) }),
                                            id,
                                            &ap,
                                        );
                                    }
                                    Some((_, ft)) => {
                                        if let Some(te) = te {
                                            let ok = if numeric(ft) && numeric(&te) { *ft == te || (*ft == T::Dec && te == T::Int) } else { same(ft, &te) };
                                            if !ok {
                                                env.mismatch(ft, &te, id, &ap);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Action::Remove(alias) => {
                        if !sc.iter().any(|(a, _)| a == alias) {
                            let names: Vec<&str> = sc.iter().map(|(a, _)| a.as_str()).collect();
                            env.push(
                                Diagnostic::new(E_REF, "ref.alias").param("name", alias.clone()).fix("choose", Edit::Choose { candidates: nearest(alias, &names, 3) }),
                                id,
                                &ap,
                            );
                        }
                    }
                    Action::Emit(eff) => {
                        if !m.effects.iter().any(|e| &e.name == eff) {
                            let names: Vec<&str> = m.effects.iter().map(|e| e.name.as_str()).collect();
                            env.push(
                                Diagnostic::new(E_REF, "ref.effect").param("name", eff.clone()).fix("choose", Edit::Choose { candidates: nearest(eff, &names, 3) }),
                                id,
                                &ap,
                            );
                        }
                    }
                    Action::Submit(target) => {
                        let ok = all.iter().any(|x| &x.id == target && x.on.iter().any(|h| h.event == Event::Submit));
                        if !ok {
                            env.push(Diagnostic::new(E_REF, "ref.submit-target").param("id", target.clone()), id, &ap);
                        }
                    }
                    Action::OpenUrl(u) => {
                        let caps: Vec<&Capability> = m.manifest.caps.iter().filter(|c| c.kind == "open-url").collect();
                        let prefix = url_prefix(u);
                        if caps.is_empty() {
                            env.push(
                                Diagnostic::new(E_CAP_UNDECLARED, "cap.undeclared")
                                    .param("cap", "open-url")
                                    .param("url", u.clone())
                                    .fix("declare-capability", Edit::AddLine { line: format!("cap open-url.main urls=[\"{}\"]", prefix) }),
                                id,
                                &ap,
                            );
                        } else if !caps.iter().any(|c| c.hosts.iter().any(|p| u.starts_with(p.as_str()))) {
                            env.push(
                                Diagnostic::new(E_CAP_SCOPE, "cap.scope")
                                    .param("cap", "open-url")
                                    .param("url", u.clone())
                                    .fix("widen-capability", Edit::AddLine { line: format!("cap {} urls=[…, \"{}\"]", caps[0].id, prefix) }),
                                id,
                                &ap,
                            );
                        }
                    }
                }
            }
        }
        // semantics references
        let mut refs: Vec<(String, &str)> = Vec::new();
        if let Some(x) = &n.sem.labelled_by {
            refs.push((x.clone(), "sem.labelled_by"));
        }
        for x in &n.sem.described_by {
            refs.push((x.clone(), "sem.described_by"));
        }
        if let Some(x) = &n.sem.error_message {
            refs.push((x.clone(), "sem.error_message"));
        }
        for (r, p) in refs {
            if !id_set.contains(r.as_str()) {
                env.push(Diagnostic::new(E_REF, "ref.node").param("id", r), id, p);
            }
        }
        // images: named or decorative (G4 §8.2)
        let role = n.sem.role;
        if role == Role::Image && !n.sem.decorative {
            let mut missing = n.sem.name.is_none();
            match &n.sem.name {
                Some(Name::Key(k)) => {
                    if m.strings.tables.iter().any(|t| t.entries.iter().any(|(kk, _)| kk == k)) && !non_empty(k) {
                        missing = true;
                    }
                }
                Some(Name::FromNode(t)) => match all.iter().find(|x| &x.id == t) {
                    Some(Node { kind: NodeKind::Text(tp), .. }) if non_empty(&tp.key) || tp.value.is_some() => {}
                    _ => env.push(Diagnostic::new(E_REF, "ref.node").param("id", t.clone()), id, "sem.name"),
                },
                None => {}
            }
            if missing {
                env.push(
                    Diagnostic::new(E_A11Y_IMG, "a11y.image-unnamed")
                        .param("kind", n.kind.name())
                        .fix("set-alt", Edit::SetProp { prop: "alt".into(), value: "\"<text alternative>\"".into() })
                        .fix("mark-decorative", Edit::SetProp { prop: "decorative".into(), value: String::new() }),
                    id,
                    "sem.name",
                );
            }
        }
        // accessible name (P5)
        if role.needs_name() && !n.sem.decorative {
            let mut missing = false;
            match &n.sem.name {
                None => missing = true,
                Some(Name::Key(k)) => {
                    if m.strings.tables.iter().any(|t| t.entries.iter().any(|(kk, _)| kk == k)) && !non_empty(k) {
                        missing = true;
                    }
                }
                Some(Name::FromNode(t)) => match all.iter().find(|x| &x.id == t) {
                    None => env.push(Diagnostic::new(E_REF, "ref.node").param("id", t.clone()), id, "sem.name"),
                    Some(tn) => match &tn.kind {
                        NodeKind::Text(tp) if non_empty(&tp.key) || tp.value.is_some() => {}
                        NodeKind::Text(_) => missing = true,
                        _ => env.push(Diagnostic::new(E_REF, "ref.node").param("id", t.clone()), id, "sem.name"),
                    },
                },
            }
            if missing {
                let mut keys = Vec::new();
                descendant_text_keys(n, &mut keys);
                let mut d = Diagnostic::new(E_A11Y_NAME, "a11y.name-missing").param("role", role.as_str());
                if let Some(k) = keys.iter().find(|k| non_empty(k)) {
                    d = d.fix("set-name", Edit::SetProp { prop: "name".into(), value: format!("@{}", k) });
                } else {
                    d = d.fix("set-name", Edit::SetProp { prop: "name".into(), value: "\"<name>\"".into() });
                }
                env.push(d, id, "sem.name");
            }
        }
        // required action (G4 §8.1)
        if !n.sem.decorative
            && matches!(role, Role::Button | Role::Checkbox | Role::Switch | Role::Radio | Role::Tab | Role::Link)
            && !n.on.iter().any(|h| h.event == Event::Press)
        {
            let what = if role == Role::Link { "href" } else { "on.press" };
            env.push(
                Diagnostic::new(E_A11Y_ACTION, "a11y.action-missing")
                    .param("role", role.as_str())
                    .fix("add-handler", Edit::SetProp { prop: what.into(), value: if role == Role::Link { "\"<url>\"".into() } else { "{ <assignments> }".into() } }),
                id,
                "on",
            );
        }
    }

    // ---- contrast (static colours against the resolved backdrop, WCAG 2.2 AA)
    let white = [255, 255, 255, 255];
    contrast_walk(m, &m.root, white, &mut env);

    // ---- strings
    let tables = &m.strings.tables;
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    for (key, node, path) in &key_uses {
        if !seen.insert((key.clone(), node.clone())) && path != "sem.name" {
            continue;
        }
        let defined_anywhere = tables.iter().any(|t| t.entries.iter().any(|(k, _)| k == key));
        if !defined_anywhere {
            let all_keys: Vec<&str> = tables.iter().flat_map(|t| t.entries.iter().map(|(k, _)| k.as_str())).collect();
            env.push(
                Diagnostic::new(E_REF, "ref.string").param("key", key.clone()).fix("choose", Edit::Choose { candidates: nearest(key, &all_keys, 3) }),
                node,
                path,
            );
            continue;
        }
        for t in tables {
            if !t.entries.iter().any(|(k, _)| k == key) {
                let dedupe = (format!("{}\u{0}{}", key, t.locale), String::new());
                if !seen.insert(dedupe) {
                    continue;
                }
                env.push(
                    Diagnostic::new(E_STRING_MISSING, "string.missing")
                        .param("key", key.clone())
                        .param("locale", t.locale.clone())
                        .fix("add-string", Edit::AddLine { line: format!("strings {} {{ {} \"<text>\" }}", t.locale, key) }),
                    node,
                    path,
                );
            }
        }
    }
    env.out
}

/// True if any diagnostic is an error.
pub fn has_errors(d: &[Diagnostic]) -> bool {
    d.iter().any(|x| x.is_error())
}
