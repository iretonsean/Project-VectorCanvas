//! Renders one signed artifact (`pvc-compile` output, test key) to a PNG on Tier 0 (vello_cpu), headless.
//! Usage: pvc_shot <app.pvca> <out.png> [width] [height]
//! Exit 0: written. Exit 3: the runtime does not support a construct in the app (message on stderr).
//! Exit 2: any other load or I/O failure.
use pvc_runtime::*;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("usage: pvc_shot <app.pvca> <out.png> [width] [height]");
        std::process::exit(2);
    }
    let w: f32 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(1280.0);
    let h: f32 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(800.0);
    let bytes = std::fs::read(&a[1]).unwrap_or_else(|e| {
        eprintln!("pvc_shot: {}: {e}", a[1]);
        std::process::exit(2)
    });
    let rt = Runtime::from_signed(
        &bytes,
        |b| pvc_ir::load_signed(b, &pvc_ir::sign::testkey::verifying_key()),
        Options { locale: "en".into(), width: w, height: h, scale: 1.0 },
    );
    let mut rt = match rt {
        Ok(rt) => rt,
        Err(e) => {
            let msg = format!("{e:?}");
            eprintln!("pvc_shot: {msg}");
            std::process::exit(if msg.contains("Unsupported") { 3 } else { 2 })
        }
    };
    rt.frame();
    std::fs::write(&a[2], render::render_cpu_png(rt.display_list())).unwrap_or_else(|e| {
        eprintln!("pvc_shot: {}: {e}", a[2]);
        std::process::exit(2)
    });
}
