//! Terse -> canonical expansion (G4 §3, P3): resolves tokens, assigns stable node ids and string
//! keys, expands `std` components into core kinds with fully explicit layout/paint/semantics/behaviour.
//! Syntax-level and schema-level errors are raised here; reference-level checks live in `validate`.

use std::collections::BTreeMap;

use crate::ast::*;
use crate::diag::*;
use crate::parse::*;
use crate::pred::nfc;
use crate::theme;

/// Core + std node kinds understood by this build.
pub const CORE_KINDS: &[&str] = &[
    "surface", "box", "row", "col", "column", "grid", "text", "text-input", "shape", "layer", "slot", "case", "view",
];
pub const STD_KINDS: &[&str] = &[
    "card", "avatar", "button", "link", "field", "password-field", "checkbox", "switch", "radio", "tab", "list", "item",
    "form",
];

pub const CAP_KINDS: &[&str] = &[
    "net", "storage", "file.pick", "clipboard.write", "clipboard.read", "camera", "mic", "location",
    "notify", "open-url", "share",
];

pub(crate) type Diags = Vec<Diagnostic>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Surface,
    Box,
    Grid,
    List,
    Item,
}

pub(crate) struct Ex {
    pub theme: Theme,
    pub diags: Diags,
    pub prov: Vec<(Id, Span)>,
    pub default_locale: String,
    pub inline: Vec<(String, String)>,
    pub names: Vec<String>,
    pub variants: Vec<String>,
    pub form_stack: Vec<Id>,
    pub avatar_pending: Vec<Id>,
}

pub(crate) struct Pr<'a> {
    pub raw: &'a RawNode,
    used_props: Vec<bool>,
    used_flags: Vec<bool>,
    used_args: usize,
}

impl<'a> Pr<'a> {
    pub fn new(raw: &'a RawNode) -> Self {
        Pr { raw, used_props: vec![false; raw.props.len()], used_flags: vec![false; raw.flags.len()], used_args: 0 }
    }
    pub fn flag(&mut self, name: &str) -> bool {
        let mut f = false;
        for (i, (n, _)) in self.raw.flags.iter().enumerate() {
            if n == name {
                self.used_flags[i] = true;
                f = true;
            }
        }
        f
    }
    pub fn prop(&mut self, name: &str) -> Option<&'a Prop> {
        let mut out = None;
        for (i, p) in self.raw.props.iter().enumerate() {
            if p.name == name {
                self.used_props[i] = true;
                out = Some(p);
            }
        }
        out
    }
    pub fn arg(&mut self, i: usize) -> Option<&'a (Atom, Span)> {
        let a = self.raw.args.get(i);
        if a.is_some() {
            self.used_args = self.used_args.max(i + 1);
        }
        a
    }
}

pub(crate) fn dec_to_len(s: &str) -> Option<Len> {
    let (ip, fp) = match s.split_once('.') {
        Some((a, b)) => (a, b),
        None => (s, ""),
    };
    if fp.len() > 6 || ip.len() > 6 {
        return None;
    }
    let i: u128 = ip.parse().ok()?;
    let mut v = i * 64;
    if !fp.is_empty() {
        let f: u128 = fp.parse().ok()?;
        let d = 10u128.pow(fp.len() as u32);
        v += (f * 64 * 2 + d) / (2 * d);
    }
    if v > i32::MAX as u128 {
        return None;
    }
    Some(Len(v as i32))
}

pub(crate) fn line_height(size: Len) -> Len {
    Len(((size.0 as i64 * 140 + 50) / 100) as i32)
}

pub(crate) struct TStyle {
    pub size: Len,
    pub weight: u16,
    pub color: Color,
}

pub(crate) fn tok(n: &str) -> Color {
    Color::Token(n.to_string())
}

fn base_layout() -> Layout {
    Layout {
        dir: Dir::Col,
        gap: Len::ZERO,
        pad: Edges::all(Len::ZERO),
        align: Align::Start,
        justify: Justify::Start,
        w: Size::Hug,
        h: Size::Hug,
        cols: 0,
    }
}

fn base_paint() -> Paint {
    Paint { fill: None, border: None, shape: Shape::RoundedRect(Len::ZERO) }
}

pub(crate) fn mk_node(id: &str, kind: NodeKind, role: Role) -> Node {
    Node {
        id: id.to_string(),
        kind,
        layout: base_layout(),
        paint: base_paint(),
        sem: Semantics::new(role),
        show: None,
        on: Vec::new(),
        at: Vec::new(),
        children: Vec::new(),
    }
}

pub(crate) fn describe_atom(a: &Atom) -> String {
    match a {
        Atom::Str(_) => "string".into(),
        Atom::Key(k) => format!("@{}", k),
        Atom::Num(n) => format!("number:{}", n),
        Atom::Word(w) => format!("word:{}", w),
        Atom::Expr(_) => "expression".into(),
        Atom::List(_) => "list".into(),
        Atom::Handler(_) => "handler".into(),
        Atom::Rows(_) => "rows".into(),
    }
}

pub(crate) fn first_text(n: &Node) -> Option<Id> {
    if let NodeKind::Text(_) = n.kind {
        if !n.sem.decorative {
            return Some(n.id.clone());
        }
    }
    for c in &n.children {
        if let Some(t) = first_text(c) {
            return Some(t);
        }
    }
    None
}

impl Ex {
    pub fn span_diag(&mut self, d: Diagnostic) {
        self.diags.push(d);
    }

    pub fn resolve(&self, e: Expr) -> Expr {
        match e {
            Expr::Var(n) => {
                if self.names.contains(&n) {
                    Expr::Var(n)
                } else if self.variants.contains(&n) {
                    Expr::Variant(n)
                } else {
                    Expr::Var(n)
                }
            }
            Expr::Str(s) => Expr::Str(nfc(&s)),
            Expr::Add(a, b) => Expr::Add(Box::new(self.resolve(*a)), Box::new(self.resolve(*b))),
            Expr::Sub(a, b) => Expr::Sub(Box::new(self.resolve(*a)), Box::new(self.resolve(*b))),
            Expr::Mul(a, b) => Expr::Mul(Box::new(self.resolve(*a)), Box::new(self.resolve(*b))),
            Expr::Not(a) => Expr::Not(Box::new(self.resolve(*a))),
            Expr::And(a, b) => Expr::And(Box::new(self.resolve(*a)), Box::new(self.resolve(*b))),
            Expr::Or(a, b) => Expr::Or(Box::new(self.resolve(*a)), Box::new(self.resolve(*b))),
            Expr::Cmp(o, a, b) => Expr::Cmp(o, Box::new(self.resolve(*a)), Box::new(self.resolve(*b))),
            Expr::Call(f, args) => Expr::Call(f, args.into_iter().map(|a| self.resolve(a)).collect()),
            x => x,
        }
    }

    pub fn schema(&mut self, span: Span, key: &'static str, prop: &str, found: &str, fields: &[&str]) {
        let mut d = Diagnostic::new(E_SCHEMA, key).at_span(span).param("prop", prop);
        if !found.is_empty() {
            d = d.param("found", found);
        }
        if !fields.is_empty() {
            d = d.fix("fields", Edit::FieldList { fields: fields.iter().map(|s| s.to_string()).collect() });
        }
        self.span_diag(d);
    }

    fn choose_fix(&mut self, cands: Vec<String>) {
        self.diags.last_mut().unwrap().fixes.push(Fix { key: "choose".into(), edit: Edit::Choose { candidates: cands } });
    }

    pub fn len(&mut self, p: &Prop, space_tokens: bool) -> Option<Len> {
        match &p.value {
            Atom::Num(n) => match dec_to_len(n) {
                Some(l) => Some(l),
                None => {
                    self.schema(p.span, "schema.bad-length", &p.name, n, &[]);
                    None
                }
            },
            Atom::Word(w) if space_tokens => match theme::space(w) {
                Some(l) => Some(l),
                None => {
                    let names: Vec<&str> = theme::SPACE.iter().map(|(n, _)| *n).collect();
                    self.span_diag(
                        Diagnostic::new(E_UNKNOWN_TOKEN, "token.unknown-space")
                            .at_span(p.span)
                            .param("prop", &p.name)
                            .param("found", w.clone())
                            .fix("choose", Edit::Choose { candidates: nearest(w, &names, 3) }),
                    );
                    None
                }
            },
            other => {
                self.schema(p.span, "schema.expected-length", &p.name, &describe_atom(other), &[]);
                None
            }
        }
    }

    pub fn radius(&mut self, p: &Prop) -> Option<Len> {
        match &p.value {
            Atom::Word(w) => match theme::radius(w) {
                Some(l) => Some(l),
                None => {
                    let names: Vec<&str> = theme::RADIUS.iter().map(|(n, _)| *n).collect();
                    self.span_diag(
                        Diagnostic::new(E_UNKNOWN_TOKEN, "token.unknown-radius")
                            .at_span(p.span)
                            .param("found", w.clone())
                            .fix("choose", Edit::Choose { candidates: nearest(w, &names, 3) }),
                    );
                    None
                }
            },
            _ => self.len(p, false),
        }
    }

    pub fn size(&mut self, p: &Prop) -> Option<Size> {
        match &p.value {
            Atom::Word(w) if w == "fill" => Some(Size::Fill),
            Atom::Word(w) if w == "hug" => Some(Size::Hug),
            Atom::Num(_) => self.len(p, false).map(Size::Px),
            other => {
                self.schema(p.span, "schema.expected-size", &p.name, &describe_atom(other), &[]);
                None
            }
        }
    }

    pub fn color_word(&mut self, w: &str, span: Span) -> Option<Color> {
        if let Some(h) = w.strip_prefix('#') {
            let ok = (h.len() == 6 || h.len() == 8) && h.bytes().all(|b| b.is_ascii_hexdigit());
            if !ok {
                self.schema(span, "schema.bad-color", "color", w, &[]);
                return None;
            }
            let n = u32::from_str_radix(h, 16).unwrap();
            return Some(if h.len() == 6 {
                Color::Rgba((n >> 16) as u8, (n >> 8) as u8, n as u8, 255)
            } else {
                Color::Rgba((n >> 24) as u8, (n >> 16) as u8, (n >> 8) as u8, n as u8)
            });
        }
        if self.theme.colors.iter().any(|(n, _)| n == w) {
            return Some(Color::Token(w.to_string()));
        }
        let names: Vec<&str> = self.theme.colors.iter().map(|(n, _)| n.as_str()).collect();
        let cands = nearest(w, &names, 3);
        self.span_diag(
            Diagnostic::new(E_UNKNOWN_TOKEN, "token.unknown-color")
                .at_span(span)
                .param("found", w)
                .fix("choose", Edit::Choose { candidates: cands }),
        );
        None
    }

    pub fn color(&mut self, p: &Prop) -> Option<Color> {
        match &p.value {
            Atom::Word(w) => self.color_word(w, p.span),
            other => {
                self.schema(p.span, "schema.expected-color", &p.name, &describe_atom(other), &[]);
                None
            }
        }
    }

    pub fn border(&mut self, p: &Prop) -> Option<Option<Border>> {
        match &p.value {
            Atom::Word(w) => match w.as_str() {
                "none" => Some(None),
                "soft" => Some(Some(Border { width: Len::from_px(1), color: tok("border-soft") })),
                "strong" => Some(Some(Border { width: Len::from_px(1), color: tok("border-strong") })),
                "normal" => Some(Some(Border { width: Len::from_px(1), color: tok("border") })),
                _ => {
                    self.schema(p.span, "schema.bad-border", &p.name, w, &[]);
                    self.choose_fix(vec!["none".into(), "soft".into(), "normal".into(), "strong".into()]);
                    None
                }
            },
            other => {
                self.schema(p.span, "schema.expected-border", &p.name, &describe_atom(other), &[]);
                None
            }
        }
    }

    pub fn expr(&mut self, p: &Prop) -> Option<Expr> {
        let e = match &p.value {
            Atom::Expr(e) => e.clone(),
            Atom::Word(w) if w == "true" => Expr::Bool(true),
            Atom::Word(w) if w == "false" => Expr::Bool(false),
            Atom::Word(w) => Expr::Var(w.clone()),
            Atom::Num(n) => match parse_num_expr(n, p.span) {
                Ok(e) => e,
                Err(d) => {
                    self.span_diag(d);
                    return None;
                }
            },
            other => {
                self.schema(p.span, "schema.expected-expression", &p.name, &describe_atom(other), &[]);
                return None;
            }
        };
        Some(self.resolve(e))
    }

    pub fn word(&mut self, p: &Prop, allowed: &[&str]) -> Option<String> {
        match &p.value {
            Atom::Word(w) if allowed.contains(&w.as_str()) => Some(w.clone()),
            other => {
                self.schema(p.span, "schema.expected-choice", &p.name, &describe_atom(other), &[]);
                self.choose_fix(allowed.iter().map(|s| s.to_string()).collect());
                None
            }
        }
    }

    pub fn ident_prop(&mut self, p: &Prop) -> Option<String> {
        match &p.value {
            Atom::Word(w) => Some(w.clone()),
            other => {
                self.schema(p.span, "schema.expected-name", &p.name, &describe_atom(other), &[]);
                None
            }
        }
    }

    /// A display string slot: literal -> new inline key in the default locale; `@key` -> that key.
    pub fn strref(&mut self, atom: &Atom, id: &str, slot: &str) -> Option<String> {
        match atom {
            Atom::Str(s) => {
                let key = format!("{}:{}", id, slot);
                self.inline.push((key.clone(), nfc(s)));
                Some(key)
            }
            Atom::Key(k) => Some(k.clone()),
            other => {
                self.schema(Span::default(), "schema.expected-string", slot, &describe_atom(other), &[]);
                None
            }
        }
    }

    pub fn strprop(&mut self, p: &Prop, id: &str, slot: &str) -> Option<String> {
        match &p.value {
            Atom::Str(_) | Atom::Key(_) => {
                let v = p.value.clone();
                self.strref(&v, id, slot)
            }
            other => {
                self.schema(p.span, "schema.expected-string", &p.name, &describe_atom(other), &[]);
                None
            }
        }
    }

    /// Positional label argument.
    pub fn label_arg(&mut self, pr: &mut Pr, id: &str, slot: &str, span: Span, required: bool) -> Option<String> {
        match pr.arg(0) {
            Some((a, _)) => {
                let a = a.clone();
                match &a {
                    Atom::Str(_) | Atom::Key(_) => self.strref(&a, id, slot),
                    other => {
                        self.schema(span, "schema.expected-string", slot, &describe_atom(other), &[]);
                        None
                    }
                }
            }
            None => {
                if required {
                    self.schema(span, "schema.missing-label", slot, "", &[]);
                }
                None
            }
        }
    }

    pub fn handler_prop(&mut self, p: &Prop, h: &mut Vec<Handler>) {
        let ev_name = p.name.strip_prefix("on.").unwrap_or(&p.name);
        let event = match Event::from_name(ev_name) {
            Some(e) => e,
            None => {
                self.span_diag(
                    Diagnostic::new(E_SCHEMA, "schema.unknown-event")
                        .at_span(p.span)
                        .param("found", ev_name)
                        .fix("choose", Edit::Choose { candidates: Event::names().iter().map(|s| s.to_string()).collect() }),
                );
                return;
            }
        };
        match &p.value {
            Atom::Word(effect) => h.push(Handler { event, guard: None, actions: vec![Action::Emit(effect.clone())] }),
            Atom::Handler(stmts) => {
                let mut guard: Option<Expr> = None;
                let mut actions = Vec::new();
                for s in stmts {
                    match s {
                        Stmt::When(e, _) => {
                            let e = self.resolve(e.clone());
                            guard = Some(match guard.take() {
                                None => e,
                                Some(g) => Expr::And(Box::new(g), Box::new(e)),
                            });
                        }
                        Stmt::Assign(n, e, _) => actions.push(Action::Assign(n.clone(), self.resolve(e.clone()))),
                        Stmt::Emit(n, _) => actions.push(Action::Emit(n.clone())),
                        Stmt::Remove(n, _) => actions.push(Action::Remove(n.clone())),
                        Stmt::Append(l, fs, _) => actions.push(Action::Append(
                            l.clone(),
                            fs.iter().map(|(f, e)| (f.clone(), self.resolve(e.clone()))).collect(),
                        )),
                    }
                }
                h.push(Handler { event, guard, actions });
            }
            other => self.schema(p.span, "schema.expected-handler", &p.name, &describe_atom(other), &[]),
        }
    }

    pub fn finish(&mut self, pr: Pr, allowed_props: &[&str], allowed_flags: &[&str], max_args: usize) {
        let raw = pr.raw;
        let mut fields: Vec<&str> = allowed_props.to_vec();
        fields.extend(allowed_flags.iter().copied());
        fields.sort();
        let unknown: Vec<(Span, String)> = raw
            .props
            .iter()
            .enumerate()
            .filter(|(i, _)| !pr.used_props[*i])
            .map(|(_, p)| (p.span, p.name.clone()))
            .chain(
                raw.flags.iter().enumerate().filter(|(i, _)| !pr.used_flags[*i]).map(|(_, (f, sp))| (*sp, f.clone())),
            )
            .collect();
        for (sp, name) in unknown {
            let cands = nearest(&name, &fields, 3);
            self.span_diag(
                Diagnostic::new(E_SCHEMA, "schema.unknown-field")
                    .at_span(sp)
                    .param("kind", raw.kind.clone())
                    .param("prop", name)
                    .fix("choose", Edit::Choose { candidates: cands })
                    .fix("fields", Edit::FieldList { fields: fields.iter().map(|s| s.to_string()).collect() }),
            );
        }
        let limit = max_args.max(pr.used_args);
        if raw.args.len() > limit {
            self.span_diag(
                Diagnostic::new(E_SCHEMA, "schema.extra-argument").at_span(raw.args[limit].1).param("kind", raw.kind.clone()),
            );
        }
    }
}

// ======================================================================== node kinds

impl Ex {
    fn text_node(&mut self, id: &str, key: String, st: TStyle, span: Span) -> Node {
        let n = mk_node(
            id,
            NodeKind::Text(TextProps {
                key,
                family: "ui".into(),
                size: st.size,
                weight: st.weight,
                line_height: line_height(st.size),
                color: st.color,
                align: TextAlign::Start,
                args: Vec::new(),
                value: None,
            }),
            Role::Label,
        );
        self.prov.push((id.to_string(), span));
        n
    }

    fn std_text(&mut self, id: &str, key: String, size: i32, weight: u16, color: &str, span: Span) -> Node {
        self.text_node(id, key, TStyle { size: Len::from_px(size), weight, color: tok(color) }, span)
    }

    fn child_id(&mut self, parent: &str, kind: &str, counters: &mut BTreeMap<String, u32>, explicit: Option<String>) -> Id {
        if let Some(e) = explicit {
            return e;
        }
        let c = counters.entry(kind.to_string()).or_insert(0);
        let n = *c;
        *c += 1;
        if parent.is_empty() {
            format!("{}{}", kind, n)
        } else {
            format!("{}/{}{}", parent, kind, n)
        }
    }

    pub(crate) fn children(&mut self, raws: &[RawNode], parent: &str) -> Vec<Node> {
        let mut counters = BTreeMap::new();
        let mut out: Vec<Node> = Vec::new();
        for r in raws {
            if let Some(n) = self.node(r, parent, &mut counters) {
                out.push(n);
            }
        }
        // avatar -> name from the first following sibling text
        for i in 0..out.len() {
            if self.avatar_pending.contains(&out[i].id) {
                let mut found = None;
                for sib in out.iter().skip(i + 1) {
                    if let Some(t) = first_text(sib) {
                        found = Some(t);
                        break;
                    }
                }
                if let Some(t) = found {
                    out[i].sem.name = Some(Name::FromNode(t));
                    let id = out[i].id.clone();
                    self.avatar_pending.retain(|x| *x != id);
                }
            }
        }
        out
    }

    pub(crate) fn node(&mut self, raw: &RawNode, parent: &str, counters: &mut BTreeMap<String, u32>) -> Option<Node> {
        let kind_norm = match raw.kind.as_str() {
            "column" => "col",
            k => k,
        };
        let known = CORE_KINDS.contains(&raw.kind.as_str()) || STD_KINDS.contains(&raw.kind.as_str());
        if !known {
            let all: Vec<&str> = CORE_KINDS.iter().chain(STD_KINDS.iter()).copied().collect();
            self.span_diag(
                Diagnostic::new(E_UNKNOWN_KIND, "kind.unknown")
                    .at_span(raw.span)
                    .param("kind", raw.kind.clone())
                    .fix("choose", Edit::Choose { candidates: nearest(&raw.kind, &all, 3) }),
            );
            return None;
        }
        let mut pr = Pr::new(raw);
        let explicit = pr.prop("id").and_then(|p| self.ident_prop(p));
        let id = self.child_id(parent, kind_norm, counters, explicit);
        self.prov.push((id.clone(), raw.span));
        match raw.kind.as_str() {
            "surface" => self.k_box(raw, pr, &id, Mode::Surface),
            "box" | "row" | "col" | "column" => self.k_box(raw, pr, &id, Mode::Box),
            "grid" => self.k_box(raw, pr, &id, Mode::Grid),
            "list" => self.k_box(raw, pr, &id, Mode::List),
            "item" => self.k_box(raw, pr, &id, Mode::Item),
            "text" => self.k_text(raw, pr, &id),
            "text-input" => self.k_input(raw, pr, &id),
            "card" => self.k_card(raw, pr, &id),
            "avatar" => self.k_avatar(raw, pr, &id),
            "button" => self.k_button(raw, pr, &id),
            "link" => self.k_link(raw, pr, &id),
            "checkbox" | "switch" => self.k_checkbox(raw, pr, &id),
            "radio" => self.k_radio(raw, pr, &id),
            "tab" => self.k_tab(raw, pr, &id),
            "field" | "password-field" => self.k_field(raw, pr, &id),
            "form" => self.k_form(raw, pr, &id),
            "shape" => self.k_shape(raw, pr, &id),
            "layer" => self.k_layer(raw, pr, &id),
            "slot" => self.k_slot(raw, pr, &id),
            "view" => self.k_view(raw, pr, &id),
            "case" => {
                self.span_diag(Diagnostic::new(E_SCHEMA, "schema.case-outside-slot").at_span(raw.span));
                None
            }
            _ => unreachable!(),
        }
    }

    /// Props shared by every node: role/name/desc/live/disabled/show/on.*/decorative.
    fn common(&mut self, pr: &mut Pr, n: &mut Node, id: &str, with_role: bool) {
        if with_role {
            if let Some(p) = pr.prop("role") {
                if let Atom::Word(w) = &p.value {
                    match Role::from_name(w) {
                        Some(r) => n.sem.role = r,
                        None => {
                            self.span_diag(
                                Diagnostic::new(E_SCHEMA, "schema.unknown-role")
                                    .at_span(p.span)
                                    .param("found", w.clone())
                                    .fix("choose", Edit::Choose { candidates: nearest(w, Role::names(), 3) }),
                            );
                        }
                    }
                } else {
                    self.schema(p.span, "schema.expected-name", "role", &describe_atom(&p.value), &[]);
                }
            }
        }
        if let Some(p) = pr.prop("name") {
            if let Some(k) = self.strprop(p, id, "name") {
                n.sem.name = Some(Name::Key(k));
            }
        }
        if let Some(p) = pr.prop("desc") {
            n.sem.description = self.strprop(p, id, "desc");
        }
        if let Some(p) = pr.prop("live") {
            if let Some(w) = self.word(p, Live::names()) {
                n.sem.live = Live::from_name(&w).unwrap();
            }
        }
        if pr.flag("decorative") {
            n.sem.decorative = true;
        }
        if let Some(p) = pr.prop("disabled") {
            n.sem.disabled = self.expr(p);
        }
        if let Some(p) = pr.prop("show") {
            n.show = self.expr(p);
        }
        self.handlers(pr, &mut n.on);
        self.at_props(pr, n);
    }

    /// `md.<prop>=…` / `sm.<prop>=…`: breakpoint overrides (width <= 768 / <= 480; `sm` wins).
    pub(crate) fn at_props(&mut self, pr: &mut Pr, n: &mut Node) {
        let props: Vec<&Prop> = pr.raw.props.iter().filter(|p| p.name.starts_with("md.") || p.name.starts_with("sm.")).collect();
        for bp in [Bp::Md, Bp::Sm] {
            let mut rule = AtRule { bp, patch: LayoutPatch::default(), show: None };
            let mut any = false;
            for p in props.iter().filter(|p| p.name.starts_with(bp.as_str())) {
                pr.prop(&p.name);
                any = true;
                let field = &p.name[3..];
                let sub = Prop { name: field.to_string(), value: p.value.clone(), span: p.span };
                match field {
                    "dir" => {
                        if let Some(w) = self.word(&sub, &["row", "col", "grid"]) {
                            rule.patch.dir = Dir::from_name(&w);
                        }
                    }
                    "cols" => match &p.value {
                        Atom::Num(c) if c.parse::<u16>().map(|x| x >= 1 && x <= 12).unwrap_or(false) => {
                            rule.patch.cols = c.parse().ok();
                        }
                        other => self.schema(p.span, "schema.bad-cols", &p.name, &describe_atom(other), &[]),
                    },
                    "gap" => rule.patch.gap = self.len(&sub, true),
                    "pad" => rule.patch.pad = self.pad_value(&sub),
                    "align" => {
                        if let Some(w) = self.word(&sub, Align::names()) {
                            rule.patch.align = Align::from_name(&w);
                        }
                    }
                    "justify" => {
                        if let Some(w) = self.word(&sub, Justify::names()) {
                            rule.patch.justify = Justify::from_name(&w);
                        }
                    }
                    "w" => rule.patch.w = self.size(&sub),
                    "h" => rule.patch.h = self.size(&sub),
                    "show" => {
                        if let Some(w) = self.word(&sub, &["true", "false"]) {
                            rule.show = Some(w == "true");
                        }
                    }
                    _ => {
                        let f = ["dir", "cols", "gap", "pad", "align", "justify", "w", "h", "show"];
                        self.span_diag(
                            Diagnostic::new(E_SCHEMA, "schema.unknown-field")
                                .at_span(p.span)
                                .param("kind", "breakpoint-override")
                                .param("prop", p.name.clone())
                                .fix("fields", Edit::FieldList { fields: f.iter().map(|x| format!("{}.{}", bp.as_str(), x)).collect() }),
                        );
                    }
                }
            }
            if any {
                n.at.push(rule);
            }
        }
    }

    pub(crate) fn pad_value(&mut self, p: &Prop) -> Option<Edges> {
        match &p.value {
            Atom::List(items) if items.len() == 2 || items.len() == 4 => {
                let mut ls = Vec::new();
                for (a, sp) in items {
                    let pp = Prop { name: "pad".into(), value: a.clone(), span: *sp };
                    ls.push(self.len(&pp, true)?);
                }
                Some(if ls.len() == 2 {
                    Edges::xy(ls[1], ls[0]) // [vertical, horizontal]
                } else {
                    Edges { top: ls[0], right: ls[1], bottom: ls[2], left: ls[3] }
                })
            }
            _ => self.len(p, true).map(Edges::all),
        }
    }

    fn handlers(&mut self, pr: &mut Pr, out: &mut Vec<Handler>) {
        let hs: Vec<&Prop> = pr.raw.props.iter().filter(|p| p.name.starts_with("on.")).collect();
        for p in hs {
            pr.prop(&p.name);
            self.handler_prop(p, out);
        }
    }

    fn req_bind(&mut self, pr: &mut Pr, raw: &RawNode) -> String {
        match pr.prop("bind") {
            Some(p) => self.ident_prop(p).unwrap_or_default(),
            None => {
                self.schema(raw.span, "schema.missing-field", "bind", "", &["bind"]);
                String::new()
            }
        }
    }

    fn box_layout(&mut self, pr: &mut Pr, n: &mut Node) {
        if let Some(p) = pr.prop("w") {
            if let Some(s) = self.size(p) {
                n.layout.w = s;
            }
        }
        if let Some(p) = pr.prop("h") {
            if let Some(s) = self.size(p) {
                n.layout.h = s;
            }
        }
        if let Some(p) = pr.prop("pad") {
            if let Some(e) = self.pad_value(p) {
                n.layout.pad = e;
            }
        }
        if let Some(p) = pr.prop("gap") {
            if let Some(l) = self.len(p, true) {
                n.layout.gap = l;
            }
        }
        if let Some(p) = pr.prop("align") {
            if let Some(w) = self.word(p, Align::names()) {
                n.layout.align = Align::from_name(&w).unwrap();
            }
        }
        if let Some(p) = pr.prop("justify") {
            if let Some(w) = self.word(p, Justify::names()) {
                n.layout.justify = Justify::from_name(&w).unwrap();
            }
        }
    }

    fn box_paint(&mut self, pr: &mut Pr, n: &mut Node) {
        if let Some(p) = pr.prop("fill") {
            if matches!(&p.value, Atom::Word(w) if w == "none") {
                n.paint.fill = None;
            } else {
                n.paint.fill = self.color(p);
            }
        }
        if let Some(p) = pr.prop("border") {
            if let Some(b) = self.border(p) {
                n.paint.border = b;
            }
        }
        if let Some(p) = pr.prop("radius") {
            if matches!(&p.value, Atom::Word(w) if w == "full" || w == "circle") {
                n.paint.shape = Shape::Circle;
            } else if let Some(l) = self.radius(p) {
                n.paint.shape = Shape::RoundedRect(l);
            }
        }
    }

    const BOX_PROPS: &'static [&'static str] = &[
        "id", "role", "name", "desc", "live", "disabled", "show", "on.press", "on.change", "on.submit",
        "w", "h", "pad", "gap", "align", "justify", "fill", "border", "radius",
    ];

    fn k_box(&mut self, raw: &RawNode, mut pr: Pr, id: &str, mode: Mode) -> Option<Node> {
        let surface = mode == Mode::Surface;
        let role = match mode {
            Mode::List => Role::List,
            Mode::Item => Role::ListItem,
            _ => Role::Container,
        };
        let mut n = mk_node(id, if surface { NodeKind::Surface } else { NodeKind::Box }, role);
        n.layout.dir = match (mode, raw.kind.as_str()) {
            (Mode::Grid, _) => Dir::Grid,
            (_, "row") => Dir::Row,
            _ => Dir::Col,
        };
        if surface {
            n.layout.pad = Edges::all(Len::from_px(24));
            n.layout.align = Align::Center;
            n.paint.fill = Some(tok("bg"));
        }
        if mode == Mode::Grid {
            n.layout.cols = 1;
            match pr.prop("cols") {
                Some(p) => match &p.value {
                    Atom::Num(c) if c.parse::<u16>().map(|x| (1..=12).contains(&x)).unwrap_or(false) => {
                        n.layout.cols = c.parse().unwrap();
                    }
                    other => self.schema(p.span, "schema.bad-cols", "cols", &describe_atom(other), &[]),
                },
                None => self.schema(raw.span, "schema.missing-field", "cols", "", &["cols"]),
            }
        }
        self.common(&mut pr, &mut n, id, true);
        self.box_layout(&mut pr, &mut n);
        self.box_paint(&mut pr, &mut n);
        if surface {
            n.layout.w = Size::Fill;
            n.layout.h = Size::Fill;
        }
        n.children = self.children(&raw.children, id);
        let mut props: Vec<&str> = Self::BOX_PROPS.to_vec();
        if mode == Mode::Grid {
            props.push("cols");
        }
        self.finish(pr, &props, &["decorative"], 0);
        Some(n)
    }

    fn k_text(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let value = pr.prop("value").and_then(|p| self.expr(p));
        let key = if value.is_some() {
            if raw.args.first().is_some() {
                self.schema(raw.span, "schema.value-and-label", "value", "", &[]);
            }
            String::new()
        } else {
            self.label_arg(&mut pr, id, "t", raw.span, true).unwrap_or_default()
        };
        // any other property is a named hole of the string: `text @money v=(subtotal)`
        const TEXT_FIXED: &[&str] =
            &["id", "size", "color", "align", "heading", "w", "role", "name", "desc", "live", "disabled", "show", "value"];
        let mut args = Vec::new();
        let arg_props: Vec<&Prop> = pr
            .raw
            .props
            .iter()
            .filter(|p| {
                !TEXT_FIXED.contains(&p.name.as_str())
                    && !p.name.starts_with("on.")
                    && !p.name.starts_with("md.")
                    && !p.name.starts_with("sm.")
            })
            .collect();
        for p in arg_props {
            pr.prop(&p.name);
            if let Some(e) = self.expr(p) {
                args.push((p.name.clone(), e));
            }
        }
        let bold = pr.flag("bold");
        let muted = pr.flag("muted");
        let body = pr.flag("body");
        let mut size = Len::from_px(14);
        if let Some(p) = pr.prop("size") {
            if let Some(l) = self.len(p, false) {
                size = l;
            }
        }
        let mut color = tok(if muted { "muted" } else if body { "body" } else { "fg" });
        if let Some(p) = pr.prop("color") {
            if let Some(c) = self.color(p) {
                color = c;
            }
        }
        let mut n = self.text_node(id, key, TStyle { size, weight: if bold { 700 } else { 400 }, color }, raw.span);
        if let NodeKind::Text(t) = &mut n.kind {
            t.args = args;
            t.value = value;
        }
        if let Some(p) = pr.prop("align") {
            if let Some(w) = self.word(p, TextAlign::names()) {
                if let NodeKind::Text(t) = &mut n.kind {
                    t.align = TextAlign::from_name(&w).unwrap();
                }
            }
        }
        if let Some(p) = pr.prop("heading") {
            match &p.value {
                Atom::Num(l) if matches!(l.as_str(), "1" | "2" | "3" | "4" | "5" | "6") => {
                    n.sem.role = Role::Heading;
                    n.sem.heading_level = Some(l.parse().unwrap());
                }
                other => self.schema(p.span, "schema.bad-heading-level", "heading", &describe_atom(other), &[]),
            }
        }
        if let Some(p) = pr.prop("w") {
            if let Some(s) = self.size(p) {
                n.layout.w = s;
            }
        }
        self.common(&mut pr, &mut n, id, true);
        if n.sem.role == Role::Heading && n.sem.heading_level.is_none() {
            n.sem.heading_level = Some(2);
        }
        self.finish(
            pr,
            &["id", "size", "color", "align", "heading", "w", "role", "name", "desc", "live", "disabled", "show", "value", "<hole>=(expr)"],
            &["bold", "muted", "body", "decorative"],
            1,
        );
        Some(n)
    }

    fn input_node(&mut self, id: &str, variant: InputVariant, bind: String, masked: Expr, span: Span) -> Node {
        let role = match variant {
            InputVariant::Single => Role::TextField,
            InputVariant::Email => Role::EmailField,
            InputVariant::Password => Role::PasswordField,
            InputVariant::Search => Role::SearchField,
            InputVariant::Multiline => Role::TextArea,
        };
        let size = Len::from_px(14);
        let mut n = mk_node(
            id,
            NodeKind::TextInput(InputProps {
                variant,
                bind,
                masked,
                family: "ui".into(),
                size,
                line_height: line_height(size),
                color: tok("fg"),
            }),
            role,
        );
        n.layout.w = Size::Fill;
        n.layout.pad = Edges::xy(Len::from_px(12), Len::from_px(8));
        n.paint = Paint {
            fill: Some(tok("input-bg")),
            border: Some(Border { width: Len::from_px(1), color: tok("border") }),
            shape: Shape::RoundedRect(Len::from_px(8)),
        };
        self.prov.push((id.to_string(), span));
        n
    }

    fn k_input(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let mut variant = InputVariant::Single;
        if let Some(p) = pr.prop("variant") {
            if let Some(w) = self.word(p, InputVariant::names()) {
                variant = InputVariant::from_name(&w).unwrap();
            }
        }
        let bind = self.req_bind(&mut pr, raw);
        let mut masked = Expr::Bool(variant == InputVariant::Password);
        if let Some(p) = pr.prop("masked") {
            if let Some(e) = self.expr(p) {
                masked = e;
            }
        }
        let mut n = self.input_node(id, variant, bind, masked, raw.span);
        if pr.flag("required") {
            n.sem.required = true;
        }
        self.common(&mut pr, &mut n, id, false);
        self.box_layout(&mut pr, &mut n);
        self.finish(
            pr,
            &["id", "bind", "variant", "masked", "name", "desc", "live", "disabled", "show", "on.change", "on.submit", "w", "h", "pad", "gap", "align", "justify"],
            &["required", "decorative"],
            0,
        );
        Some(n)
    }

    fn k_card(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let mut n = mk_node(id, NodeKind::Box, Role::Container);
        n.layout.pad = Edges::all(Len::from_px(16));
        n.layout.gap = Len::from_px(16);
        n.paint = Paint { fill: Some(tok("surface")), border: None, shape: Shape::RoundedRect(Len::from_px(12)) };
        self.common(&mut pr, &mut n, id, true);
        self.box_layout(&mut pr, &mut n);
        self.box_paint(&mut pr, &mut n);
        n.children = self.children(&raw.children, id);
        self.finish(pr, Self::BOX_PROPS, &["decorative"], 0);
        Some(n)
    }

    fn fill_on(&self, c: &Color) -> Color {
        let rgba = match c {
            Color::Rgba(r, g, b, a) => [*r, *g, *b, *a],
            Color::Token(t) => self.theme.colors.iter().find(|(n, _)| n == t).map(|(_, v)| *v).unwrap_or([0, 0, 0, 255]),
        };
        tok(theme::best_on_token(rgba, &self.theme))
    }

    fn k_avatar(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let key = self.label_arg(&mut pr, id, "initials", raw.span, true).unwrap_or_default();
        let mut size = Len::from_px(40);
        if let Some(p) = pr.prop("size") {
            if let Some(l) = self.len(p, false) {
                size = l;
            }
        }
        let mut fill = tok("blue");
        if let Some(p) = pr.prop("fill") {
            if let Some(c) = self.color(p) {
                fill = c;
            }
        }
        let mut n = mk_node(id, NodeKind::Box, Role::Image);
        n.layout.dir = Dir::Row;
        n.layout.w = Size::Px(size);
        n.layout.h = Size::Px(size);
        n.layout.align = Align::Center;
        n.layout.justify = Justify::Center;
        n.paint = Paint { fill: Some(fill.clone()), border: None, shape: Shape::Circle };
        if pr.flag("decorative") {
            n.sem.decorative = true;
        }
        let mut has_alt = false;
        if let Some(p) = pr.prop("alt") {
            if let Some(k) = self.strprop(p, id, "alt") {
                n.sem.name = Some(Name::Key(k));
                has_alt = true;
            }
        }
        if let Some(p) = pr.prop("show") {
            n.show = self.expr(p);
        }
        // initials: 40% of the avatar size, bold, best-contrast ink on the fill
        let isize = Len(((size.0 as i64 * 2 + 2) / 5) as i32);
        let lid = format!("{}/label", id);
        let on = self.fill_on(&fill);
        let mut t = self.text_node(&lid, key, TStyle { size: isize, weight: 700, color: on }, raw.span);
        t.sem.decorative = true;
        n.children.push(t);
        if !has_alt && !n.sem.decorative {
            self.avatar_pending.push(id.to_string());
        }
        self.at_props(&mut pr, &mut n);
        self.finish(pr, &["id", "size", "fill", "alt", "show"], &["decorative"], 1);
        Some(n)
    }

    fn button_node(&mut self, id: &str, label_key: String, fill: Option<Color>, span: Span) -> Node {
        let mut n = mk_node(id, NodeKind::Box, Role::Button);
        n.layout.dir = Dir::Row;
        n.layout.pad = Edges::xy(Len::from_px(16), Len::from_px(10));
        n.layout.align = Align::Center;
        n.layout.justify = Justify::Center;
        let (border, ink) = match &fill {
            Some(f) => (None, self.fill_on(f)),
            None => (Some(Border { width: Len::from_px(1), color: tok("border") }), tok("fg")),
        };
        n.paint = Paint { fill, border, shape: Shape::RoundedRect(Len::from_px(8)) };
        n.sem.name = Some(Name::Key(label_key.clone()));
        let lid = format!("{}/label", id);
        let mut t = self.text_node(&lid, label_key, TStyle { size: Len::from_px(14), weight: 400, color: ink }, span);
        t.sem.decorative = true;
        n.children.push(t);
        self.prov.push((id.to_string(), span));
        n
    }

    fn k_button(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let label = self.label_arg(&mut pr, id, "label", raw.span, true).unwrap_or_default();
        let mut fill = Some(tok("accent"));
        if let Some(p) = pr.prop("fill") {
            if matches!(&p.value, Atom::Word(w) if w == "none") {
                fill = None; // secondary look: 1 px border, no fill
            } else if let Some(c) = self.color(p) {
                fill = Some(c);
            }
        }
        let mut n = self.button_node(id, label, fill, raw.span);
        if let Some(p) = pr.prop("w") {
            if let Some(s) = self.size(p) {
                n.layout.w = s;
            }
        }
        if let Some(p) = pr.prop("name") {
            if let Some(k) = self.strprop(p, id, "name") {
                n.sem.name = Some(Name::Key(k));
            }
        }
        if let Some(p) = pr.prop("disabled") {
            n.sem.disabled = self.expr(p);
        }
        if let Some(p) = pr.prop("show") {
            n.show = self.expr(p);
        }
        let mut hs = Vec::new();
        self.handlers(&mut pr, &mut hs);
        n.on = hs;
        if pr.flag("submit") {
            match self.form_stack.last().cloned() {
                Some(f) => n.on.push(Handler { event: Event::Press, guard: None, actions: vec![Action::Submit(f)] }),
                None => self.schema(raw.span, "schema.submit-outside-form", "submit", "", &[]),
            }
        }
        self.at_props(&mut pr, &mut n);
        self.finish(pr, &["id", "fill", "w", "name", "disabled", "show", "on.press"], &["submit"], 1);
        Some(n)
    }

    fn k_checkbox(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let is_switch = raw.kind == "switch";
        // label (visible text inside the control) or `name=` (bare control, e.g. in a settings row)
        let label = self.label_arg(&mut pr, id, "label", raw.span, false);
        let bind = self.req_bind(&mut pr, raw);
        let mut n = mk_node(id, NodeKind::Box, if is_switch { Role::Switch } else { Role::Checkbox });
        n.layout.dir = Dir::Row;
        n.layout.gap = Len::from_px(8);
        n.layout.align = Align::Center;
        n.layout.pad = Edges::xy(Len::ZERO, Len::from_px(2));
        let explicit_name = pr.prop("name").and_then(|p| self.strprop(p, id, "name"));
        match (&label, &explicit_name) {
            (Some(l), None) => n.sem.name = Some(Name::Key(l.clone())),
            (_, Some(k)) => n.sem.name = Some(Name::Key(k.clone())),
            (None, None) => self.schema(raw.span, "schema.missing-label", "label", "", &["name"]),
        }
        n.sem.checked = Some(Expr::Var(bind.clone()));
        n.on.push(Handler {
            event: Event::Press,
            guard: None,
            actions: vec![Action::Assign(bind.clone(), Expr::Not(Box::new(Expr::Var(bind.clone()))))],
        });
        let bid = format!("{}/box", id);
        let mut b = mk_node(&bid, NodeKind::Box, Role::Container);
        b.sem.decorative = true;
        b.layout.dir = Dir::Row;
        b.layout.align = Align::Center;
        b.paint = Paint {
            fill: Some(tok("input-bg")),
            border: Some(Border { width: Len::from_px(1), color: tok("border") }),
            shape: Shape::RoundedRect(Len::from_px(4)),
        };
        if is_switch {
            // 40x22 track; the knob sits left when off, right when on (two knobs + a spacer, no conditional paint)
            b.layout.w = Size::Px(Len::from_px(40));
            b.layout.h = Size::Px(Len::from_px(22));
            b.layout.pad = Edges::all(Len::from_px(2));
            b.paint.shape = Shape::RoundedRect(Len::from_px(11));
            let knob = |ex: &mut Ex, nid: String, fill: &str, show: Expr| {
                let mut k = mk_node(&nid, NodeKind::Shape, Role::Image);
                k.sem.decorative = true;
                k.layout.w = Size::Px(Len::from_px(16));
                k.layout.h = Size::Px(Len::from_px(16));
                k.paint = Paint { fill: Some(tok(fill)), border: None, shape: Shape::Circle };
                k.show = Some(show);
                ex.prov.push((nid, raw.span));
                k
            };
            let off = knob(self, format!("{}/knob-off", id), "muted", Expr::Not(Box::new(Expr::Var(bind.clone()))));
            let mut sp = mk_node(&format!("{}/spacer", id), NodeKind::Box, Role::Container);
            sp.sem.decorative = true;
            sp.layout.w = Size::Fill;
            sp.show = Some(Expr::Var(bind.clone()));
            self.prov.push((sp.id.clone(), raw.span));
            let on = knob(self, format!("{}/knob-on", id), "accent", Expr::Var(bind.clone()));
            b.children.extend([off, sp, on]);
        } else {
            b.layout.w = Size::Px(Len::from_px(20));
            b.layout.h = Size::Px(Len::from_px(20));
            b.layout.justify = Justify::Center;
            let mid = format!("{}/mark", id);
            let mut m = mk_node(&mid, NodeKind::Box, Role::Container);
            m.sem.decorative = true;
            m.layout.w = Size::Px(Len::from_px(12));
            m.layout.h = Size::Px(Len::from_px(12));
            m.paint = Paint { fill: Some(tok("accent")), border: None, shape: Shape::RoundedRect(Len::from_px(2)) };
            m.show = Some(Expr::Var(bind));
            self.prov.push((mid, raw.span));
            b.children.push(m);
        }
        self.prov.push((bid, raw.span));
        n.children.push(b);
        if let Some(l) = label {
            let lid = format!("{}/label", id);
            let mut t = self.std_text(&lid, l, 14, 400, "fg", raw.span);
            t.sem.decorative = true;
            n.children.push(t);
        }
        if let Some(p) = pr.prop("disabled") {
            n.sem.disabled = self.expr(p);
        }
        if let Some(p) = pr.prop("show") {
            n.show = self.expr(p);
        }
        let mut hs = Vec::new();
        self.handlers(&mut pr, &mut hs);
        n.on.extend(hs);
        self.at_props(&mut pr, &mut n);
        self.finish(pr, &["id", "bind", "name", "disabled", "show", "on.press"], &[], 1);
        Some(n)
    }

    fn k_form(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let mut n = mk_node(id, NodeKind::Box, Role::Form);
        n.layout.gap = Len::from_px(16);
        if let Some(k) = self.label_arg(&mut pr, id, "name", raw.span, false) {
            n.sem.name = Some(Name::Key(k));
        }
        self.common(&mut pr, &mut n, id, false);
        self.box_layout(&mut pr, &mut n);
        self.form_stack.push(id.to_string());
        n.children = self.children(&raw.children, id);
        self.form_stack.pop();
        self.finish(
            pr,
            &["id", "name", "desc", "live", "disabled", "show", "on.submit", "w", "h", "pad", "gap", "align", "justify"],
            &["decorative"],
            1,
        );
        Some(n)
    }

    fn k_field(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let is_pw = raw.kind == "password-field";
        let label = self.label_arg(&mut pr, id, "label", raw.span, true).unwrap_or_default();
        let bind = self.req_bind(&mut pr, raw);
        let mut variant = if is_pw { InputVariant::Password } else { InputVariant::Single };
        if !is_pw {
            if let Some(p) = pr.prop("kind") {
                if let Some(w) = self.word(p, &["text", "email", "search", "multiline"]) {
                    variant = match w.as_str() {
                        "email" => InputVariant::Email,
                        "search" => InputVariant::Search,
                        "multiline" => InputVariant::Multiline,
                        _ => InputVariant::Single,
                    };
                }
            }
        }
        let help = pr.prop("help").and_then(|p| self.strprop(p, id, "help"));
        let error = pr.prop("error").and_then(|p| self.strprop(p, id, "error"));
        let error_when = pr.prop("error-when").and_then(|p| self.expr(p));
        if error.is_some() != error_when.is_some() {
            self.schema(
                raw.span,
                "schema.error-needs-when",
                if error.is_some() { "error-when" } else { "error" },
                "",
                &["error", "error-when"],
            );
        }
        let required = pr.flag("required");
        let (toggle, toggle_name) = if is_pw {
            let t = pr.prop("toggle").and_then(|p| self.ident_prop(p));
            let tn = pr.prop("toggle-name").and_then(|p| self.strprop(p, id, "toggle-name"));
            if t.is_none() || tn.is_none() {
                self.schema(
                    raw.span,
                    "schema.missing-field",
                    if t.is_none() { "toggle" } else { "toggle-name" },
                    "",
                    &["toggle", "toggle-name"],
                );
            }
            (t, tn)
        } else {
            (None, None)
        };
        let show = pr.prop("show").and_then(|p| self.expr(p));
        let disabled = pr.prop("disabled").and_then(|p| self.expr(p));
        let mut on = Vec::new();
        self.handlers(&mut pr, &mut on);

        let mut col = mk_node(id, NodeKind::Box, Role::Container);
        col.layout.gap = Len::from_px(4);
        col.layout.w = Size::Fill;
        col.show = show;
        self.at_props(&mut pr, &mut col);
        self.prov.push((id.to_string(), raw.span));

        let lid = format!("{}/label", id);
        let label_node = self.std_text(&lid, label.clone(), 14, 700, "fg", raw.span);
        col.children.push(label_node);

        let iid = format!("{}/input", id);
        let masked = match &toggle {
            Some(t) => Expr::Not(Box::new(Expr::Var(t.clone()))),
            None => Expr::Bool(false),
        };
        let mut input = self.input_node(&iid, variant, bind, masked, raw.span);
        input.sem.name = Some(Name::Key(label));
        input.sem.labelled_by = Some(lid);
        input.sem.required = required;
        input.sem.disabled = disabled;
        input.sem.invalid = error_when.clone();
        input.on = on;
        let hid = format!("{}/help", id);
        let eid = format!("{}/error", id);
        if help.is_some() {
            input.sem.described_by.push(hid.clone());
        }
        if error.is_some() {
            input.sem.error_message = Some(eid.clone());
        }

        if let (true, Some(t), Some(tn)) = (is_pw, toggle.clone(), toggle_name) {
            let rid = format!("{}/row", id);
            let mut row = mk_node(&rid, NodeKind::Box, Role::Container);
            row.layout.dir = Dir::Row;
            row.layout.gap = Len::from_px(8);
            row.layout.align = Align::Center;
            row.layout.w = Size::Fill;
            self.prov.push((rid, raw.span));
            row.children.push(input);
            let tid = format!("{}/toggle", id);
            let mut b = self.button_node(&tid, tn, None, raw.span);
            b.layout.pad = Edges::xy(Len::from_px(12), Len::from_px(8));
            b.sem.pressed = Some(Expr::Var(t.clone()));
            b.on.push(Handler {
                event: Event::Press,
                guard: None,
                actions: vec![Action::Assign(t.clone(), Expr::Not(Box::new(Expr::Var(t))))],
            });
            if let NodeKind::Text(tp) = &mut b.children[0].kind {
                tp.size = Len::from_px(12);
                tp.line_height = line_height(tp.size);
            }
            row.children.push(b);
            col.children.push(row);
        } else {
            col.children.push(input);
        }
        if let Some(k) = help {
            let t = self.std_text(&hid, k, 12, 400, "muted", raw.span);
            col.children.push(t);
        }
        if let (Some(k), Some(w)) = (error, error_when) {
            let mut t = self.std_text(&eid, k, 12, 400, "error", raw.span);
            t.sem.role = Role::Status;
            t.sem.live = Live::Polite;
            t.show = Some(w);
            col.children.push(t);
        }
        let mut allowed: Vec<&str> =
            vec!["id", "bind", "help", "error", "error-when", "show", "disabled", "on.change", "on.submit"];
        if is_pw {
            allowed.push("toggle");
            allowed.push("toggle-name");
        } else {
            allowed.push("kind");
        }
        self.finish(pr, &allowed, &["required"], 1);
        Some(col)
    }
}

// ======================================================================== module level

fn parse_ver(s: &str) -> Option<(u16, u16)> {
    let (a, b) = s.split_once('.')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

fn atom_u64(p: &Prop) -> Option<u64> {
    match &p.value {
        Atom::Num(n) => n.parse().ok(),
        _ => None,
    }
}

fn unknown_field(span: Span, kind: &str, prop: &str, fields: &[&str]) -> Diagnostic {
    Diagnostic::new(E_SCHEMA, "schema.unknown-field")
        .at_span(span)
        .param("kind", kind)
        .param("prop", prop)
        .fix("fields", Edit::FieldList { fields: fields.iter().map(|s| s.to_string()).collect() })
}

/// The `std@0.1` default font set (`fonts std`): content hashes of the files dev-render bundles (`pvc-runtime/fonts/`; the CJK bold face is subset to the L01+F01 strings, G6 §3).
pub const STD_FONTS: &[(&str, u16, Option<&str>, &str, &str, u32)] = &[
    ("ui", 400, None, "DejaVuSans.ttf", "ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280", 0),
    ("ui", 700, None, "DejaVuSans-Bold.ttf", "5c1247acef7f2b8522a31742c76d6adcb5569bacc0be7ceaa4dc39dd252ce895", 0),
    ("ui", 400, Some("Hans"), "NotoSansCJKsc-Regular.otf", "6ead94f36c0f9423a88356f5a58b2b1b8180a3ff553ecab2f7a816990b306c60", 0),
    ("ui", 700, Some("Hans"), "NotoSansCJKsc-Bold.subset.otf", "a44805f531bccdd9eb28df4a5446132d88cf93326ed96b38366c45c2e4e3c5f9", 0),
];

fn expand_fonts(raw: &RawModule, diags: &mut Diags) -> Vec<FontEntry> {
    let mut fonts = Vec::new();
    if raw.fonts_std {
        for (family, weight, script, file, sha, index) in STD_FONTS {
            fonts.push(FontEntry {
                family: family.to_string(),
                weight: *weight,
                script: script.map(|s| s.to_string()),
                sha256: sha.to_string(),
                file: file.to_string(),
                index: *index,
            });
        }
    }
    for f in &raw.fonts {
        let mut weight = f.weight.parse::<u16>().unwrap_or(0);
        if weight == 0 || weight > 1000 {
            weight = 400;
            diags.push(Diagnostic::new(E_SCHEMA, "schema.bad-number").at_span(f.span).param("prop", "weight"));
        }
        let (mut sha, mut file, mut index, mut script) = (None, None, 0u32, None);
        for p in &f.props {
            match (p.name.as_str(), &p.value) {
                ("sha256", Atom::Word(w)) | ("sha256", Atom::Str(w)) | ("sha256", Atom::Num(w)) => sha = Some(w.to_lowercase()),
                ("file", Atom::Str(w)) => file = Some(w.clone()),
                ("index", Atom::Num(n)) => index = n.parse().unwrap_or(0),
                ("script", Atom::Word(w)) => script = Some(w.clone()),
                (n, _) => diags.push(unknown_field(p.span, "font", n, &["sha256", "file", "index", "script"])),
            }
        }
        match sha {
            Some(h) if h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()) => fonts.push(FontEntry {
                family: f.family.clone(),
                weight,
                script,
                sha256: h,
                file: file.unwrap_or_default(),
                index,
            }),
            _ => diags.push(
                Diagnostic::new(E_SCHEMA, "schema.missing-field")
                    .at_span(f.span)
                    .param("prop", "sha256")
                    .fix("fields", Edit::FieldList { fields: vec!["sha256".into()] }),
            ),
        }
    }
    fonts
}

fn expand_caps(raw: &RawModule, diags: &mut Diags) -> Vec<Capability> {
    let mut caps = Vec::new();
    for c in &raw.caps {
        let kind = match CAP_KINDS
            .iter()
            .filter(|k| c.id == **k || c.id.starts_with(&format!("{}.", k)))
            .max_by_key(|k| k.len())
        {
            Some(k) => k.to_string(),
            None => {
                diags.push(
                    Diagnostic::new(E_SCHEMA, "schema.unknown-capability-kind")
                        .at_span(c.span)
                        .param("found", c.id.clone())
                        .fix("choose", Edit::Choose { candidates: nearest(&c.id, CAP_KINDS, 3) }),
                );
                continue;
            }
        };
        let mut cap = Capability { id: c.id.clone(), kind, hosts: vec![], methods: vec![], max_request_bytes: 0, max_daily_bytes: 0 };
        let list = |p: &Prop| -> Option<Vec<String>> {
            match &p.value {
                Atom::List(v) => v
                    .iter()
                    .map(|(a, _)| match a {
                        Atom::Str(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => None,
            }
        };
        for p in &c.props {
            let bad_list = |name: &str| Diagnostic::new(E_SCHEMA, "schema.expected-string-list").at_span(p.span).param("prop", name);
            let bad_num = |name: &str| Diagnostic::new(E_SCHEMA, "schema.bad-number").at_span(p.span).param("prop", name);
            match p.name.as_str() {
                "hosts" | "urls" if (p.name == "hosts") == (cap.kind != "open-url") => match list(p) {
                    Some(v) => cap.hosts = v,
                    None => diags.push(bad_list(&p.name)),
                },
                "methods" => match list(p) {
                    Some(v) => cap.methods = v,
                    None => diags.push(bad_list("methods")),
                },
                "max-request" => match atom_u64(p) {
                    Some(v) => cap.max_request_bytes = v,
                    None => diags.push(bad_num("max-request")),
                },
                "max-daily" => match atom_u64(p) {
                    Some(v) => cap.max_daily_bytes = v,
                    None => diags.push(bad_num("max-daily")),
                },
                n => diags.push(unknown_field(p.span, "cap", n, &["hosts (net)", "urls (open-url)", "methods", "max-request", "max-daily"])),
            }
        }
        if cap.kind == "open-url" && cap.hosts.is_empty() {
            diags.push(
                Diagnostic::new(E_SCHEMA, "schema.missing-field")
                    .at_span(c.span)
                    .param("prop", "urls")
                    .fix("fields", Edit::FieldList { fields: vec!["urls".into()] }),
            );
        }
        if cap.kind == "net" && (cap.hosts.is_empty() || cap.methods.is_empty()) {
            diags.push(
                Diagnostic::new(E_SCHEMA, "schema.missing-field")
                    .at_span(c.span)
                    .param("prop", "hosts,methods")
                    .fix("fields", Edit::FieldList { fields: vec!["hosts".into(), "methods".into()] }),
            );
        }
        cap.hosts.sort();
        cap.methods.sort();
        caps.push(cap);
    }
    caps
}

fn atom_value(ty: &Ty, a: &Atom) -> Option<Value> {
    match (ty, a) {
        (Ty::Bool, Atom::Word(w)) if w == "true" || w == "false" => Some(Value::Bool(w == "true")),
        (Ty::Int, Atom::Num(n)) if n.parse::<i64>().is_ok() => Some(Value::Int(n.parse().unwrap())),
        (Ty::Text, Atom::Str(t)) => Some(Value::Text(nfc(t))),
        (Ty::Enum(v), Atom::Word(w)) if v.contains(w) => Some(Value::Enum(w.clone())),
        (Ty::Decimal, Atom::Num(n)) => match parse_num_expr(n, Span::default()) {
            Ok(Expr::Dec(d)) => Some(Value::Decimal(d)),
            Ok(Expr::Int(i)) => Some(Value::Decimal(i * 100)),
            _ => None,
        },
        _ => None,
    }
}

fn scalar_ty(name: &str) -> Option<Ty> {
    match name {
        "bool" => Some(Ty::Bool),
        "int" => Some(Ty::Int),
        "text" => Some(Ty::Text),
        "decimal" => Some(Ty::Decimal),
        _ => None,
    }
}

fn expand_state(raw: &RawModule, diags: &mut Diags) -> Vec<StateDecl> {
    let mut state: Vec<StateDecl> = Vec::new();
    for s in &raw.state {
        let ty = match s.ty.as_str() {
            "bool" => Ty::Bool,
            "int" => Ty::Int,
            "text" => Ty::Text,
            "decimal" => Ty::Decimal,
            "enum" => Ty::Enum(s.variants.clone()),
            "list" => {
                let mut fields = Vec::new();
                for (fname, fty) in &s.fields {
                    match scalar_ty(fty) {
                        Some(t) => fields.push((fname.clone(), t)),
                        None => diags.push(
                            Diagnostic::new(E_SCHEMA, "schema.unknown-type")
                                .at_span(s.span)
                                .param("found", fty.clone())
                                .fix("choose", Edit::Choose { candidates: vec!["bool".into(), "int".into(), "text".into(), "decimal".into()] }),
                        ),
                    }
                }
                let max = match s.max {
                    Some(m) if m >= 1 => m,
                    _ => {
                        diags.push(
                            Diagnostic::new("PVC-E-UNBOUNDED", "unbounded.list-max")
                                .at_span(s.span)
                                .param("list", s.name.clone())
                                .fix("add-prop", Edit::SetProp { prop: "max".into(), value: "<N>".into() }),
                        );
                        continue;
                    }
                };
                Ty::List { fields, max }
            }
            other => {
                diags.push(
                    Diagnostic::new(E_SCHEMA, "schema.unknown-type")
                        .at_span(s.span)
                        .param("found", other)
                        .fix("choose", Edit::Choose { candidates: TyKind::names().iter().map(|x| x.to_string()).collect() }),
                );
                continue;
            }
        };
        let bad_init = |sp: Span| {
            Diagnostic::new(E_TYPE, "type.bad-initial-value").at_span(sp).param("field", s.name.clone()).param("expected", s.ty.clone())
        };
        let init = match (&ty, &s.init) {
            (Ty::Bool, None) => Value::Bool(false),
            (Ty::Int, None) => Value::Int(0),
            (Ty::Text, None) => Value::Text(String::new()),
            (Ty::Decimal, None) => Value::Decimal(0),
            (Ty::Enum(v), None) => Value::Enum(v.first().cloned().unwrap_or_default()),
            (Ty::List { .. }, None) => Value::List(Vec::new()),
            (Ty::List { fields, max }, Some((Atom::Rows(rows), sp))) => {
                let mut out = Vec::new();
                let mut ok = true;
                if rows.len() as u32 > *max {
                    diags.push(bad_init(*sp).param("detail", "more rows than max"));
                    ok = false;
                }
                for r in rows {
                    let mut row = Vec::new();
                    for (fname, fty) in fields {
                        match r.iter().find(|(n, _, _)| n == fname) {
                            Some((_, a, asp)) => match atom_value(fty, a) {
                                Some(v) => row.push((fname.clone(), v)),
                                None => {
                                    diags.push(bad_init(*asp).param("detail", fname.clone()));
                                    ok = false;
                                }
                            },
                            None => {
                                diags.push(bad_init(*sp).param("detail", format!("missing {}", fname)));
                                ok = false;
                            }
                        }
                    }
                    for (n, _, asp) in r {
                        if !fields.iter().any(|(f, _)| f == n) {
                            diags.push(
                                Diagnostic::new(E_SCHEMA, "schema.unknown-field")
                                    .at_span(*asp)
                                    .param("kind", "list-row")
                                    .param("prop", n.clone())
                                    .fix("fields", Edit::FieldList { fields: fields.iter().map(|(f, _)| f.clone()).collect() }),
                            );
                            ok = false;
                        }
                    }
                    out.push(row);
                }
                if !ok {
                    continue;
                }
                Value::List(out)
            }
            (_, Some((a, sp))) => match atom_value(&ty, a) {
                Some(v) => v,
                None => {
                    diags.push(bad_init(*sp));
                    continue;
                }
            },
        };
        if state.iter().any(|d| d.name == s.name) {
            diags.push(Diagnostic::new(E_ID_COLLISION, "id.duplicate-state").at_span(s.span).param("name", s.name.clone()));
            continue;
        }
        state.push(StateDecl { name: s.name.clone(), ty, init });
    }
    state
}

/// Expand a raw module to the canonical AST. Returns every schema-level diagnostic found.
pub fn expand(raw: &RawModule) -> Result<Module, Diags> {
    let theme_name = raw.theme.clone().unwrap_or_else(|| "dark".to_string());
    let mut diags: Diags = Vec::new();
    let theme = match theme::builtin(&theme_name) {
        Some(t) => t,
        None => {
            diags.push(
                Diagnostic::new(E_UNKNOWN_TOKEN, "token.unknown-theme")
                    .at_span(raw.header_span)
                    .param("found", theme_name.clone())
                    .fix("choose", Edit::Choose { candidates: nearest(&theme_name, theme::theme_names(), 3) }),
            );
            theme::builtin("dark").unwrap()
        }
    };
    for (what, found) in [("ir", &raw.ir), ("std", &raw.std)] {
        if parse_ver(found) != Some((0, 1)) {
            diags.push(
                Diagnostic::new(E_SCHEMA, "schema.unsupported-version")
                    .at_span(raw.header_span)
                    .param("what", what)
                    .param("found", found.clone())
                    .param("supported", "0.1"),
            );
        }
    }
    let locales = if raw.locales.is_empty() { vec!["en".to_string()] } else { raw.locales.clone() };
    let fonts = expand_fonts(raw, &mut diags);
    let caps = expand_caps(raw, &mut diags);
    let state = expand_state(raw, &mut diags);

    let mut ex = Ex {
        theme: theme.clone(),
        diags: Vec::new(),
        prov: Vec::new(),
        default_locale: locales[0].clone(),
        inline: Vec::new(),
        names: state.iter().map(|s| s.name.clone()).chain(raw.derived.iter().map(|d| d.0.clone())).collect(),
        variants: state
            .iter()
            .flat_map(|s| match &s.ty {
                Ty::Enum(v) => v.clone(),
                _ => vec![],
            })
            .collect(),
        form_stack: Vec::new(),
        avatar_pending: Vec::new(),
    };
    let derived: Vec<Derived> =
        raw.derived.iter().map(|(n, e, _)| Derived { name: n.clone(), expr: ex.resolve(e.clone()) }).collect();
    let effects: Vec<Effect> = raw.effects.iter().map(|(n, c, _)| Effect { name: n.clone(), cap: c.clone() }).collect();

    let root = if raw.nodes.len() == 1 && raw.nodes[0].kind == "surface" {
        let mut counters = BTreeMap::new();
        ex.node(&raw.nodes[0], "", &mut counters)
    } else if raw.nodes.is_empty() {
        ex.diags.push(Diagnostic::new(E_SCHEMA, "schema.empty-module").at_span(raw.header_span));
        None
    } else {
        // implicit surface wrapping the top-level nodes
        let id = "surface0".to_string();
        let mut s = mk_node(&id, NodeKind::Surface, Role::Container);
        s.layout.w = Size::Fill;
        s.layout.h = Size::Fill;
        s.layout.pad = Edges::all(Len::from_px(24));
        s.layout.align = Align::Center;
        s.paint.fill = Some(tok("bg"));
        ex.prov.push((id.clone(), raw.nodes[0].span));
        s.children = ex.children(&raw.nodes, &id);
        Some(s)
    };

    let mut tables: Vec<LocaleTable> =
        locales.iter().map(|l| LocaleTable { locale: l.clone(), entries: Vec::new() }).collect();
    for (loc, entries) in &raw.strings {
        match tables.iter_mut().find(|t| &t.locale == loc) {
            Some(t) => {
                for (k, v, sp) in entries {
                    if t.entries.iter().any(|(kk, _)| kk == k) {
                        diags.push(Diagnostic::new(E_ID_COLLISION, "id.duplicate-string-key").at_span(*sp).param("key", k.clone()));
                    } else {
                        t.entries.push((k.clone(), nfc(v)));
                    }
                }
            }
            None => diags.push(
                Diagnostic::new(E_SCHEMA, "schema.locale-not-declared")
                    .at_span(raw.header_span)
                    .param("locale", loc.clone())
                    .fix("choose", Edit::Choose { candidates: locales.clone() })
                    .fix(
                        "add-line",
                        Edit::AddLine {
                            line: format!("locales {}", locales.iter().cloned().chain([loc.clone()]).collect::<Vec<_>>().join(" ")),
                        },
                    ),
            ),
        }
    }
    for (k, v) in &ex.inline {
        let t = tables.iter_mut().find(|t| t.locale == ex.default_locale).unwrap();
        if t.entries.iter().any(|(kk, _)| kk == k) {
            ex.diags.push(Diagnostic::new(E_ID_COLLISION, "id.duplicate-string-key").param("key", k.clone()));
        } else {
            t.entries.push((k.clone(), v.clone()));
        }
    }
    for t in &mut tables {
        t.entries.sort_by(|a, b| a.0.cmp(&b.0));
    }

    diags.extend(ex.diags);
    let root = match root {
        Some(r) if !diags.iter().any(|d| d.is_error()) => r,
        _ => {
            diags.sort_by_key(|d| d.location.span.map(|s| (s.line, s.col)).unwrap_or((0, 0)));
            return Err(diags);
        }
    };
    Ok(Module {
        header: Header { ir: (0, 1), std: (0, 1), dialect: raw.dialect.clone(), min_runtime: (0, 1) },
        manifest: Manifest { locales, fonts, caps, viewport: (320, 4096, 320, 4096), text_scale: (100, 200) },
        theme,
        strings: Strings { tables },
        state,
        derived,
        effects,
        root,
        provenance: ex.prov,
    })
}


// ======================================================================== more kinds

impl Ex {
    fn std_name(&mut self, pr: &mut Pr, id: &str, n: &mut Node, label: &Option<String>, span: Span) {
        let explicit = pr.prop("name").and_then(|p| self.strprop(p, id, "name"));
        match (label, explicit) {
            (_, Some(k)) => n.sem.name = Some(Name::Key(k)),
            (Some(l), None) => n.sem.name = Some(Name::Key(l.clone())),
            (None, None) => self.schema(span, "schema.missing-label", "label", "", &["name"]),
        }
    }

    pub(crate) fn k_link(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let label = self.label_arg(&mut pr, id, "label", raw.span, false);
        let mut n = mk_node(id, NodeKind::Box, Role::Link);
        n.layout.dir = Dir::Row;
        n.layout.gap = Len::from_px(8);
        n.layout.align = Align::Center;
        self.std_name(&mut pr, id, &mut n, &label, raw.span);
        if let Some(l) = label {
            let mut t = self.std_text(&format!("{}/label", id), l, 14, 400, "accent", raw.span);
            t.sem.decorative = true;
            n.children.push(t);
        }
        if let Some(p) = pr.prop("href") {
            match &p.value {
                Atom::Str(u) => n.on.push(Handler { event: Event::Press, guard: None, actions: vec![Action::OpenUrl(nfc(u))] }),
                other => self.schema(p.span, "schema.expected-string", "href", &describe_atom(other), &[]),
            }
        }
        if let Some(p) = pr.prop("w") {
            if let Some(s) = self.size(p) {
                n.layout.w = s;
            }
        }
        if let Some(p) = pr.prop("disabled") {
            n.sem.disabled = self.expr(p);
        }
        if let Some(p) = pr.prop("show") {
            n.show = self.expr(p);
        }
        let mut hs = Vec::new();
        self.handlers(&mut pr, &mut hs);
        n.on.extend(hs);
        self.at_props(&mut pr, &mut n);
        let mut kids = self.children(&raw.children, id);
        n.children.append(&mut kids);
        self.finish(pr, &["id", "href", "name", "w", "disabled", "show", "on.press"], &[], 1);
        Some(n)
    }

    pub(crate) fn k_radio(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let label = self.label_arg(&mut pr, id, "label", raw.span, false);
        let bind = self.req_bind(&mut pr, raw);
        let value = match pr.prop("value") {
            Some(p) => self.ident_prop(p).unwrap_or_default(),
            None => {
                self.schema(raw.span, "schema.missing-field", "value", "", &["value"]);
                String::new()
            }
        };
        let mut n = mk_node(id, NodeKind::Box, Role::Radio);
        n.layout.dir = Dir::Row;
        n.layout.gap = Len::from_px(8);
        n.layout.align = Align::Center;
        n.layout.pad = Edges::xy(Len::ZERO, Len::from_px(2));
        self.std_name(&mut pr, id, &mut n, &label, raw.span);
        n.sem.checked = Some(Expr::Cmp(CmpOp::Eq, Box::new(Expr::Var(bind.clone())), Box::new(Expr::Variant(value.clone()))));
        n.on.push(Handler {
            event: Event::Press,
            guard: None,
            actions: vec![Action::Assign(bind.clone(), Expr::Variant(value.clone()))],
        });
        let rid = format!("{}/ring", id);
        let mut ring = mk_node(&rid, NodeKind::Box, Role::Container);
        ring.sem.decorative = true;
        ring.layout.dir = Dir::Row;
        ring.layout.w = Size::Px(Len::from_px(20));
        ring.layout.h = Size::Px(Len::from_px(20));
        ring.layout.align = Align::Center;
        ring.layout.justify = Justify::Center;
        ring.paint = Paint {
            fill: Some(tok("input-bg")),
            border: Some(Border { width: Len::from_px(1), color: tok("border") }),
            shape: Shape::Circle,
        };
        let did = format!("{}/dot", id);
        let mut dot = mk_node(&did, NodeKind::Shape, Role::Image);
        dot.sem.decorative = true;
        dot.layout.w = Size::Px(Len::from_px(10));
        dot.layout.h = Size::Px(Len::from_px(10));
        dot.paint = Paint { fill: Some(tok("accent")), border: None, shape: Shape::Circle };
        dot.show = n.sem.checked.clone();
        self.prov.push((rid, raw.span));
        self.prov.push((did, raw.span));
        ring.children.push(dot);
        n.children.push(ring);
        if let Some(l) = label {
            let mut t = self.std_text(&format!("{}/label", id), l, 14, 400, "fg", raw.span);
            t.sem.decorative = true;
            n.children.push(t);
        }
        if let Some(p) = pr.prop("disabled") {
            n.sem.disabled = self.expr(p);
        }
        if let Some(p) = pr.prop("show") {
            n.show = self.expr(p);
        }
        let mut hs = Vec::new();
        self.handlers(&mut pr, &mut hs);
        n.on.extend(hs);
        self.at_props(&mut pr, &mut n);
        self.finish(pr, &["id", "bind", "value", "name", "disabled", "show", "on.press"], &[], 1);
        Some(n)
    }

    pub(crate) fn k_tab(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let label = self.label_arg(&mut pr, id, "label", raw.span, true).unwrap_or_default();
        let mut n = self.button_node(id, label, None, raw.span);
        n.sem.role = Role::Tab;
        n.layout.pad = Edges::xy(Len::from_px(16), Len::from_px(10));
        if let Some(p) = pr.prop("selected") {
            n.sem.selected = self.expr(p);
        }
        if n.sem.selected.is_none() {
            self.schema(raw.span, "schema.missing-field", "selected", "", &["selected"]);
        }
        if let Some(p) = pr.prop("disabled") {
            n.sem.disabled = self.expr(p);
        }
        if let Some(p) = pr.prop("show") {
            n.show = self.expr(p);
        }
        let mut hs = Vec::new();
        self.handlers(&mut pr, &mut hs);
        n.on = hs;
        self.at_props(&mut pr, &mut n);
        self.finish(pr, &["id", "selected", "disabled", "show", "on.press"], &[], 1);
        Some(n)
    }

    pub(crate) fn k_shape(&mut self, _raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let mut n = mk_node(id, NodeKind::Shape, Role::Image);
        n.layout.w = Size::Px(Len::from_px(16));
        n.layout.h = Size::Px(Len::from_px(16));
        n.paint = Paint { fill: Some(tok("accent")), border: None, shape: Shape::RoundedRect(Len::ZERO) };
        for g in ["circle", "rect", "rounded"] {
            if pr.flag(g) {
                n.paint.shape = match g {
                    "circle" => Shape::Circle,
                    "rounded" => Shape::RoundedRect(Len::from_px(4)),
                    _ => Shape::RoundedRect(Len::ZERO),
                };
            }
        }
        if pr.flag("decorative") {
            n.sem.decorative = true;
        }
        if let Some(p) = pr.prop("alt") {
            if let Some(k) = self.strprop(p, id, "alt") {
                n.sem.name = Some(Name::Key(k));
            }
        }
        if let Some(p) = pr.prop("name") {
            if let Some(k) = self.strprop(p, id, "name") {
                n.sem.name = Some(Name::Key(k));
            }
        }
        if let Some(p) = pr.prop("show") {
            n.show = self.expr(p);
        }
        self.box_layout(&mut pr, &mut n);
        self.box_paint(&mut pr, &mut n);
        self.at_props(&mut pr, &mut n);
        self.finish(
            pr,
            &["id", "w", "h", "fill", "border", "radius", "alt", "name", "show", "md.<prop>"],
            &["circle", "rect", "rounded", "decorative"],
            0,
        );
        Some(n)
    }

    pub(crate) fn k_layer(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let mut n = mk_node(id, NodeKind::Layer, Role::Dialog);
        n.layout.pad = Edges::all(Len::from_px(24));
        n.layout.gap = Len::from_px(16);
        n.layout.w = Size::Px(Len::from_px(320));
        n.paint = Paint {
            fill: Some(tok("surface")),
            border: Some(Border { width: Len::from_px(1), color: tok("border") }),
            shape: Shape::RoundedRect(Len::from_px(12)),
        };
        self.common(&mut pr, &mut n, id, true);
        if !matches!(n.sem.role, Role::Dialog | Role::AlertDialog) {
            self.schema(raw.span, "schema.bad-layer-role", "role", n.sem.role.as_str(), &[]);
            self.diags.last_mut().unwrap().fixes.push(Fix {
                key: "choose".into(),
                edit: Edit::Choose { candidates: vec!["dialog".into(), "alert-dialog".into()] },
            });
        }
        self.box_layout(&mut pr, &mut n);
        self.box_paint(&mut pr, &mut n);
        if n.show.is_none() {
            self.schema(raw.span, "schema.missing-field", "show", "", &["show"]);
        }
        n.children = self.children(&raw.children, id);
        let mut props: Vec<&str> = Self::BOX_PROPS.to_vec();
        props.retain(|p| !p.starts_with("on."));
        self.finish(pr, &props, &["decorative"], 0);
        Some(n)
    }

    pub(crate) fn k_slot(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let on = match pr.prop("on") {
            Some(p) => self.expr(p),
            None => {
                self.schema(raw.span, "schema.missing-field", "on", "", &["on"]);
                None
            }
        };
        let mut n = mk_node(id, NodeKind::Box, Role::Container);
        if let Some(p) = pr.prop("show") {
            n.show = self.expr(p);
        }
        self.box_layout(&mut pr, &mut n);
        self.at_props(&mut pr, &mut n);
        let mut cases = Vec::new();
        for c in &raw.children {
            if c.kind != "case" {
                self.span_diag(
                    Diagnostic::new(E_SCHEMA, "schema.slot-child-not-case")
                        .at_span(c.span)
                        .param("found", c.kind.clone())
                        .fix("add-line", Edit::AddLine { line: "case <variant> { ... }".into() }),
                );
                continue;
            }
            let variant = match c.flags.first() {
                Some((v, _)) => v.clone(),
                None => {
                    self.schema(c.span, "schema.missing-field", "case-value", "", &[]);
                    continue;
                }
            };
            if cases.contains(&variant) {
                self.span_diag(Diagnostic::new(E_ID_COLLISION, "id.duplicate-case").at_span(c.span).param("case", variant.clone()));
                continue;
            }
            let cid = format!("{}/{}", id, variant);
            self.prov.push((cid.clone(), c.span));
            let mut cpr = Pr::new(c);
            cpr.flag(&variant);
            let mut cn = mk_node(&cid, NodeKind::Box, Role::Container);
            self.common(&mut cpr, &mut cn, &cid, true);
            self.box_layout(&mut cpr, &mut cn);
            self.box_paint(&mut cpr, &mut cn);
            cn.children = self.children(&c.children, &cid);
            let mut props: Vec<&str> = Self::BOX_PROPS.to_vec();
            props.retain(|p| !p.starts_with("on."));
            self.finish(cpr, &props, &["decorative"], 0);
            cases.push(variant);
            n.children.push(cn);
        }
        let on = on?;
        n.kind = NodeKind::Slot(SlotProps { on, cases });
        self.finish(pr, &["id", "on", "show", "w", "h", "pad", "gap", "align", "justify"], &[], 0);
        Some(n)
    }

    pub(crate) fn k_view(&mut self, raw: &RawNode, mut pr: Pr, id: &str) -> Option<Node> {
        let list = match raw.flags.first() {
            Some((l, _)) => {
                pr.flag(l);
                l.clone()
            }
            None => {
                self.schema(raw.span, "schema.missing-field", "list", "", &["<list-name>"]);
                String::new()
            }
        };
        let alias = match pr.prop("as") {
            Some(p) => self.ident_prop(p).unwrap_or_else(|| "item".into()),
            None => "item".into(),
        };
        let filter = pr.prop("where").and_then(|p| self.expr(p));
        let mut n = mk_node(id, NodeKind::Box, Role::Container);
        self.common(&mut pr, &mut n, id, true);
        self.box_layout(&mut pr, &mut n);
        n.children = self.children(&raw.children, id);
        n.kind = NodeKind::View(ViewProps { list, alias, filter });
        let mut props: Vec<&str> = Self::BOX_PROPS.to_vec();
        props.push("as");
        props.push("where");
        self.finish(pr, &props, &["decorative"], 0);
        Some(n)
    }
}
