//! Reference semantics for guards and handlers (G4 §7.3, D2): pure, total, integer-only.
//! Hosts/runtime may call this directly so there is exactly one implementation of the expression
//! language and of the predicates. No floats, no clock, no I/O.
//!
//! Decimals are fixed-point with 2 fraction digits (`Value::Decimal(hundredths)`); `percent(x, p)`
//! is `round-half-away-from-zero(x * p / 100)` computed in 128-bit integers.

use crate::ast::*;
use crate::pred;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub values: Vec<(String, Value)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalError {
    UnknownName(String),
    Type(String),
    Overflow,
}

/// Row bindings in force: `alias` (a `view` template variable) or `it` (inside `count`/`sum`) -> (list, row index).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scope {
    pub rows: Vec<(String, String, usize)>,
}

impl Scope {
    pub fn with(alias: &str, list: &str, index: usize) -> Scope {
        Scope { rows: vec![(alias.to_string(), list.to_string(), index)] }
    }
}

/// Result of running one handler.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Outcome {
    /// False if the guard evaluated to false (nothing ran).
    pub ran: bool,
    /// Effect names emitted, in order (to be routed to the dispatcher).
    pub emitted: Vec<String>,
    /// Nodes on which `submit` was raised, in order (the caller runs their `submit` handlers).
    pub submits: Vec<Id>,
    /// URLs passed to `open-url` (dispatcher stub only logs).
    pub opened: Vec<String>,
    /// Number of rows removed / appended (so a host knows the views changed).
    pub removed: usize,
    pub appended: usize,
}

impl State {
    pub fn init(m: &Module) -> State {
        State { values: m.state.iter().map(|d| (d.name.clone(), d.init.clone())).collect() }
    }
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.values.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }
    pub fn set(&mut self, name: &str, v: Value) -> bool {
        match self.values.iter_mut().find(|(n, _)| n == name) {
            Some(slot) => {
                slot.1 = v;
                true
            }
            None => false,
        }
    }
    /// Rows of a list state field.
    pub fn rows(&self, list: &str) -> Option<&Vec<Vec<(String, Value)>>> {
        match self.get(list) {
            Some(Value::List(r)) => Some(r),
            _ => None,
        }
    }
}

fn truthy(v: Value) -> Result<bool, EvalError> {
    match v {
        Value::Bool(b) => Ok(b),
        other => Err(EvalError::Type(format!("expected bool, got {:?}", other))),
    }
}

fn text(v: Value) -> Result<String, EvalError> {
    match v {
        Value::Text(s) => Ok(s),
        other => Err(EvalError::Type(format!("expected text, got {:?}", other))),
    }
}

/// Promote an int to a decimal when mixed with a decimal.
fn promote(a: Value, b: Value) -> (Value, Value) {
    match (&a, &b) {
        (Value::Int(i), Value::Decimal(_)) => (Value::Decimal(i.saturating_mul(100)), b),
        (Value::Decimal(_), Value::Int(i)) => {
            let i = i.saturating_mul(100);
            (a, Value::Decimal(i))
        }
        _ => (a, b),
    }
}

fn lookup_field(st: &State, list: &str, idx: usize, field: &str) -> Result<Value, EvalError> {
    let rows = st.rows(list).ok_or_else(|| EvalError::UnknownName(list.to_string()))?;
    let row = rows.get(idx).ok_or_else(|| EvalError::UnknownName(format!("{}[{}]", list, idx)))?;
    row.iter()
        .find(|(f, _)| f == field)
        .map(|(_, v)| v.clone())
        .ok_or_else(|| EvalError::UnknownName(format!("{}.{}", list, field)))
}

/// True if `e` (evaluated against rows of `list` bound to `it`) has decimal type: used to type empty sums.
fn static_decimal(m: &Module, list: &str, e: &Expr) -> bool {
    match e {
        Expr::Dec(_) => true,
        Expr::Var(n) => {
            if let Some(f) = n.strip_prefix("it.") {
                if let Some(StateDecl { ty: Ty::List { fields, .. }, .. }) = m.state.iter().find(|d| d.name == list) {
                    return fields.iter().any(|(fname, t)| fname == f && *t == Ty::Decimal);
                }
                return false;
            }
            if let Some(d) = m.state.iter().find(|d| &d.name == n) {
                return d.ty == Ty::Decimal;
            }
            m.derived.iter().find(|d| &d.name == n).map(|d| static_decimal(m, list, &d.expr)).unwrap_or(false)
        }
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) => static_decimal(m, list, a) || static_decimal(m, list, b),
        Expr::Call(Func::Percent, _) => true,
        Expr::Call(Func::Sum, args) if args.len() == 2 => match &args[0] {
            Expr::Var(l) => static_decimal(m, l, &args[1]),
            _ => false,
        },
        _ => false,
    }
}

fn list_name(e: &Expr) -> Result<&str, EvalError> {
    match e {
        Expr::Var(l) => Ok(l),
        _ => Err(EvalError::Type("expected a list name".into())),
    }
}

/// Evaluate with no row bindings.
pub fn eval(m: &Module, st: &State, e: &Expr) -> Result<Value, EvalError> {
    eval_in(m, st, &Scope::default(), e)
}

pub fn eval_in(m: &Module, st: &State, sc: &Scope, e: &Expr) -> Result<Value, EvalError> {
    match e {
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        Expr::Int(i) => Ok(Value::Int(*i)),
        Expr::Dec(d) => Ok(Value::Decimal(*d)),
        Expr::Str(s) => Ok(Value::Text(s.clone())),
        Expr::Variant(v) => Ok(Value::Enum(v.clone())),
        Expr::Var(n) => {
            if let Some((alias, field)) = n.split_once('.') {
                // innermost binding wins
                if let Some((_, list, idx)) = sc.rows.iter().rev().find(|(a, _, _)| a == alias) {
                    return lookup_field(st, list, *idx, field);
                }
                return Err(EvalError::UnknownName(n.clone()));
            }
            if let Some(v) = st.get(n) {
                return Ok(v.clone());
            }
            match m.derived.iter().find(|d| &d.name == n) {
                Some(d) => eval_in(m, st, sc, &d.expr),
                None => Err(EvalError::UnknownName(n.clone())),
            }
        }
        Expr::Not(a) => Ok(Value::Bool(!truthy(eval_in(m, st, sc, a)?)?)),
        Expr::And(a, b) => Ok(Value::Bool(truthy(eval_in(m, st, sc, a)?)? && truthy(eval_in(m, st, sc, b)?)?)),
        Expr::Or(a, b) => Ok(Value::Bool(truthy(eval_in(m, st, sc, a)?)? || truthy(eval_in(m, st, sc, b)?)?)),
        Expr::Add(a, b) | Expr::Sub(a, b) => {
            let (x, y) = promote(eval_in(m, st, sc, a)?, eval_in(m, st, sc, b)?);
            let add = matches!(e, Expr::Add(..));
            match (x, y) {
                (Value::Int(x), Value::Int(y)) => {
                    let r = if add { x.checked_add(y) } else { x.checked_sub(y) };
                    r.map(Value::Int).ok_or(EvalError::Overflow)
                }
                (Value::Decimal(x), Value::Decimal(y)) => {
                    let r = if add { x.checked_add(y) } else { x.checked_sub(y) };
                    r.map(Value::Decimal).ok_or(EvalError::Overflow)
                }
                _ => Err(EvalError::Type("+/- need two ints or two decimals".into())),
            }
        }
        Expr::Mul(a, b) => match (eval_in(m, st, sc, a)?, eval_in(m, st, sc, b)?) {
            (Value::Int(x), Value::Int(y)) => x.checked_mul(y).map(Value::Int).ok_or(EvalError::Overflow),
            (Value::Int(x), Value::Decimal(y)) | (Value::Decimal(y), Value::Int(x)) => {
                x.checked_mul(y).map(Value::Decimal).ok_or(EvalError::Overflow)
            }
            _ => Err(EvalError::Type("* needs int*int or int*decimal".into())),
        },
        Expr::Cmp(op, a, b) => {
            let (x, y) = promote(eval_in(m, st, sc, a)?, eval_in(m, st, sc, b)?);
            let r = match op {
                CmpOp::Eq => x == y,
                CmpOp::Ne => x != y,
                _ => {
                    let (x, y) = match (x, y) {
                        (Value::Int(x), Value::Int(y)) | (Value::Decimal(x), Value::Decimal(y)) => (x, y),
                        _ => return Err(EvalError::Type("ordered comparison needs ints or decimals".into())),
                    };
                    match op {
                        CmpOp::Lt => x < y,
                        CmpOp::Le => x <= y,
                        CmpOp::Gt => x > y,
                        _ => x >= y,
                    }
                }
            };
            Ok(Value::Bool(r))
        }
        Expr::Call(f, args) => eval_call(m, st, sc, *f, args),
    }
}

fn eval_call(m: &Module, st: &State, sc: &Scope, f: Func, args: &[Expr]) -> Result<Value, EvalError> {
    match (f, args.len()) {
        (Func::Count, 1) | (Func::Count, 2) => {
            let list = list_name(&args[0])?;
            let n = st.rows(list).ok_or_else(|| EvalError::UnknownName(list.to_string()))?.len();
            let mut c = 0i64;
            for i in 0..n {
                let keep = if args.len() == 2 {
                    let mut s2 = sc.clone();
                    s2.rows.push(("it".into(), list.to_string(), i));
                    truthy(eval_in(m, st, &s2, &args[1])?)?
                } else {
                    true
                };
                if keep {
                    c += 1;
                }
            }
            Ok(Value::Int(c))
        }
        (Func::Sum, 2) => {
            let list = list_name(&args[0])?;
            let n = st.rows(list).ok_or_else(|| EvalError::UnknownName(list.to_string()))?.len();
            let dec = static_decimal(m, list, &args[1]);
            let (mut si, mut sd) = (0i64, 0i64);
            for i in 0..n {
                let mut s2 = sc.clone();
                s2.rows.push(("it".into(), list.to_string(), i));
                match eval_in(m, st, &s2, &args[1])? {
                    Value::Int(x) => si = si.checked_add(x).ok_or(EvalError::Overflow)?,
                    Value::Decimal(x) => sd = sd.checked_add(x).ok_or(EvalError::Overflow)?,
                    other => return Err(EvalError::Type(format!("sum over {:?}", other))),
                }
            }
            Ok(if dec || sd != 0 { Value::Decimal(sd + si * 100) } else { Value::Int(si) })
        }
        (Func::Percent, 2) => {
            let x = match eval_in(m, st, sc, &args[0])? {
                Value::Decimal(x) => x as i128,
                other => return Err(EvalError::Type(format!("percent of {:?}", other))),
            };
            // percentage in hundredths of a percent: `8.25` -> 825, `10` -> 1000
            let p = match eval_in(m, st, sc, &args[1])? {
                Value::Decimal(p) => p as i128,
                Value::Int(p) => (p as i128) * 100,
                other => return Err(EvalError::Type(format!("percent by {:?}", other))),
            };
            let num = x * p;
            let half = 5000i128;
            let r = if num >= 0 { (num + half) / 10000 } else { -((-num + half) / 10000) };
            i64::try_from(r).map(Value::Decimal).map_err(|_| EvalError::Overflow)
        }
        (Func::IsEmpty, 1) => match eval_in(m, st, sc, &args[0])? {
            Value::Text(s) => Ok(Value::Bool(pred::is_empty(&s))),
            Value::List(r) => Ok(Value::Bool(r.is_empty())),
            other => Err(EvalError::Type(format!("is-empty of {:?}", other))),
        },
        (Func::Length, 1) => match eval_in(m, st, sc, &args[0])? {
            Value::Text(s) => Ok(Value::Int(pred::length(&s))),
            Value::List(r) => Ok(Value::Int(r.len() as i64)),
            other => Err(EvalError::Type(format!("length of {:?}", other))),
        },
        (Func::IsEmail, 1) => Ok(Value::Bool(pred::is_email(&text(eval_in(m, st, sc, &args[0])?)?))),
        (Func::Contains, 2) => {
            let n = text(eval_in(m, st, sc, &args[1])?)?;
            let h = text(eval_in(m, st, sc, &args[0])?)?;
            Ok(Value::Bool(h.contains(&n)))
        }
        _ => Err(EvalError::Type("bad call arity".into())),
    }
}

/// Evaluate a boolean expression.
pub fn eval_bool(m: &Module, st: &State, e: &Expr) -> Result<bool, EvalError> {
    truthy(eval(m, st, e)?)
}

pub fn eval_bool_in(m: &Module, st: &State, sc: &Scope, e: &Expr) -> Result<bool, EvalError> {
    truthy(eval_in(m, st, sc, e)?)
}

/// Text form of a value as shown in a text node hole: ints as digits, decimals with 2 fraction digits.
pub fn format_value(v: &Value) -> String {
    match v {
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Text(t) => t.clone(),
        Value::Enum(e) => e.clone(),
        Value::Decimal(d) => {
            let a = d.unsigned_abs();
            format!("{}{}.{:02}", if *d < 0 { "-" } else { "" }, a / 100, a % 100)
        }
        Value::List(r) => format!("[{} rows]", r.len()),
    }
}

/// The string a `text` node shows: its `value` expression verbatim, or its strings-table entry for
/// `locale` with `{name}` holes filled from `args` (`{{`/`}}` are literal braces). `None` if a name is
/// unknown or an expression fails.
pub fn text_content(m: &Module, st: &State, sc: &Scope, locale: &str, t: &TextProps) -> Option<String> {
    if let Some(e) = &t.value {
        return eval_in(m, st, sc, e).ok().map(|v| format_value(&v));
    }
    let tpl = m.string(locale, &t.key)?;
    let mut out = String::new();
    let mut it = tpl.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '{' if it.peek() == Some(&'{') => {
                it.next();
                out.push('{');
            }
            '}' if it.peek() == Some(&'}') => {
                it.next();
                out.push('}');
            }
            '{' => {
                let mut name = String::new();
                loop {
                    match it.next() {
                        Some('}') => break,
                        Some(ch) => name.push(ch),
                        None => return None,
                    }
                }
                let (_, e) = t.args.iter().find(|(n, _)| *n == name)?;
                out.push_str(&format_value(&eval_in(m, st, sc, e).ok()?));
            }
            c => out.push(c),
        }
    }
    Some(out)
}

/// Names of the `{holes}` in a template string; `Err(())` if braces are unbalanced.
pub fn template_holes(tpl: &str) -> Result<Vec<String>, ()> {
    let mut holes = Vec::new();
    let mut it = tpl.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '{' if it.peek() == Some(&'{') => {
                it.next();
            }
            '}' if it.peek() == Some(&'}') => {
                it.next();
            }
            '{' => {
                let mut name = String::new();
                loop {
                    match it.next() {
                        Some('}') => break,
                        Some(ch) => name.push(ch),
                        None => return Err(()),
                    }
                }
                holes.push(name);
            }
            '}' => return Err(()),
            _ => {}
        }
    }
    Ok(holes)
}

fn default_for(t: &Ty) -> Value {
    match t {
        Ty::Bool => Value::Bool(false),
        Ty::Int => Value::Int(0),
        Ty::Text => Value::Text(String::new()),
        Ty::Decimal => Value::Decimal(0),
        Ty::Enum(v) => Value::Enum(v.first().cloned().unwrap_or_default()),
        Ty::List { .. } => Value::List(Vec::new()),
    }
}

/// Run one handler against `st` with no row bindings.
pub fn run_handler(m: &Module, st: &mut State, h: &Handler) -> Result<Outcome, EvalError> {
    run_handler_in(m, st, &Scope::default(), h)
}

/// Run one handler (guard, then actions in order; effects/submits/URLs are reported, not executed).
/// Assignments see the effect of earlier ones. `remove alias` / `alias.field = …` use `sc`.
pub fn run_handler_in(m: &Module, st: &mut State, sc: &Scope, h: &Handler) -> Result<Outcome, EvalError> {
    let mut out = Outcome::default();
    if let Some(g) = &h.guard {
        if !eval_bool_in(m, st, sc, g)? {
            return Ok(out);
        }
    }
    out.ran = true;
    let mut sc = sc.clone();
    for a in &h.actions {
        match a {
            Action::Assign(n, e) => {
                let v = eval_in(m, st, &sc, e)?;
                if let Some((alias, field)) = n.split_once('.') {
                    let (_, list, idx) = sc
                        .rows
                        .iter()
                        .rev()
                        .find(|(x, _, _)| x == alias)
                        .cloned()
                        .ok_or_else(|| EvalError::UnknownName(n.clone()))?;
                    let rows = match st.values.iter_mut().find(|(k, _)| *k == list) {
                        Some((_, Value::List(r))) => r,
                        _ => return Err(EvalError::UnknownName(list)),
                    };
                    let row = rows.get_mut(idx).ok_or_else(|| EvalError::UnknownName(n.clone()))?;
                    let slot = row.iter_mut().find(|(f, _)| f == field).ok_or_else(|| EvalError::UnknownName(n.clone()))?;
                    slot.1 = v;
                } else if !st.set(n, v) {
                    return Err(EvalError::UnknownName(n.clone()));
                }
            }
            Action::Append(list, fields) => {
                let decl = m.state.iter().find(|d| &d.name == list).ok_or_else(|| EvalError::UnknownName(list.clone()))?;
                let (decl_fields, max) = match &decl.ty {
                    Ty::List { fields, max } => (fields, *max),
                    _ => return Err(EvalError::Type(format!("{} is not a list", list))),
                };
                let mut row = Vec::new();
                for (fname, fty) in decl_fields {
                    let v = match fields.iter().find(|(n, _)| n == fname) {
                        Some((_, e)) => eval_in(m, st, &sc, e)?,
                        None => default_for(fty),
                    };
                    row.push((fname.clone(), v));
                }
                if let Some((_, Value::List(rows))) = st.values.iter_mut().find(|(k, _)| k == list) {
                    if (rows.len() as u32) < max {
                        rows.push(row);
                        out.appended += 1;
                    }
                }
            }
            Action::Remove(alias) => {
                let (_, list, idx) = sc
                    .rows
                    .iter()
                    .rev()
                    .find(|(x, _, _)| x == alias)
                    .cloned()
                    .ok_or_else(|| EvalError::UnknownName(alias.clone()))?;
                if let Some((_, Value::List(rows))) = st.values.iter_mut().find(|(k, _)| *k == list) {
                    if idx < rows.len() {
                        rows.remove(idx);
                        out.removed += 1;
                    }
                }
                // the row is gone: later actions of this handler can no longer address it
                sc.rows.retain(|(x, _, _)| x != alias);
            }
            Action::Emit(e) => out.emitted.push(e.clone()),
            Action::Submit(id) => out.submits.push(id.clone()),
            Action::OpenUrl(u) => out.opened.push(u.clone()),
        }
    }
    Ok(out)
}
