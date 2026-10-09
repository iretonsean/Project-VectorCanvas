//! Canonical serialisation (G4 §3 "Canonical ordering", §14 D1).
//!
//! One byte form: compact JSON, fixed field order (the order written by `module_to_val`), integers
//! only (no floats; lengths are 1/64 px), strings NFC, minimal escapes (RFC 8785 style). The
//! reader is strict: it accepts exactly the shapes the writer produces, in the same order, and the
//! loader additionally re-serialises and compares bytes ([`crate::sign::load_signed`]).
//! Hand-rolled on purpose: no serde in the IR crate (D-10).

use crate::ast::*;
use crate::pred::nfc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonError(pub String);

impl std::fmt::Display for CanonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical form error: {}", self.0)
    }
}
impl std::error::Error for CanonError {}

fn err<T>(m: impl Into<String>) -> Result<T, CanonError> {
    Err(CanonError(m.into()))
}

// ---------------------------------------------------------------- generic value

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Val {
    Null,
    Bool(bool),
    Int(i64),
    Str(String),
    Arr(Vec<Val>),
    Obj(Vec<(String, Val)>),
}

fn write_str(out: &mut Vec<u8>, s: &str) {
    out.push(b'"');
    for ch in s.chars() {
        match ch {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{8}' => out.extend_from_slice(b"\\b"),
            '\u{c}' => out.extend_from_slice(b"\\f"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            c if (c as u32) < 0x20 => out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes()),
            c => {
                let mut b = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut b).as_bytes());
            }
        }
    }
    out.push(b'"');
}

pub fn write_val(v: &Val, out: &mut Vec<u8>) {
    match v {
        Val::Null => out.extend_from_slice(b"null"),
        Val::Bool(true) => out.extend_from_slice(b"true"),
        Val::Bool(false) => out.extend_from_slice(b"false"),
        Val::Int(i) => out.extend_from_slice(i.to_string().as_bytes()),
        Val::Str(s) => write_str(out, s),
        Val::Arr(a) => {
            out.push(b'[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_val(x, out);
            }
            out.push(b']');
        }
        Val::Obj(o) => {
            out.push(b'{');
            for (i, (k, x)) in o.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_str(out, k);
                out.push(b':');
                write_val(x, out);
            }
            out.push(b'}');
        }
    }
}

struct Rdr<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Rdr<'a> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }
    fn lit(&mut self, s: &str) -> Result<(), CanonError> {
        if self.b[self.i..].starts_with(s.as_bytes()) {
            self.i += s.len();
            Ok(())
        } else {
            err(format!("expected `{}` at byte {}", s, self.i))
        }
    }
    fn hex4(&mut self) -> Result<u32, CanonError> {
        if self.i + 4 > self.b.len() {
            return err("short \\u escape");
        }
        let s = std::str::from_utf8(&self.b[self.i..self.i + 4]).map_err(|_| CanonError("bad \\u".into()))?;
        self.i += 4;
        u32::from_str_radix(s, 16).map_err(|_| CanonError("bad \\u".into()))
    }
    fn string(&mut self) -> Result<String, CanonError> {
        self.lit("\"")?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let c = self.peek().ok_or_else(|| CanonError("unterminated string".into()))?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = self.peek().ok_or_else(|| CanonError("bad escape".into()))?;
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let mut cp = self.hex4()?;
                            if (0xD800..0xDC00).contains(&cp) {
                                self.lit("\\u")?;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return err("bad surrogate pair");
                                }
                                cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                            }
                            let ch = char::from_u32(cp).ok_or_else(|| CanonError("bad codepoint".into()))?;
                            let mut b = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
                        }
                        _ => return err("unknown escape"),
                    }
                }
                c if c < 0x20 => return err("raw control char in string"),
                c => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|_| CanonError("invalid utf-8".into()))
    }
    fn value(&mut self, depth: usize) -> Result<Val, CanonError> {
        if depth > 256 {
            return err("nesting too deep");
        }
        match self.peek() {
            Some(b'n') => self.lit("null").map(|_| Val::Null),
            Some(b't') => self.lit("true").map(|_| Val::Bool(true)),
            Some(b'f') => self.lit("false").map(|_| Val::Bool(false)),
            Some(b'"') => self.string().map(Val::Str),
            Some(b'[') => {
                self.i += 1;
                let mut a = Vec::new();
                if self.peek() == Some(b']') {
                    self.i += 1;
                    return Ok(Val::Arr(a));
                }
                loop {
                    a.push(self.value(depth + 1)?);
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            break;
                        }
                        _ => return err(format!("expected , or ] at byte {}", self.i)),
                    }
                }
                Ok(Val::Arr(a))
            }
            Some(b'{') => {
                self.i += 1;
                let mut o = Vec::new();
                if self.peek() == Some(b'}') {
                    self.i += 1;
                    return Ok(Val::Obj(o));
                }
                loop {
                    let k = self.string()?;
                    self.lit(":")?;
                    o.push((k, self.value(depth + 1)?));
                    match self.peek() {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            break;
                        }
                        _ => return err(format!("expected , or }} at byte {}", self.i)),
                    }
                }
                Ok(Val::Obj(o))
            }
            Some(c) if c == b'-' || c.is_ascii_digit() => {
                let st = self.i;
                if c == b'-' {
                    self.i += 1;
                }
                while matches!(self.peek(), Some(d) if d.is_ascii_digit()) {
                    self.i += 1;
                }
                let s = std::str::from_utf8(&self.b[st..self.i]).unwrap();
                if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
                    return err("floats are not allowed in canonical form");
                }
                let digits = s.trim_start_matches('-');
                if digits.is_empty() || (digits.len() > 1 && digits.starts_with('0')) || s == "-0" {
                    return err(format!("non-canonical integer `{}`", s));
                }
                s.parse::<i64>().map(Val::Int).map_err(|_| CanonError("integer out of range".into()))
            }
            _ => err(format!("unexpected byte at {}", self.i)),
        }
    }
}

pub fn parse_val(b: &[u8]) -> Result<Val, CanonError> {
    let mut r = Rdr { b, i: 0 };
    let v = r.value(0)?;
    if r.i != b.len() {
        return err("trailing bytes after value");
    }
    Ok(v)
}

// ---------------------------------------------------------------- strict object reader

struct O {
    f: Vec<(String, Val)>,
    pos: usize,
    ctx: &'static str,
}

impl O {
    fn new(v: Val, ctx: &'static str) -> Result<O, CanonError> {
        match v {
            Val::Obj(f) => Ok(O { f, pos: 0, ctx }),
            _ => err(format!("{}: expected object", ctx)),
        }
    }
    fn get(&mut self, name: &str) -> Result<Val, CanonError> {
        match self.f.get_mut(self.pos) {
            Some((k, v)) if k == name => {
                self.pos += 1;
                Ok(std::mem::replace(v, Val::Null))
            }
            Some((k, _)) => err(format!("{}: expected field `{}`, found `{}`", self.ctx, name, k)),
            None => err(format!("{}: missing field `{}`", self.ctx, name)),
        }
    }
    /// Take the next field if (and only if) it is named `name`.
    fn opt(&mut self, name: &str) -> Option<Val> {
        match self.f.get_mut(self.pos) {
            Some((k, v)) if k == name => {
                self.pos += 1;
                Some(std::mem::replace(v, Val::Null))
            }
            _ => None,
        }
    }
    fn done(self) -> Result<(), CanonError> {
        if self.pos == self.f.len() {
            Ok(())
        } else {
            err(format!("{}: unexpected extra field `{}`", self.ctx, self.f[self.pos].0))
        }
    }
}

fn obj(v: Vec<(&str, Val)>) -> Val {
    Val::Obj(v.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}
fn s(x: &str) -> Val {
    Val::Str(nfc(x))
}
fn i(x: i64) -> Val {
    Val::Int(x)
}
fn arr<T>(xs: &[T], f: impl Fn(&T) -> Val) -> Val {
    Val::Arr(xs.iter().map(f).collect())
}
fn opt<T>(x: &Option<T>, f: impl Fn(&T) -> Val) -> Val {
    match x {
        None => Val::Null,
        Some(x) => f(x),
    }
}
fn as_str(v: Val, what: &str) -> Result<String, CanonError> {
    match v {
        Val::Str(s) => Ok(s),
        _ => err(format!("{}: expected string", what)),
    }
}
fn as_int(v: Val, what: &str) -> Result<i64, CanonError> {
    match v {
        Val::Int(i) => Ok(i),
        _ => err(format!("{}: expected integer", what)),
    }
}
fn as_u<T: TryFrom<i64>>(v: Val, what: &str) -> Result<T, CanonError> {
    T::try_from(as_int(v, what)?).map_err(|_| CanonError(format!("{}: integer out of range", what)))
}
fn as_bool(v: Val, what: &str) -> Result<bool, CanonError> {
    match v {
        Val::Bool(b) => Ok(b),
        _ => err(format!("{}: expected bool", what)),
    }
}
fn as_arr(v: Val, what: &str) -> Result<Vec<Val>, CanonError> {
    match v {
        Val::Arr(a) => Ok(a),
        _ => err(format!("{}: expected array", what)),
    }
}
fn as_vec<T>(v: Val, what: &str, f: impl Fn(Val) -> Result<T, CanonError>) -> Result<Vec<T>, CanonError> {
    as_arr(v, what)?.into_iter().map(f).collect()
}
fn as_opt<T>(v: Val, f: impl Fn(Val) -> Result<T, CanonError>) -> Result<Option<T>, CanonError> {
    match v {
        Val::Null => Ok(None),
        v => f(v).map(Some),
    }
}
fn en<T>(v: Val, what: &str, f: impl Fn(&str) -> Option<T>) -> Result<T, CanonError> {
    let st = as_str(v, what)?;
    f(&st).ok_or_else(|| CanonError(format!("{}: unknown value `{}`", what, st)))
}
fn pair(v: Val, what: &str) -> Result<(u16, u16), CanonError> {
    let a = as_arr(v, what)?;
    if a.len() != 2 {
        return err(format!("{}: expected 2 elements", what));
    }
    let mut it = a.into_iter();
    Ok((as_u(it.next().unwrap(), what)?, as_u(it.next().unwrap(), what)?))
}

// ---------------------------------------------------------------- to_val

fn color_to(c: &Color) -> Val {
    match c {
        Color::Token(t) => s(t),
        Color::Rgba(r, g, b, a) => Val::Str(format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)),
    }
}
fn color_from(v: Val) -> Result<Color, CanonError> {
    let st = as_str(v, "color")?;
    if let Some(h) = st.strip_prefix('#') {
        if h.len() != 8 || !h.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
            return err("color: bad hex");
        }
        let n = u32::from_str_radix(h, 16).unwrap();
        Ok(Color::Rgba((n >> 24) as u8, (n >> 16) as u8, (n >> 8) as u8, n as u8))
    } else {
        Ok(Color::Token(st))
    }
}

pub fn expr_to(e: &Expr) -> Val {
    match e {
        Expr::Bool(b) => Val::Bool(*b),
        Expr::Int(n) => Val::Int(*n),
        Expr::Str(t) => Val::Arr(vec![s("s"), s(t)]),
        Expr::Var(n) => Val::Arr(vec![s("v"), s(n)]),
        Expr::Variant(n) => Val::Arr(vec![s("e"), s(n)]),
        Expr::Dec(n) => Val::Arr(vec![s("d"), i(*n)]),
        Expr::Add(a, b) => Val::Arr(vec![s("+"), expr_to(a), expr_to(b)]),
        Expr::Sub(a, b) => Val::Arr(vec![s("-"), expr_to(a), expr_to(b)]),
        Expr::Mul(a, b) => Val::Arr(vec![s("*"), expr_to(a), expr_to(b)]),
        Expr::Not(a) => Val::Arr(vec![s("not"), expr_to(a)]),
        Expr::And(a, b) => Val::Arr(vec![s("and"), expr_to(a), expr_to(b)]),
        Expr::Or(a, b) => Val::Arr(vec![s("or"), expr_to(a), expr_to(b)]),
        Expr::Cmp(op, a, b) => Val::Arr(vec![s(op.as_str()), expr_to(a), expr_to(b)]),
        Expr::Call(f, args) => {
            let mut v = vec![s("call"), s(f.as_str())];
            v.extend(args.iter().map(expr_to));
            Val::Arr(v)
        }
    }
}
pub fn expr_from(v: Val) -> Result<Expr, CanonError> {
    match v {
        Val::Bool(b) => Ok(Expr::Bool(b)),
        Val::Int(n) => Ok(Expr::Int(n)),
        Val::Arr(a) => {
            let mut it = a.into_iter();
            let tag = as_str(it.next().ok_or_else(|| CanonError("empty expr".into()))?, "expr tag")?;
            let mut rest: Vec<Val> = it.collect();
            let bx = |rest: &mut Vec<Val>| -> Result<Box<Expr>, CanonError> {
                if rest.is_empty() {
                    return err("expr: missing operand");
                }
                Ok(Box::new(expr_from(rest.remove(0))?))
            };
            let r = match tag.as_str() {
                "s" | "v" | "e" => {
                    if rest.len() != 1 {
                        return err("expr: bad arity");
                    }
                    let t = as_str(rest.remove(0), "expr text")?;
                    match tag.as_str() {
                        "s" => Expr::Str(t),
                        "v" => Expr::Var(t),
                        _ => Expr::Variant(t),
                    }
                }
                "d" => {
                    if rest.len() != 1 {
                        return err("expr: bad arity");
                    }
                    Expr::Dec(as_int(rest.remove(0), "decimal")?)
                }
                "+" | "-" | "*" => {
                    let a = bx(&mut rest)?;
                    let b = bx(&mut rest)?;
                    match tag.as_str() {
                        "+" => Expr::Add(a, b),
                        "-" => Expr::Sub(a, b),
                        _ => Expr::Mul(a, b),
                    }
                }
                "not" => {
                    let a = bx(&mut rest)?;
                    Expr::Not(a)
                }
                "and" | "or" => {
                    let a = bx(&mut rest)?;
                    let b = bx(&mut rest)?;
                    if tag == "and" {
                        Expr::And(a, b)
                    } else {
                        Expr::Or(a, b)
                    }
                }
                "call" => {
                    if rest.is_empty() {
                        return err("expr: call without function");
                    }
                    let f = en(rest.remove(0), "func", Func::from_name)?;
                    let args = rest.into_iter().map(expr_from).collect::<Result<Vec<_>, _>>()?;
                    return Ok(Expr::Call(f, args));
                }
                op => match CmpOp::from_name(op) {
                    Some(op) => {
                        let a = bx(&mut rest)?;
                        let b = bx(&mut rest)?;
                        Expr::Cmp(op, a, b)
                    }
                    None => return err(format!("expr: unknown tag `{}`", op)),
                },
            };
            if !rest.is_empty() && !matches!(r, Expr::Call(..)) {
                return err("expr: extra operands");
            }
            Ok(r)
        }
        _ => err("expr: bad shape"),
    }
}

fn opt_expr_to(e: &Option<Expr>) -> Val {
    opt(e, expr_to)
}
fn opt_expr_from(v: Val) -> Result<Option<Expr>, CanonError> {
    as_opt(v, expr_from)
}

fn value_to(v: &Value) -> Val {
    match v {
        Value::Bool(b) => Val::Bool(*b),
        Value::Int(n) => Val::Int(*n),
        Value::Text(t) => Val::Arr(vec![s("t"), s(t)]),
        Value::Enum(e) => Val::Arr(vec![s("e"), s(e)]),
        Value::Decimal(n) => Val::Arr(vec![s("d"), i(*n)]),
        Value::List(rows) => {
            let mut a = vec![s("l")];
            for r in rows {
                a.push(Val::Arr(r.iter().map(|(f, v)| Val::Arr(vec![s(f), value_to(v)])).collect()));
            }
            Val::Arr(a)
        }
    }
}
fn value_from(v: Val) -> Result<Value, CanonError> {
    match v {
        Val::Bool(b) => Ok(Value::Bool(b)),
        Val::Int(n) => Ok(Value::Int(n)),
        Val::Arr(a) if !a.is_empty() => {
            let mut it = a.into_iter();
            let tag = as_str(it.next().unwrap(), "value tag")?;
            match tag.as_str() {
                "t" | "e" => {
                    let t = as_str(it.next().ok_or_else(|| CanonError("value: missing".into()))?, "value")?;
                    Ok(if tag == "t" { Value::Text(t) } else { Value::Enum(t) })
                }
                "d" => Ok(Value::Decimal(as_int(it.next().ok_or_else(|| CanonError("value: missing".into()))?, "decimal")?)),
                "l" => {
                    let mut rows = Vec::new();
                    for r in it {
                        let mut row = Vec::new();
                        for f in as_arr(r, "row")? {
                            let mut fi = as_arr(f, "field")?.into_iter();
                            let name = as_str(fi.next().ok_or_else(|| CanonError("field: empty".into()))?, "field name")?;
                            let v = value_from(fi.next().ok_or_else(|| CanonError("field: missing value".into()))?)?;
                            row.push((name, v));
                        }
                        rows.push(row);
                    }
                    Ok(Value::List(rows))
                }
                _ => err("value: bad tag"),
            }
        }
        _ => err("value: bad shape"),
    }
}

fn ty_to(t: &Ty) -> Val {
    match t {
        Ty::Bool => s("bool"),
        Ty::Int => s("int"),
        Ty::Text => s("text"),
        Ty::Enum(v) => {
            let mut a = vec![s("enum")];
            a.extend(v.iter().map(|x| s(x)));
            Val::Arr(a)
        }
        Ty::Decimal => s("decimal"),
        Ty::List { fields, max } => {
            let mut a = vec![s("list"), i(*max as i64)];
            a.extend(fields.iter().map(|(n, t)| Val::Arr(vec![s(n), ty_to(t)])));
            Val::Arr(a)
        }
    }
}
fn ty_from(v: Val) -> Result<Ty, CanonError> {
    match v {
        Val::Str(t) => match t.as_str() {
            "bool" => Ok(Ty::Bool),
            "int" => Ok(Ty::Int),
            "text" => Ok(Ty::Text),
            "decimal" => Ok(Ty::Decimal),
            _ => err("type: unknown"),
        },
        Val::Arr(a) => {
            let mut it = a.into_iter();
            let tag = as_str(it.next().ok_or_else(|| CanonError("type: empty".into()))?, "type")?;
            match tag.as_str() {
                "enum" => Ok(Ty::Enum(it.map(|x| as_str(x, "variant")).collect::<Result<_, _>>()?)),
                "list" => {
                    let max = as_u::<u32>(it.next().ok_or_else(|| CanonError("list: missing max".into()))?, "max")?;
                    let mut fields = Vec::new();
                    for f in it {
                        let mut fi = as_arr(f, "field")?.into_iter();
                        let n = as_str(fi.next().ok_or_else(|| CanonError("field: empty".into()))?, "field name")?;
                        let t = ty_from(fi.next().ok_or_else(|| CanonError("field: missing type".into()))?)?;
                        fields.push((n, t));
                    }
                    Ok(Ty::List { fields, max })
                }
                _ => err("type: bad array"),
            }
        }
        _ => err("type: bad shape"),
    }
}

fn action_to(a: &Action) -> Val {
    match a {
        Action::Assign(n, e) => Val::Arr(vec![s("set"), s(n), expr_to(e)]),
        Action::Append(l, fs) => Val::Arr(vec![
            s("append"),
            s(l),
            Val::Arr(fs.iter().map(|(f, e)| Val::Arr(vec![s(f), expr_to(e)])).collect()),
        ]),
        Action::Remove(a) => Val::Arr(vec![s("remove"), s(a)]),
        Action::OpenUrl(u) => Val::Arr(vec![s("open-url"), s(u)]),
        Action::Emit(n) => Val::Arr(vec![s("emit"), s(n)]),
        Action::Submit(id) => Val::Arr(vec![s("submit"), s(id)]),
    }
}
fn action_from(v: Val) -> Result<Action, CanonError> {
    let mut a = as_arr(v, "action")?.into_iter();
    let tag = as_str(a.next().ok_or_else(|| CanonError("action: empty".into()))?, "action")?;
    let mut next = |w: &str| a.next().ok_or_else(|| CanonError(format!("action: missing {}", w)));
    match tag.as_str() {
        "set" => {
            let n = as_str(next("target")?, "target")?;
            let e = expr_from(next("expr")?)?;
            Ok(Action::Assign(n, e))
        }
        "append" => {
            let l = as_str(next("list")?, "list")?;
            let mut fs = Vec::new();
            for f in as_arr(next("fields")?, "fields")? {
                let mut fi = as_arr(f, "field")?.into_iter();
                let n = as_str(fi.next().ok_or_else(|| CanonError("field: empty".into()))?, "field")?;
                let e = expr_from(fi.next().ok_or_else(|| CanonError("field: missing expr".into()))?)?;
                fs.push((n, e));
            }
            Ok(Action::Append(l, fs))
        }
        "remove" => Ok(Action::Remove(as_str(next("alias")?, "alias")?)),
        "open-url" => Ok(Action::OpenUrl(as_str(next("url")?, "url")?)),
        "emit" => Ok(Action::Emit(as_str(next("effect")?, "effect")?)),
        "submit" => Ok(Action::Submit(as_str(next("node")?, "node")?)),
        _ => err("action: unknown"),
    }
}

fn handler_to(h: &Handler) -> Val {
    obj(vec![
        ("event", s(h.event.as_str())),
        ("guard", opt_expr_to(&h.guard)),
        ("do", arr(&h.actions, action_to)),
    ])
}
fn handler_from(v: Val) -> Result<Handler, CanonError> {
    let mut o = O::new(v, "handler")?;
    let h = Handler {
        event: en(o.get("event")?, "event", Event::from_name)?,
        guard: opt_expr_from(o.get("guard")?)?,
        actions: as_vec(o.get("do")?, "do", action_from)?,
    };
    o.done()?;
    Ok(h)
}

fn size_to(z: &Size) -> Val {
    match z {
        Size::Hug => s("hug"),
        Size::Fill => s("fill"),
        Size::Px(l) => i(l.0 as i64),
    }
}
fn size_from(v: Val) -> Result<Size, CanonError> {
    match v {
        Val::Str(t) if t == "hug" => Ok(Size::Hug),
        Val::Str(t) if t == "fill" => Ok(Size::Fill),
        Val::Int(n) => Ok(Size::Px(Len(i32::try_from(n).map_err(|_| CanonError("size range".into()))?))),
        _ => err("size: bad shape"),
    }
}
fn len_from(v: Val, w: &str) -> Result<Len, CanonError> {
    Ok(Len(as_u::<i32>(v, w)?))
}

fn layout_to(l: &Layout) -> Val {
    let mut v = obj(vec![
        ("dir", s(l.dir.as_str())),
        ("gap", i(l.gap.0 as i64)),
        ("pad", Val::Arr(vec![i(l.pad.top.0 as i64), i(l.pad.right.0 as i64), i(l.pad.bottom.0 as i64), i(l.pad.left.0 as i64)])),
        ("align", s(l.align.as_str())),
        ("justify", s(l.justify.as_str())),
        ("w", size_to(&l.w)),
        ("h", size_to(&l.h)),
    ]);
    if l.dir == Dir::Grid {
        if let Val::Obj(o) = &mut v {
            o.push(("cols".to_string(), i(l.cols as i64)));
        }
    }
    v
}
fn layout_from(v: Val) -> Result<Layout, CanonError> {
    let mut o = O::new(v, "layout")?;
    let dir = en(o.get("dir")?, "dir", Dir::from_name)?;
    let gap = len_from(o.get("gap")?, "gap")?;
    let p = as_vec(o.get("pad")?, "pad", |x| len_from(x, "pad"))?;
    if p.len() != 4 {
        return err("pad: expected 4 values");
    }
    let l = Layout {
        dir,
        gap,
        pad: Edges { top: p[0], right: p[1], bottom: p[2], left: p[3] },
        align: en(o.get("align")?, "align", Align::from_name)?,
        justify: en(o.get("justify")?, "justify", Justify::from_name)?,
        w: size_from(o.get("w")?)?,
        h: size_from(o.get("h")?)?,
        cols: 0,
    };
    let mut l = l;
    if l.dir == Dir::Grid {
        l.cols = as_u(o.get("cols")?, "cols")?;
    }
    o.done()?;
    Ok(l)
}

fn edges_to(e: &Edges) -> Val {
    Val::Arr(vec![i(e.top.0 as i64), i(e.right.0 as i64), i(e.bottom.0 as i64), i(e.left.0 as i64)])
}
fn edges_from(v: Val) -> Result<Edges, CanonError> {
    let p = as_vec(v, "pad", |x| len_from(x, "pad"))?;
    if p.len() != 4 {
        return err("pad: expected 4 values");
    }
    Ok(Edges { top: p[0], right: p[1], bottom: p[2], left: p[3] })
}

fn at_to(a: &AtRule) -> Val {
    let p = &a.patch;
    let mut f: Vec<(&str, Val)> = Vec::new();
    if let Some(d) = p.dir {
        f.push(("dir", s(d.as_str())));
    }
    if let Some(c) = p.cols {
        f.push(("cols", i(c as i64)));
    }
    if let Some(g) = p.gap {
        f.push(("gap", i(g.0 as i64)));
    }
    if let Some(e) = &p.pad {
        f.push(("pad", edges_to(e)));
    }
    if let Some(x) = p.align {
        f.push(("align", s(x.as_str())));
    }
    if let Some(x) = p.justify {
        f.push(("justify", s(x.as_str())));
    }
    if let Some(z) = &p.w {
        f.push(("w", size_to(z)));
    }
    if let Some(z) = &p.h {
        f.push(("h", size_to(z)));
    }
    obj(vec![
        ("bp", s(a.bp.as_str())),
        ("patch", obj(f)),
        ("show", opt(&a.show, |b| Val::Bool(*b))),
    ])
}
fn at_from(v: Val) -> Result<AtRule, CanonError> {
    let mut o = O::new(v, "at")?;
    let bp = en(o.get("bp")?, "bp", Bp::from_name)?;
    let mut po = O::new(o.get("patch")?, "patch")?;
    let patch = LayoutPatch {
        dir: po.opt("dir").map(|v| en(v, "dir", Dir::from_name)).transpose()?,
        cols: po.opt("cols").map(|v| as_u::<u16>(v, "cols")).transpose()?,
        gap: po.opt("gap").map(|v| len_from(v, "gap")).transpose()?,
        pad: po.opt("pad").map(edges_from).transpose()?,
        align: po.opt("align").map(|v| en(v, "align", Align::from_name)).transpose()?,
        justify: po.opt("justify").map(|v| en(v, "justify", Justify::from_name)).transpose()?,
        w: po.opt("w").map(size_from).transpose()?,
        h: po.opt("h").map(size_from).transpose()?,
    };
    po.done()?;
    let show = as_opt(o.get("show")?, |v| as_bool(v, "show"))?;
    o.done()?;
    Ok(AtRule { bp, patch, show })
}

fn paint_to(p: &Paint) -> Val {
    obj(vec![
        ("fill", opt(&p.fill, color_to)),
        (
            "border",
            opt(&p.border, |b| obj(vec![("w", i(b.width.0 as i64)), ("color", color_to(&b.color))])),
        ),
        (
            "shape",
            match p.shape {
                Shape::Circle => s("circle"),
                Shape::RoundedRect(r) => i(r.0 as i64),
            },
        ),
    ])
}
fn paint_from(v: Val) -> Result<Paint, CanonError> {
    let mut o = O::new(v, "paint")?;
    let fill = as_opt(o.get("fill")?, color_from)?;
    let border = as_opt(o.get("border")?, |b| {
        let mut bo = O::new(b, "border")?;
        let r = Border { width: len_from(bo.get("w")?, "border.w")?, color: color_from(bo.get("color")?)? };
        bo.done()?;
        Ok(r)
    })?;
    let shape = match o.get("shape")? {
        Val::Str(t) if t == "circle" => Shape::Circle,
        Val::Int(n) => Shape::RoundedRect(Len(i32::try_from(n).map_err(|_| CanonError("shape range".into()))?)),
        _ => return err("shape: bad shape"),
    };
    o.done()?;
    Ok(Paint { fill, border, shape })
}

fn name_to(n: &Name) -> Val {
    match n {
        Name::Key(k) => Val::Arr(vec![s("key"), s(k)]),
        Name::FromNode(id) => Val::Arr(vec![s("node"), s(id)]),
    }
}
fn name_from(v: Val) -> Result<Name, CanonError> {
    let mut a = as_arr(v, "name")?.into_iter();
    let tag = as_str(a.next().ok_or_else(|| CanonError("name: empty".into()))?, "name")?;
    let val = as_str(a.next().ok_or_else(|| CanonError("name: missing".into()))?, "name")?;
    match tag.as_str() {
        "key" => Ok(Name::Key(val)),
        "node" => Ok(Name::FromNode(val)),
        _ => err("name: bad tag"),
    }
}

fn sem_to(m: &Semantics) -> Val {
    let mut v = obj(vec![
        ("role", s(m.role.as_str())),
        ("decorative", Val::Bool(m.decorative)),
        ("name", opt(&m.name, name_to)),
        ("desc", opt(&m.description, |d| s(d))),
        ("level", opt(&m.heading_level, |l| i(*l as i64))),
        ("live", s(m.live.as_str())),
        ("required", Val::Bool(m.required)),
        ("disabled", opt_expr_to(&m.disabled)),
        ("checked", opt_expr_to(&m.checked)),
        ("pressed", opt_expr_to(&m.pressed)),
        ("invalid", opt_expr_to(&m.invalid)),
    ]);
    if let (Some(sel), Val::Obj(o)) = (&m.selected, &mut v) {
        o.push(("selected".to_string(), expr_to(sel)));
    }
    if let Val::Obj(o) = &mut v {
        o.push(("labelled_by".to_string(), opt(&m.labelled_by, |x| s(x))));
        o.push(("described_by".to_string(), arr(&m.described_by, |x| s(x))));
        o.push(("error_message".to_string(), opt(&m.error_message, |x| s(x))));
    }
    v
}
fn sem_from(v: Val) -> Result<Semantics, CanonError> {
    let mut o = O::new(v, "sem")?;
    let r = Semantics {
        role: en(o.get("role")?, "role", Role::from_name)?,
        decorative: as_bool(o.get("decorative")?, "decorative")?,
        name: as_opt(o.get("name")?, name_from)?,
        description: as_opt(o.get("desc")?, |x| as_str(x, "desc"))?,
        heading_level: as_opt(o.get("level")?, |x| as_u::<u8>(x, "level"))?,
        live: en(o.get("live")?, "live", Live::from_name)?,
        required: as_bool(o.get("required")?, "required")?,
        disabled: opt_expr_from(o.get("disabled")?)?,
        checked: opt_expr_from(o.get("checked")?)?,
        pressed: opt_expr_from(o.get("pressed")?)?,
        invalid: opt_expr_from(o.get("invalid")?)?,
        selected: o.opt("selected").map(expr_from).transpose()?,
        labelled_by: as_opt(o.get("labelled_by")?, |x| as_str(x, "labelled_by"))?,
        described_by: as_vec(o.get("described_by")?, "described_by", |x| as_str(x, "described_by"))?,
        error_message: as_opt(o.get("error_message")?, |x| as_str(x, "error_message"))?,
    };
    o.done()?;
    Ok(r)
}

fn kind_data_to(k: &NodeKind) -> Val {
    match k {
        NodeKind::Surface | NodeKind::Box | NodeKind::Shape | NodeKind::Layer => Val::Null,
        NodeKind::Slot(p) => obj(vec![("on", expr_to(&p.on)), ("cases", arr(&p.cases, |c| s(c)))]),
        NodeKind::View(p) => obj(vec![
            ("list", s(&p.list)),
            ("alias", s(&p.alias)),
            ("filter", opt_expr_to(&p.filter)),
        ]),
        NodeKind::Text(t) => {
            let mut v = obj(vec![
                ("key", s(&t.key)),
                ("family", s(&t.family)),
                ("size", i(t.size.0 as i64)),
                ("weight", i(t.weight as i64)),
                ("line_height", i(t.line_height.0 as i64)),
                ("color", color_to(&t.color)),
                ("align", s(t.align.as_str())),
            ]);
            if !t.args.is_empty() || t.value.is_some() {
                if let Val::Obj(o) = &mut v {
                    o.push((
                        "args".to_string(),
                        arr(&t.args, |(n, e)| Val::Arr(vec![s(n), expr_to(e)])),
                    ));
                    o.push(("value".to_string(), opt_expr_to(&t.value)));
                }
            }
            v
        }
        NodeKind::TextInput(t) => obj(vec![
            ("variant", s(t.variant.as_str())),
            ("bind", s(&t.bind)),
            ("masked", expr_to(&t.masked)),
            ("family", s(&t.family)),
            ("size", i(t.size.0 as i64)),
            ("line_height", i(t.line_height.0 as i64)),
            ("color", color_to(&t.color)),
        ]),
    }
}
fn kind_from(name: &str, data: Val) -> Result<NodeKind, CanonError> {
    match name {
        "surface" | "box" | "shape" | "layer" => {
            if data != Val::Null {
                return err("node data: expected null");
            }
            Ok(match name {
                "surface" => NodeKind::Surface,
                "box" => NodeKind::Box,
                "shape" => NodeKind::Shape,
                _ => NodeKind::Layer,
            })
        }
        "slot" => {
            let mut o = O::new(data, "slot")?;
            let r = SlotProps {
                on: expr_from(o.get("on")?)?,
                cases: as_vec(o.get("cases")?, "cases", |x| as_str(x, "case"))?,
            };
            o.done()?;
            Ok(NodeKind::Slot(r))
        }
        "view" => {
            let mut o = O::new(data, "view")?;
            let r = ViewProps {
                list: as_str(o.get("list")?, "list")?,
                alias: as_str(o.get("alias")?, "alias")?,
                filter: opt_expr_from(o.get("filter")?)?,
            };
            o.done()?;
            Ok(NodeKind::View(r))
        }
        "text" => {
            let mut o = O::new(data, "text")?;
            let r = TextProps {
                key: as_str(o.get("key")?, "key")?,
                family: as_str(o.get("family")?, "family")?,
                size: len_from(o.get("size")?, "size")?,
                weight: as_u(o.get("weight")?, "weight")?,
                line_height: len_from(o.get("line_height")?, "line_height")?,
                color: color_from(o.get("color")?)?,
                align: en(o.get("align")?, "align", TextAlign::from_name)?,
                args: Vec::new(),
                value: None,
            };
            let mut r = r;
            if let Some(a) = o.opt("args") {
                r.args = as_vec(a, "args", |x| {
                    let mut it = as_arr(x, "arg")?.into_iter();
                    let n = as_str(it.next().ok_or_else(|| CanonError("arg: empty".into()))?, "arg name")?;
                    let e = expr_from(it.next().ok_or_else(|| CanonError("arg: missing".into()))?)?;
                    Ok((n, e))
                })?;
                r.value = opt_expr_from(o.get("value")?)?;
            }
            o.done()?;
            Ok(NodeKind::Text(r))
        }
        "text-input" => {
            let mut o = O::new(data, "text-input")?;
            let r = InputProps {
                variant: en(o.get("variant")?, "variant", InputVariant::from_name)?,
                bind: as_str(o.get("bind")?, "bind")?,
                masked: expr_from(o.get("masked")?)?,
                family: as_str(o.get("family")?, "family")?,
                size: len_from(o.get("size")?, "size")?,
                line_height: len_from(o.get("line_height")?, "line_height")?,
                color: color_from(o.get("color")?)?,
            };
            o.done()?;
            Ok(NodeKind::TextInput(r))
        }
        _ => err(format!("unknown node kind `{}`", name)),
    }
}

pub fn node_to(n: &Node) -> Val {
    let mut v = obj(vec![
        ("id", s(&n.id)),
        ("kind", s(n.kind.name())),
        ("data", kind_data_to(&n.kind)),
        ("layout", layout_to(&n.layout)),
        ("paint", paint_to(&n.paint)),
        ("sem", sem_to(&n.sem)),
        ("show", opt_expr_to(&n.show)),
        ("on", arr(&n.on, handler_to)),
    ]);
    if let Val::Obj(o) = &mut v {
        if !n.at.is_empty() {
            o.push(("at".to_string(), arr(&n.at, at_to)));
        }
        o.push(("children".to_string(), arr(&n.children, node_to)));
    }
    v
}
fn node_from(v: Val) -> Result<Node, CanonError> {
    let mut o = O::new(v, "node")?;
    let id = as_str(o.get("id")?, "id")?;
    let kname = as_str(o.get("kind")?, "kind")?;
    let data = o.get("data")?;
    let n = Node {
        id,
        kind: kind_from(&kname, data)?,
        layout: layout_from(o.get("layout")?)?,
        paint: paint_from(o.get("paint")?)?,
        sem: sem_from(o.get("sem")?)?,
        show: opt_expr_from(o.get("show")?)?,
        on: as_vec(o.get("on")?, "on", handler_from)?,
        at: match o.opt("at") {
            Some(a) => as_vec(a, "at", at_from)?,
            None => Vec::new(),
        },
        children: as_vec(o.get("children")?, "children", node_from)?,
    };
    o.done()?;
    Ok(n)
}

pub fn module_to_val(m: &Module) -> Val {
    let h = &m.header;
    let mf = &m.manifest;
    obj(vec![
        (
            "header",
            obj(vec![
                ("ir", Val::Arr(vec![i(h.ir.0 as i64), i(h.ir.1 as i64)])),
                ("std", Val::Arr(vec![i(h.std.0 as i64), i(h.std.1 as i64)])),
                ("dialect", opt(&h.dialect, |d| s(d))),
                ("min_runtime", Val::Arr(vec![i(h.min_runtime.0 as i64), i(h.min_runtime.1 as i64)])),
            ]),
        ),
        (
            "manifest",
            obj(vec![
                ("locales", arr(&mf.locales, |x| s(x))),
                (
                    "fonts",
                    arr(&mf.fonts, |f| {
                        obj(vec![
                            ("family", s(&f.family)),
                            ("weight", i(f.weight as i64)),
                            ("script", opt(&f.script, |x| s(x))),
                            ("sha256", s(&f.sha256)),
                            ("file", s(&f.file)),
                            ("index", i(f.index as i64)),
                        ])
                    }),
                ),
                (
                    "caps",
                    arr(&mf.caps, |c| {
                        obj(vec![
                            ("id", s(&c.id)),
                            ("kind", s(&c.kind)),
                            ("hosts", arr(&c.hosts, |x| s(x))),
                            ("methods", arr(&c.methods, |x| s(x))),
                            ("max_request_bytes", i(c.max_request_bytes as i64)),
                            ("max_daily_bytes", i(c.max_daily_bytes as i64)),
                        ])
                    }),
                ),
                (
                    "viewport",
                    Val::Arr(vec![i(mf.viewport.0 as i64), i(mf.viewport.1 as i64), i(mf.viewport.2 as i64), i(mf.viewport.3 as i64)]),
                ),
                ("text_scale", Val::Arr(vec![i(mf.text_scale.0 as i64), i(mf.text_scale.1 as i64)])),
            ]),
        ),
        (
            "theme",
            obj(vec![
                ("name", s(&m.theme.name)),
                (
                    "colors",
                    arr(&m.theme.colors, |(n, c)| {
                        Val::Arr(vec![s(n), i(c[0] as i64), i(c[1] as i64), i(c[2] as i64), i(c[3] as i64)])
                    }),
                ),
            ]),
        ),
        (
            "strings",
            arr(&m.strings.tables, |t| {
                obj(vec![
                    ("locale", s(&t.locale)),
                    ("entries", arr(&t.entries, |(k, v)| Val::Arr(vec![s(k), s(v)]))),
                ])
            }),
        ),
        (
            "state",
            arr(&m.state, |d| {
                obj(vec![("name", s(&d.name)), ("ty", ty_to(&d.ty)), ("init", value_to(&d.init))])
            }),
        ),
        (
            "derived",
            arr(&m.derived, |d| obj(vec![("name", s(&d.name)), ("expr", expr_to(&d.expr))])),
        ),
        (
            "effects",
            arr(&m.effects, |e| obj(vec![("name", s(&e.name)), ("cap", s(&e.cap))])),
        ),
        ("tree", node_to(&m.root)),
    ])
}

pub fn module_from_val(v: Val) -> Result<Module, CanonError> {
    let mut o = O::new(v, "module")?;
    let header = {
        let mut h = O::new(o.get("header")?, "header")?;
        let r = Header {
            ir: pair(h.get("ir")?, "ir")?,
            std: pair(h.get("std")?, "std")?,
            dialect: as_opt(h.get("dialect")?, |x| as_str(x, "dialect"))?,
            min_runtime: pair(h.get("min_runtime")?, "min_runtime")?,
        };
        h.done()?;
        r
    };
    let manifest = {
        let mut m = O::new(o.get("manifest")?, "manifest")?;
        let locales = as_vec(m.get("locales")?, "locales", |x| as_str(x, "locale"))?;
        let fonts = as_vec(m.get("fonts")?, "fonts", |f| {
            let mut fo = O::new(f, "font")?;
            let r = FontEntry {
                family: as_str(fo.get("family")?, "family")?,
                weight: as_u(fo.get("weight")?, "weight")?,
                script: as_opt(fo.get("script")?, |x| as_str(x, "script"))?,
                sha256: as_str(fo.get("sha256")?, "sha256")?,
                file: as_str(fo.get("file")?, "file")?,
                index: as_u(fo.get("index")?, "index")?,
            };
            fo.done()?;
            Ok(r)
        })?;
        let caps = as_vec(m.get("caps")?, "caps", |c| {
            let mut co = O::new(c, "cap")?;
            let r = Capability {
                id: as_str(co.get("id")?, "id")?,
                kind: as_str(co.get("kind")?, "kind")?,
                hosts: as_vec(co.get("hosts")?, "hosts", |x| as_str(x, "host"))?,
                methods: as_vec(co.get("methods")?, "methods", |x| as_str(x, "method"))?,
                max_request_bytes: as_u(co.get("max_request_bytes")?, "max_request_bytes")?,
                max_daily_bytes: as_u(co.get("max_daily_bytes")?, "max_daily_bytes")?,
            };
            co.done()?;
            Ok(r)
        })?;
        let vp = as_vec(m.get("viewport")?, "viewport", |x| as_u::<u32>(x, "viewport"))?;
        if vp.len() != 4 {
            return err("viewport: expected 4 values");
        }
        let r = Manifest {
            locales,
            fonts,
            caps,
            viewport: (vp[0], vp[1], vp[2], vp[3]),
            text_scale: pair(m.get("text_scale")?, "text_scale")?,
        };
        m.done()?;
        r
    };
    let theme = {
        let mut t = O::new(o.get("theme")?, "theme")?;
        let name = as_str(t.get("name")?, "name")?;
        let colors = as_vec(t.get("colors")?, "colors", |c| {
            let a = as_arr(c, "color row")?;
            if a.len() != 5 {
                return err("color row: expected 5 elements");
            }
            let mut it = a.into_iter();
            let n = as_str(it.next().unwrap(), "color name")?;
            let mut ch = [0u8; 4];
            for slot in ch.iter_mut() {
                *slot = as_u(it.next().unwrap(), "channel")?;
            }
            Ok((n, ch))
        })?;
        t.done()?;
        Theme { name, colors }
    };
    let strings = Strings {
        tables: as_vec(o.get("strings")?, "strings", |t| {
            let mut to = O::new(t, "locale table")?;
            let r = LocaleTable {
                locale: as_str(to.get("locale")?, "locale")?,
                entries: as_vec(to.get("entries")?, "entries", |e| {
                    let a = as_arr(e, "entry")?;
                    if a.len() != 2 {
                        return err("entry: expected [key, text]");
                    }
                    let mut it = a.into_iter();
                    Ok((as_str(it.next().unwrap(), "key")?, as_str(it.next().unwrap(), "text")?))
                })?,
            };
            to.done()?;
            Ok(r)
        })?,
    };
    let state = as_vec(o.get("state")?, "state", |d| {
        let mut so = O::new(d, "state decl")?;
        let r = StateDecl {
            name: as_str(so.get("name")?, "name")?,
            ty: ty_from(so.get("ty")?)?,
            init: value_from(so.get("init")?)?,
        };
        so.done()?;
        Ok(r)
    })?;
    let derived = as_vec(o.get("derived")?, "derived", |d| {
        let mut dd = O::new(d, "derived")?;
        let r = Derived { name: as_str(dd.get("name")?, "name")?, expr: expr_from(dd.get("expr")?)? };
        dd.done()?;
        Ok(r)
    })?;
    let effects = as_vec(o.get("effects")?, "effects", |e| {
        let mut eo = O::new(e, "effect")?;
        let r = Effect { name: as_str(eo.get("name")?, "name")?, cap: as_str(eo.get("cap")?, "cap")? };
        eo.done()?;
        Ok(r)
    })?;
    let root = node_from(o.get("tree")?)?;
    o.done()?;
    Ok(Module { header, manifest, theme, strings, state, derived, effects, root, provenance: Vec::new() })
}

/// The one canonical byte serialisation of a module (provenance excluded).
pub fn canonicalize(m: &Module) -> Vec<u8> {
    let mut out = Vec::new();
    write_val(&module_to_val(m), &mut out);
    out
}

/// Parse canonical bytes back to a module. Strict: field order, integer-only numbers.
/// Does **not** itself check byte-for-byte canonicality; [`crate::sign::load_signed`] does.
pub fn from_canonical(b: &[u8]) -> Result<Module, CanonError> {
    module_from_val(parse_val(b)?)
}
