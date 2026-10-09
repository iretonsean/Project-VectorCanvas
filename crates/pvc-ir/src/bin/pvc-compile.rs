//! `pvc-compile <in.pvc> <out.pvca> [--canonical <out.json>]`: compile terse text, sign the canonical
//! bytes with the committed TEST key, write the signed artifact. Build-time tool: not part of the runtime.

use pvc_ir::sign::{digest_hex, testkey};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("usage: pvc-compile <in.pvc> <out.pvca> [--canonical <out.json>]");
        std::process::exit(2);
    }
    let src = std::fs::read_to_string(&args[0]).expect("read input");
    let compiled = match pvc_ir::compile_terse(&src) {
        Ok(c) => c,
        Err(d) => {
            for x in &d {
                println!("{}", x.render());
            }
            std::process::exit(1);
        }
    };
    for x in &compiled.diagnostics {
        println!("{}", x.render());
    }
    let canon = pvc_ir::canonicalize(&compiled.module);
    let art = pvc_ir::sign(&compiled.module, &testkey::signing_key());
    std::fs::write(&args[1], &art).expect("write artifact");
    if let Some(i) = args.iter().position(|a| a == "--canonical") {
        std::fs::write(&args[i + 1], &canon).expect("write canonical");
    }
    println!("canonical_bytes={} sha256={} signer={}", canon.len(), digest_hex(&canon), testkey::public_hex());
}
