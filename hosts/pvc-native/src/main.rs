use pvc_native::gpu::Renderer;
use pvc_native::{app, loader};
use pvc_runtime::{Options, Runtime};
use std::time::Instant;

fn main() {
    let t_start = Instant::now();
    let rt_start_ns = pvc_native::trace::realtime_ns();
    let mut artifact = None;
    let mut locale = "en".to_string();
    let mut renderer = Renderer::Gl;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--locale" => locale = args.next().unwrap_or_default(),
            "--renderer" => renderer = match args.next().as_deref() { Some("gl") => Renderer::Gl, Some("cpu") => Renderer::Cpu, o => { eprintln!("pvc-native: --renderer gl|cpu, got {o:?}"); std::process::exit(2) } },
            "-h" | "--help" => {
                println!("usage: pvc-native <signed-artifact> [--locale en|zh-CN] [--renderer gl|cpu]");
                return;
            }
            _ if artifact.is_none() => artifact = Some(a),
            _ => {
                eprintln!("pvc-native: unexpected argument {a}");
                std::process::exit(2);
            }
        }
    }
    let Some(artifact) = artifact else {
        eprintln!("usage: pvc-native <signed-artifact> [--locale en|zh-CN] [--renderer gl|cpu]");
        std::process::exit(2);
    };
    if locale != "en" && locale != "zh-CN" {
        eprintln!("pvc-native: unsupported locale {locale} (en|zh-CN)");
        std::process::exit(2);
    }
    // verify-on-load: exit 3 on any refusal, before a window or GPU object exists
    let loaded = match loader::load(std::path::Path::new(&artifact)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("pvc-native: refusing artifact: {e}");
            std::process::exit(3);
        }
    };
    eprintln!("pvc-native: artifact verified, canonical sha256 {}", loaded.sha256);
    let rt = match Runtime::from_signed(&loaded.bytes, loader::verify, Options { locale: locale.clone(), width: 1366.0, height: 768.0, scale: 1.0 }) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("pvc-native: refusing artifact: runtime load: {e}");
            std::process::exit(3);
        }
    };
    let info = app::HostInfo {
        artifact_sha256: loaded.sha256,
        artifact,
        locale,
        renderer,
        profile: if cfg!(debug_assertions) { "dev(debug)" } else { "release" },
    };
    if let Err(e) = app::run(rt, info, t_start, rt_start_ns) {
        eprintln!("pvc-native: {e}");
        std::process::exit(1);
    }
}
