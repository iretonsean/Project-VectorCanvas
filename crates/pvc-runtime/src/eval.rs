//! Behaviour interpreter (G4 §7, v0 subset as defined by dev-ir's AST): typed state, derived pure
//! expressions, guards, assignments, effect emission to a logging dispatcher stub. No floats, no
//! network, no clock. Total: no loops, no recursion beyond the expression tree.

use pvc_ir::ast::{Action, CmpOp, Expr, Func, Handler, Event, Module, Value};
use pvc_ir::pred;
use std::collections::HashMap;

/// One dispatched (or refused) effect. The dispatcher is a logging stub: nothing leaves the process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectRecord {
    pub seq: u64,
    pub effect: String,
    pub capability: String,
    /// `true` when the capability is declared in the manifest; the stub still only logs.
    pub permitted: bool,
}

pub struct Behaviour {
    pub state: HashMap<String, Value>,
    pub derived: HashMap<String, Value>,
    pub effects: Vec<EffectRecord>,
    seq: u64,
}

impl Behaviour {
    pub fn new(m: &Module) -> Behaviour {
        let mut b = Behaviour { state: HashMap::new(), derived: HashMap::new(), effects: vec![], seq: 0 };
        for s in &m.state {
            b.state.insert(s.name.clone(), s.init.clone());
        }
        b.recompute(m);
        b
    }

    pub fn recompute(&mut self, m: &Module) {
        self.derived.clear();
        for d in &m.derived {
            let v = self.eval(&d.expr);
            self.derived.insert(d.name.clone(), v);
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.derived.get(name).or_else(|| self.state.get(name))
    }

    pub fn text(&self, name: &str) -> String {
        match self.get(name) {
            Some(Value::Text(s)) => s.clone(),
            _ => String::new(),
        }
    }

    pub fn truthy(&self, e: &Expr) -> bool {
        matches!(self.eval(e), Value::Bool(true))
    }

    pub fn eval(&self, e: &Expr) -> Value {
        match e {
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Int(i) => Value::Int(*i),
            Expr::Str(s) => Value::Text(s.clone()),
            Expr::Var(v) => self.get(v).cloned().unwrap_or(Value::Bool(false)),
            Expr::Variant(v) => Value::Enum(v.clone()),
            Expr::Not(a) => Value::Bool(!matches!(self.eval(a), Value::Bool(true))),
            Expr::And(a, b) => Value::Bool(self.truthy(a) && self.truthy(b)),
            Expr::Or(a, b) => Value::Bool(self.truthy(a) || self.truthy(b)),
            Expr::Cmp(op, a, b) => {
                let (x, y) = (self.eval(a), self.eval(b));
                let ord = match (&x, &y) {
                    (Value::Int(p), Value::Int(q)) => Some(p.cmp(q)),
                    (Value::Text(p), Value::Text(q)) => Some(p.cmp(q)),
                    (Value::Bool(p), Value::Bool(q)) => Some(p.cmp(q)),
                    (Value::Enum(p), Value::Enum(q)) => Some(p.cmp(q)),
                    _ => None,
                };
                use std::cmp::Ordering::*;
                Value::Bool(match (op, ord) {
                    (CmpOp::Eq, Some(o)) => o == Equal,
                    (CmpOp::Ne, Some(o)) => o != Equal,
                    (CmpOp::Lt, Some(o)) => o == Less,
                    (CmpOp::Le, Some(o)) => o != Greater,
                    (CmpOp::Gt, Some(o)) => o == Greater,
                    (CmpOp::Ge, Some(o)) => o != Less,
                    (CmpOp::Ne, None) => true,
                    _ => false,
                })
            }
            Expr::Dec(_) | Expr::Add(..) | Expr::Sub(..) | Expr::Mul(..) => Value::Bool(false), // refused at load (see support::check)
            Expr::Call(f, args) => {
                let a: Vec<Value> = args.iter().map(|x| self.eval(x)).collect();
                let s0 = match a.first() {
                    Some(Value::Text(s)) => s.as_str(),
                    _ => "",
                };
                match f {
                    Func::IsEmpty => Value::Bool(pred::is_empty(s0)),
                    Func::Length => Value::Int(pred::length(s0)),
                    Func::IsEmail => Value::Bool(pred::is_email(s0)),
                    Func::Count | Func::Sum | Func::Percent => Value::Bool(false), // refused at load
                    Func::Contains => {
                        let s1 = match a.get(1) {
                            Some(Value::Text(s)) => s.as_str(),
                            _ => "",
                        };
                        Value::Bool(s0.contains(s1))
                    }
                }
            }
        }
    }

    /// Run the handlers of `event` on a node. Returns true if state or effects changed.
    pub fn run_handlers(&mut self, m: &Module, handlers: &[Handler], event: Event, submit_target: &mut Vec<String>) -> bool {
        let mut changed = false;
        for h in handlers.iter().filter(|h| h.event == event) {
            if let Some(g) = &h.guard {
                if !self.truthy(g) {
                    continue;
                }
            }
            for a in &h.actions {
                match a {
                    Action::Assign(f, e) => {
                        let v = self.eval(e);
                        if self.state.get(f) != Some(&v) {
                            self.state.insert(f.clone(), v);
                            changed = true;
                        }
                        self.recompute(m);
                    }
                    Action::Emit(name) => {
                        self.seq += 1;
                        let cap = m.effects.iter().find(|e| &e.name == name).map(|e| e.cap.clone());
                        let permitted =
                            cap.as_ref().map(|c| m.manifest.caps.iter().any(|k| &k.id == c)).unwrap_or(false);
                        self.effects.push(EffectRecord {
                            seq: self.seq,
                            effect: name.clone(),
                            capability: cap.unwrap_or_default(),
                            permitted,
                        });
                        changed = true;
                    }
                    Action::Submit(id) => submit_target.push(id.clone()),
                    Action::Append(..) | Action::Remove(_) | Action::OpenUrl(_) => {} // refused at load
                }
            }
        }
        changed
    }
}
