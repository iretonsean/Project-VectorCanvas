//! Scope gate for A2: the runtime executes exactly the construct set L01/F01 need (Plan §2). Anything
//! beyond it is refused with a named reason instead of being silently mis-laid-out. The headless
//! extension for the pilot specimens (D-2, "B4-runtime") replaces this gate.

use crate::runtime::LoadError;
use pvc_ir::ast::*;

fn expr_ok(e: &Expr) -> Result<(), String> {
    match e {
        Expr::Dec(_) | Expr::Add(..) | Expr::Sub(..) | Expr::Mul(..) => Err("decimal/arithmetic expression".into()),
        Expr::Not(a) => expr_ok(a),
        Expr::And(a, b) | Expr::Or(a, b) | Expr::Cmp(_, a, b) => expr_ok(a).and(expr_ok(b)),
        Expr::Call(f, args) => {
            if matches!(f, Func::Count | Func::Sum | Func::Percent) {
                return Err(format!("function {}", f.as_str()));
            }
            args.iter().try_for_each(expr_ok)
        }
        _ => Ok(()),
    }
}

pub fn check(m: &Module) -> Result<(), LoadError> {
    let bad = |s: String| Err(LoadError::Unsupported(s));
    for d in &m.derived {
        if let Err(e) = expr_ok(&d.expr) {
            return bad(format!("derived {}: {e}", d.name));
        }
    }
    for s in &m.state {
        if matches!(s.ty, Ty::Decimal | Ty::List { .. }) {
            return bad(format!("state {}: decimal/list type", s.name));
        }
    }
    let mut res = Ok(());
    m.walk(&mut |n, _| {
        if res.is_err() {
            return;
        }
        let mut fail = |why: String| res = Err(LoadError::Unsupported(format!("node {}: {why}", n.id)));
        match &n.kind {
            NodeKind::Surface | NodeKind::Box | NodeKind::Text(_) | NodeKind::TextInput(_) => {}
            other => return fail(format!("kind {}", other.name())),
        }
        if n.layout.dir == Dir::Grid {
            return fail("grid layout".into());
        }
        if !n.at.is_empty() {
            return fail("breakpoint rules".into());
        }
        if let Some(e) = &n.show {
            if let Err(w) = expr_ok(e) {
                return fail(w);
            }
        }
        for h in &n.on {
            if let Some(g) = &h.guard {
                if let Err(w) = expr_ok(g) {
                    return fail(w);
                }
            }
            for a in &h.actions {
                match a {
                    Action::Assign(_, e) => {
                        if let Err(w) = expr_ok(e) {
                            return fail(w);
                        }
                    }
                    Action::Append(..) | Action::Remove(_) | Action::OpenUrl(_) => return fail("list/url action".into()),
                    _ => {}
                }
            }
        }
    });
    res
}
