//! `pvc-validate [--json] <file.pvc>`: parse + validate terse PVC-IR, print diagnostics (one per line,
//! stable format; or one JSON object with `--json`), exit 0 if there are no errors, 1 if there are errors,
//! 2 on I/O trouble. No network.

use pvc_ir::canon::{write_val, Val};

fn s(x: &str) -> Val {
    Val::Str(x.to_string())
}

fn main() {
    let mut json = false;
    let mut path = None;
    for a in std::env::args().skip(1) {
        if a == "--json" {
            json = true;
        } else {
            path = Some(a);
        }
    }
    let path = match path {
        Some(p) => p,
        None => {
            eprintln!("usage: pvc-validate [--json] <file.pvc>");
            std::process::exit(2);
        }
    };
    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pvc-validate: cannot read {}: {}", path, e);
            std::process::exit(2);
        }
    };
    let diags = match pvc_ir::compile_terse(&src) {
        Ok(c) => c.diagnostics,
        Err(d) => d,
    };
    let errs = diags.iter().filter(|d| d.is_error()).count();
    let warns = diags.iter().filter(|d| d.severity == pvc_ir::Severity::Warning).count();
    if json {
        let list: Vec<Val> = diags
            .iter()
            .map(|d| {
                let sev = match d.severity {
                    pvc_ir::Severity::Error => "error",
                    pvc_ir::Severity::Warning => "warning",
                    pvc_ir::Severity::Info => "info",
                };
                Val::Obj(vec![
                    ("code".into(), s(d.code)),
                    ("severity".into(), s(sev)),
                    ("line".into(), d.location.span.map(|p| Val::Int(p.line as i64)).unwrap_or(Val::Null)),
                    ("col".into(), d.location.span.map(|p| Val::Int(p.col as i64)).unwrap_or(Val::Null)),
                    ("node".into(), d.location.node.as_deref().map(s).unwrap_or(Val::Null)),
                    ("path".into(), s(&d.location.path)),
                    ("message_key".into(), s(d.message_key)),
                    ("params".into(), Val::Obj(d.params.iter().map(|(k, v)| (k.clone(), s(v))).collect())),
                    ("message".into(), s(&d.message())),
                    ("fix_hint".into(), s(&d.fix_hint())),
                    ("text".into(), s(&d.render())),
                ])
            })
            .collect();
        let v = Val::Obj(vec![
            ("ok".into(), Val::Bool(errs == 0)),
            ("errors".into(), Val::Int(errs as i64)),
            ("warnings".into(), Val::Int(warns as i64)),
            ("diagnostics".into(), Val::Arr(list)),
        ]);
        let mut out = Vec::new();
        write_val(&v, &mut out);
        println!("{}", String::from_utf8(out).unwrap());
    } else {
        for d in &diags {
            println!("{}", d.render());
        }
        if errs == 0 {
            println!("OK errors=0 warnings={}", warns);
        } else {
            println!("FAIL errors={} warnings={}", errs, warns);
        }
    }
    if errs > 0 {
        std::process::exit(1);
    }
}
