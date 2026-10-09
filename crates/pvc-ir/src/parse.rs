//! Terse-form parser: tokens -> raw syntax tree (no std expansion, no type checks). GRAMMAR.md §3.

use crate::ast::{CmpOp, Expr, Func};
use crate::diag::{Diagnostic, Edit, Span, E_PARSE};
use crate::lex::{lex, Tok, Token};

#[derive(Clone, Debug, PartialEq)]
pub enum Atom {
    Str(String),
    Key(String),
    Num(String),
    Word(String),
    Expr(Expr),
    List(Vec<(Atom, Span)>),
    Handler(Vec<Stmt>),
    /// List-literal rows (state initialisers only): `[ { f = v … }, … ]`.
    Rows(Vec<Vec<(String, Atom, Span)>>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    When(Expr, Span),
    Assign(String, Expr, Span),
    Emit(String, Span),
    /// `append <list> { f = expr … }`
    Append(String, Vec<(String, Expr)>, Span),
    /// `remove <alias>`
    Remove(String, Span),
}

#[derive(Clone, Debug)]
pub struct Prop {
    pub name: String,
    pub value: Atom,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct RawNode {
    pub kind: String,
    pub span: Span,
    /// Positional atoms (strings, keys, numbers) in order.
    pub args: Vec<(Atom, Span)>,
    pub flags: Vec<(String, Span)>,
    pub props: Vec<Prop>,
    pub children: Vec<RawNode>,
}

#[derive(Clone, Debug)]
pub struct RawFont {
    pub family: String,
    pub weight: String,
    pub props: Vec<Prop>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct RawCap {
    pub id: String,
    pub props: Vec<Prop>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct RawState {
    pub name: String,
    pub ty: String,
    pub variants: Vec<String>,
    /// `list(name:type, …)` fields.
    pub fields: Vec<(String, String)>,
    pub max: Option<u32>,
    pub init: Option<(Atom, Span)>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct RawModule {
    pub ir: String,
    pub std: String,
    pub theme: Option<String>,
    pub dialect: Option<String>,
    pub locales: Vec<String>,
    pub fonts: Vec<RawFont>,
    /// `fonts std`: the std@0.1 default font set.
    pub fonts_std: bool,
    pub caps: Vec<RawCap>,
    pub state: Vec<RawState>,
    pub derived: Vec<(String, Expr, Span)>,
    pub effects: Vec<(String, String, Span)>,
    pub strings: Vec<(String, Vec<(String, String, Span)>)>,
    pub nodes: Vec<RawNode>,
    pub header_span: Span,
}

pub const KEYWORDS: &[&str] = &[
    "pvc", "locales", "font", "fonts", "cap", "state", "derive", "effect", "strings",
];

struct P {
    t: Vec<Token>,
    i: usize,
}

type R<T> = Result<T, Diagnostic>;

impl P {
    fn peek(&self) -> &Tok {
        &self.t[self.i].tok
    }
    fn peek_at(&self, n: usize) -> &Tok {
        &self.t[(self.i + n).min(self.t.len() - 1)].tok
    }
    fn span(&self) -> Span {
        self.t[self.i].span
    }
    fn bump(&mut self) -> Token {
        let t = self.t[self.i].clone();
        if self.i < self.t.len() - 1 {
            self.i += 1;
        }
        t
    }
    fn describe(t: &Tok) -> String {
        match t {
            Tok::Ident(s) => format!("word:{}", s),
            Tok::Key(s) => format!("@{}", s),
            Tok::Num(s) => format!("number:{}", s),
            Tok::Str(_) => "string".into(),
            Tok::P(p) => format!("'{}'", p),
            Tok::Nl => "end-of-line".into(),
            Tok::Eof => "end-of-input".into(),
        }
    }
    fn err_expected(&self, expected: &str) -> Diagnostic {
        Diagnostic::new(E_PARSE, "parse.expected")
            .at_span(self.span())
            .param("expected", expected)
            .param("found", Self::describe(self.peek()))
            .fix("insert", Edit::Expect { expected: expected.to_string() })
    }
    fn is_p(&self, p: &str) -> bool {
        matches!(self.peek(), Tok::P(q) if *q == p)
    }
    fn eat_p(&mut self, p: &str) -> bool {
        if self.is_p(p) {
            self.bump();
            true
        } else {
            false
        }
    }
    fn expect_p(&mut self, p: &str) -> R<Span> {
        if self.is_p(p) {
            Ok(self.bump().span)
        } else {
            Err(self.err_expected(&format!("'{}'", p)))
        }
    }
    fn ident(&mut self, what: &str) -> R<(String, Span)> {
        match self.peek().clone() {
            Tok::Ident(s) => {
                let sp = self.bump().span;
                Ok((s, sp))
            }
            _ => Err(self.err_expected(what)),
        }
    }
    fn skip_nl(&mut self) {
        while matches!(self.peek(), Tok::Nl) || self.is_p(";") {
            self.bump();
        }
    }
    fn at_stmt_end(&self) -> bool {
        matches!(self.peek(), Tok::Nl | Tok::Eof) || self.is_p(";") || self.is_p("}")
    }
    fn end_stmt(&mut self) -> R<()> {
        if self.at_stmt_end() {
            Ok(())
        } else {
            Err(self.err_expected("end-of-line-or-';'"))
        }
    }

    // ------------------------------------------------------------ expressions
    fn expr(&mut self) -> R<Expr> {
        self.or()
    }
    fn or(&mut self) -> R<Expr> {
        let mut l = self.and()?;
        while matches!(self.peek(), Tok::Ident(s) if s == "or") {
            self.bump();
            let r = self.and()?;
            l = Expr::Or(Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    fn and(&mut self) -> R<Expr> {
        let mut l = self.not()?;
        while matches!(self.peek(), Tok::Ident(s) if s == "and") {
            self.bump();
            let r = self.not()?;
            l = Expr::And(Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    fn not(&mut self) -> R<Expr> {
        if matches!(self.peek(), Tok::Ident(s) if s == "not") {
            self.bump();
            let e = self.not()?;
            return Ok(Expr::Not(Box::new(e)));
        }
        self.cmp()
    }
    fn add(&mut self) -> R<Expr> {
        let mut l = self.mul()?;
        loop {
            if self.is_p("+") {
                self.bump();
                let r = self.mul()?;
                l = Expr::Add(Box::new(l), Box::new(r));
            } else if self.is_p("-") {
                self.bump();
                let r = self.mul()?;
                l = Expr::Sub(Box::new(l), Box::new(r));
            } else {
                return Ok(l);
            }
        }
    }
    fn mul(&mut self) -> R<Expr> {
        let mut l = self.atom_expr()?;
        while self.is_p("*") {
            self.bump();
            let r = self.atom_expr()?;
            l = Expr::Mul(Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    fn cmp(&mut self) -> R<Expr> {
        let l = self.add()?;
        let op = match self.peek() {
            Tok::P("==") => Some(CmpOp::Eq),
            Tok::P("!=") => Some(CmpOp::Ne),
            Tok::P("<") => Some(CmpOp::Lt),
            Tok::P("<=") => Some(CmpOp::Le),
            Tok::P(">") => Some(CmpOp::Gt),
            Tok::P(">=") => Some(CmpOp::Ge),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let r = self.add()?;
            return Ok(Expr::Cmp(op, Box::new(l), Box::new(r)));
        }
        Ok(l)
    }
    fn atom_expr(&mut self) -> R<Expr> {
        match self.peek().clone() {
            Tok::P("(") => {
                self.bump();
                let e = self.expr()?;
                self.expect_p(")")?;
                Ok(e)
            }
            Tok::P("-") => {
                self.bump();
                match self.peek().clone() {
                    Tok::Num(n) => {
                        let sp = self.span();
                        self.bump();
                        match parse_num_expr(&n, sp)? {
                            Expr::Int(i) => Ok(Expr::Int(-i)),
                            Expr::Dec(d) => Ok(Expr::Dec(-d)),
                            e => Ok(e),
                        }
                    }
                    _ => Err(self.err_expected("number")),
                }
            }
            Tok::Num(n) => {
                let sp = self.span();
                self.bump();
                parse_num_expr(&n, sp)
            }
            Tok::Str(s) => {
                self.bump();
                Ok(Expr::Str(s))
            }
            Tok::Ident(w) => {
                let sp = self.span();
                self.bump();
                if w == "true" {
                    return Ok(Expr::Bool(true));
                }
                if w == "false" {
                    return Ok(Expr::Bool(false));
                }
                if self.is_p("(") {
                    self.bump();
                    let mut args = Vec::new();
                    if !self.is_p(")") {
                        loop {
                            args.push(self.expr()?);
                            if !self.eat_p(",") {
                                break;
                            }
                        }
                    }
                    self.expect_p(")")?;
                    return match Func::from_name(&w) {
                        Some(f) => Ok(Expr::Call(f, args)),
                        None => Err(Diagnostic::new(E_PARSE, "parse.unknown-function")
                            .at_span(sp)
                            .param("found", w.clone())
                            .fix(
                                "choose",
                                Edit::Choose {
                                    candidates: crate::diag::nearest(&w, Func::names(), 3),
                                },
                            )),
                    };
                }
                // Variable or enum variant: resolved later against state declarations.
                Ok(Expr::Var(w))
            }
            _ => Err(self.err_expected("expression")),
        }
    }

    // ------------------------------------------------------------ atoms & statements
    fn atom(&mut self) -> R<Atom> {
        match self.peek().clone() {
            Tok::Str(s) => {
                self.bump();
                Ok(Atom::Str(s))
            }
            Tok::Key(k) => {
                self.bump();
                Ok(Atom::Key(k))
            }
            Tok::Num(n) => {
                self.bump();
                Ok(Atom::Num(n))
            }
            Tok::P("-") if matches!(self.peek_at(1), Tok::Num(_)) => {
                self.bump();
                if let Tok::Num(n) = self.bump().tok {
                    Ok(Atom::Num(format!("-{}", n)))
                } else {
                    unreachable!()
                }
            }
            Tok::Ident(w) => {
                self.bump();
                Ok(Atom::Word(w))
            }
            Tok::P("(") => {
                self.bump();
                let e = self.expr()?;
                self.expect_p(")")?;
                Ok(Atom::Expr(e))
            }
            Tok::P("[") => {
                self.bump();
                let mut v = Vec::new();
                loop {
                    while matches!(self.peek(), Tok::Nl) {
                        self.bump();
                    }
                    if self.is_p("]") {
                        break;
                    }
                    let sp = self.span();
                    v.push((self.atom()?, sp));
                    self.eat_p(",");
                }
                self.expect_p("]")?;
                Ok(Atom::List(v))
            }
            Tok::P("{") => {
                self.bump();
                let mut stmts = Vec::new();
                loop {
                    self.skip_nl();
                    if self.is_p("}") {
                        break;
                    }
                    stmts.push(self.stmt()?);
                    if !self.at_stmt_end() {
                        return Err(self.err_expected("end-of-line-or-';'"));
                    }
                }
                self.expect_p("}")?;
                Ok(Atom::Handler(stmts))
            }
            _ => Err(self.err_expected("value")),
        }
    }

    /// `[ { f = v  g = v }, { … } ]` (list initialiser)
    fn rows(&mut self) -> R<Atom> {
        self.expect_p("[")?;
        let mut rows = Vec::new();
        loop {
            while matches!(self.peek(), Tok::Nl) || self.is_p(",") {
                self.bump();
            }
            if self.is_p("]") {
                break;
            }
            self.expect_p("{")?;
            let mut row = Vec::new();
            loop {
                while matches!(self.peek(), Tok::Nl) || self.is_p(",") || self.is_p(";") {
                    self.bump();
                }
                if self.is_p("}") {
                    break;
                }
                let sp = self.span();
                let (f, _) = self.ident("field-name")?;
                self.expect_p("=")?;
                row.push((f, self.atom()?, sp));
            }
            self.expect_p("}")?;
            rows.push(row);
        }
        self.expect_p("]")?;
        Ok(Atom::Rows(rows))
    }

    fn stmt(&mut self) -> R<Stmt> {
        let sp = self.span();
        let (w, _) = self.ident("statement")?;
        match w.as_str() {
            "when" => Ok(Stmt::When(self.expr()?, sp)),
            "emit" => {
                let (n, _) = self.ident("effect-name")?;
                Ok(Stmt::Emit(n, sp))
            }
            "remove" => {
                let (n, _) = self.ident("row-alias")?;
                Ok(Stmt::Remove(n, sp))
            }
            "append" => {
                let (list, _) = self.ident("list-name")?;
                self.expect_p("{")?;
                let mut fields = Vec::new();
                loop {
                    while matches!(self.peek(), Tok::Nl) || self.is_p(";") || self.is_p(",") {
                        self.bump();
                    }
                    if self.is_p("}") {
                        break;
                    }
                    let (f, _) = self.ident("field-name")?;
                    self.expect_p("=")?;
                    fields.push((f, self.expr()?));
                }
                self.expect_p("}")?;
                Ok(Stmt::Append(list, fields, sp))
            }
            _ => {
                self.expect_p("=")?;
                Ok(Stmt::Assign(w, self.expr()?, sp))
            }
        }
    }

    // ------------------------------------------------------------ items
    fn props_until_end(&mut self) -> R<Vec<Prop>> {
        let mut props = Vec::new();
        while !self.at_stmt_end() {
            let (name, span) = self.ident("property")?;
            self.expect_p("=")?;
            let value = self.atom()?;
            props.push(Prop { name, value, span });
        }
        Ok(props)
    }

    fn node(&mut self) -> R<RawNode> {
        let span = self.span();
        let (kind, _) = self.ident("node-kind")?;
        let mut n = RawNode {
            kind,
            span,
            args: Vec::new(),
            flags: Vec::new(),
            props: Vec::new(),
            children: Vec::new(),
        };
        loop {
            if self.at_stmt_end() {
                break;
            }
            let sp = self.span();
            match self.peek().clone() {
                Tok::Str(_) | Tok::Key(_) | Tok::Num(_) => {
                    let a = self.atom()?;
                    n.args.push((a, sp));
                }
                Tok::Ident(w) => {
                    if matches!(self.peek_at(1), Tok::P("=")) {
                        self.bump();
                        self.bump();
                        let value = self.atom()?;
                        n.props.push(Prop { name: w, value, span: sp });
                    } else {
                        self.bump();
                        n.flags.push((w, sp));
                    }
                }
                Tok::P("{") => {
                    self.bump();
                    loop {
                        self.skip_nl();
                        if self.is_p("}") {
                            break;
                        }
                        if matches!(self.peek(), Tok::Eof) {
                            return Err(self.err_expected("'}'"));
                        }
                        n.children.push(self.node()?);
                        if !self.at_stmt_end() {
                            return Err(self.err_expected("end-of-line-or-';'"));
                        }
                    }
                    self.expect_p("}")?;
                    break;
                }
                _ => return Err(self.err_expected("property-or-'{'")),
            }
        }
        Ok(n)
    }

    fn module(&mut self) -> R<RawModule> {
        self.skip_nl();
        let header_span = self.span();
        match self.peek().clone() {
            Tok::Ident(w) if w == "pvc" => {
                self.bump();
            }
            _ => return Err(self.err_expected("'pvc <version>'")),
        }
        let ir = match self.peek().clone() {
            Tok::Num(n) => {
                self.bump();
                n
            }
            _ => return Err(self.err_expected("ir-version")),
        };
        let mut m = RawModule {
            ir,
            std: "0.1".into(),
            theme: None,
            dialect: None,
            locales: Vec::new(),
            fonts: Vec::new(),
            fonts_std: false,
            caps: Vec::new(),
            state: Vec::new(),
            derived: Vec::new(),
            effects: Vec::new(),
            strings: Vec::new(),
            nodes: Vec::new(),
            header_span,
        };
        // header continuation: `use std@0.1 theme dark dialect en@0.1`
        while !self.at_stmt_end() {
            let (w, sp) = self.ident("'use', 'theme' or 'dialect'")?;
            match w.as_str() {
                "use" => {
                    let (s, _) = self.ident("std@<version>")?;
                    match s.strip_prefix("std@") {
                        Some(v) => m.std = v.to_string(),
                        None => {
                            return Err(Diagnostic::new(E_PARSE, "parse.expected")
                                .at_span(sp)
                                .param("expected", "std@<version>")
                                .param("found", s))
                        }
                    }
                }
                "theme" => m.theme = Some(self.ident("theme-name")?.0),
                "dialect" => m.dialect = Some(self.ident("dialect-id")?.0),
                _ => {
                    return Err(Diagnostic::new(E_PARSE, "parse.expected")
                        .at_span(sp)
                        .param("expected", "use|theme|dialect")
                        .param("found", w))
                }
            }
        }
        loop {
            self.skip_nl();
            if matches!(self.peek(), Tok::Eof) {
                break;
            }
            let sp = self.span();
            let kw = match self.peek() {
                Tok::Ident(w) if KEYWORDS.contains(&w.as_str()) => Some(w.clone()),
                _ => None,
            };
            match kw.as_deref() {
                Some("pvc") => {
                    return Err(Diagnostic::new(E_PARSE, "parse.duplicate-header").at_span(sp))
                }
                Some("locales") => {
                    self.bump();
                    while !self.at_stmt_end() {
                        m.locales.push(self.ident("locale")?.0);
                    }
                }
                Some("fonts") => {
                    self.bump();
                    match self.peek().clone() {
                        Tok::Ident(w) if w == "std" => {
                            self.bump();
                            m.fonts_std = true;
                        }
                        _ => return Err(self.err_expected("'std'")),
                    }
                    self.end_stmt()?;
                }
                Some("font") => {
                    self.bump();
                    let (family, _) = self.ident("font-family-id")?;
                    let weight = match self.peek().clone() {
                        Tok::Num(n) => {
                            self.bump();
                            n
                        }
                        _ => return Err(self.err_expected("font-weight")),
                    };
                    let props = self.props_until_end()?;
                    m.fonts.push(RawFont { family, weight, props, span: sp });
                }
                Some("cap") => {
                    self.bump();
                    let (id, _) = self.ident("capability-id")?;
                    let props = self.props_until_end()?;
                    m.caps.push(RawCap { id, props, span: sp });
                }
                Some("state") => {
                    self.bump();
                    self.skip_nl();
                    self.expect_p("{")?;
                    loop {
                        self.skip_nl();
                        if self.is_p("}") {
                            break;
                        }
                        let dsp = self.span();
                        let (name, _) = self.ident("state-field-name")?;
                        self.expect_p(":")?;
                        let (ty, _) = self.ident("type")?;
                        let mut variants = Vec::new();
                        let mut fields = Vec::new();
                        let mut max = None;
                        if ty == "list" {
                            self.expect_p("(")?;
                            loop {
                                let (fname, _) = self.ident("field-name")?;
                                self.expect_p(":")?;
                                let (fty, _) = self.ident("field-type")?;
                                fields.push((fname, fty));
                                if !self.eat_p(",") {
                                    break;
                                }
                            }
                            self.expect_p(")")?;
                            if matches!(self.peek(), Tok::Ident(w) if w == "max") {
                                self.bump();
                                self.expect_p("=")?;
                                match self.peek().clone() {
                                    Tok::Num(n) => {
                                        let sp2 = self.span();
                                        self.bump();
                                        max = Some(parse_int(&n, sp2)? as u32);
                                    }
                                    _ => return Err(self.err_expected("max-length")),
                                }
                            }
                        }
                        if ty == "enum" {
                            self.expect_p("(")?;
                            loop {
                                variants.push(self.ident("variant")?.0);
                                if !self.eat_p(",") {
                                    break;
                                }
                            }
                            self.expect_p(")")?;
                        }
                        let mut init = None;
                        if self.eat_p("=") {
                            let isp = self.span();
                            if ty == "list" {
                                init = Some((self.rows()?, isp));
                            } else {
                                init = Some((self.atom()?, isp));
                            }
                        }
                        self.end_stmt()?;
                        m.state.push(RawState { name, ty, variants, fields, max, init, span: dsp });
                    }
                    self.expect_p("}")?;
                }
                Some("derive") => {
                    self.bump();
                    let (name, _) = self.ident("derived-name")?;
                    self.expect_p("=")?;
                    let e = self.expr()?;
                    self.end_stmt()?;
                    m.derived.push((name, e, sp));
                }
                Some("effect") => {
                    self.bump();
                    let (name, _) = self.ident("effect-name")?;
                    match self.peek().clone() {
                        Tok::Ident(u) if u == "uses" => {
                            self.bump();
                        }
                        _ => return Err(self.err_expected("'uses'")),
                    }
                    let (cap, _) = self.ident("capability-id")?;
                    self.end_stmt()?;
                    m.effects.push((name, cap, sp));
                }
                Some("strings") => {
                    self.bump();
                    let (locale, _) = self.ident("locale")?;
                    self.skip_nl();
                    self.expect_p("{")?;
                    let mut entries = Vec::new();
                    loop {
                        self.skip_nl();
                        if self.is_p("}") {
                            break;
                        }
                        let esp = self.span();
                        let (key, _) = self.ident("string-key")?;
                        let text = match self.peek().clone() {
                            Tok::Str(s) => {
                                self.bump();
                                s
                            }
                            _ => return Err(self.err_expected("string")),
                        };
                        // entries are self-delimiting (`key "text"`): several may share a line
                        if !matches!(self.peek(), Tok::Ident(_)) {
                            self.end_stmt()?;
                        }
                        entries.push((key, text, esp));
                    }
                    self.expect_p("}")?;
                    m.strings.push((locale, entries));
                }
                _ => {
                    m.nodes.push(self.node()?);
                    self.end_stmt()?;
                }
            }
        }
        Ok(m)
    }
}

/// Number literal in an expression: `12` -> Int, `4.99` -> Dec(499) (at most 2 fraction digits).
pub fn parse_num_expr(s: &str, span: Span) -> R<Expr> {
    match s.split_once('.') {
        None => Ok(Expr::Int(parse_int(s, span)?)),
        Some((i, f)) => {
            if f.len() > 2 {
                return Err(Diagnostic::new(E_PARSE, "parse.bad-number")
                    .at_span(span)
                    .param("found", s)
                    .param("expected", "at most 2 decimal places"));
            }
            let ip = parse_int(i, span)?;
            let fp: i64 = if f.len() == 1 { parse_int(f, span)? * 10 } else { parse_int(f, span)? };
            Ok(Expr::Dec(ip * 100 + fp))
        }
    }
}

pub fn parse_int(s: &str, span: Span) -> R<i64> {
    s.parse::<i64>().map_err(|_| {
        Diagnostic::new(E_PARSE, "parse.bad-number")
            .at_span(span)
            .param("found", s)
            .param("expected", "integer")
    })
}

/// Parse terse text to the raw tree.
pub fn parse_raw(src: &str) -> R<RawModule> {
    let t = lex(src)?;
    let mut p = P { t, i: 0 };
    p.module()
}
