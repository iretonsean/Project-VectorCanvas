//! Font / RSS probe (G6 risk 6): VmRSS after each stage, native, release. Prints one JSON line.
use pvc_runtime::*;
use std::fmt::Write;

fn status(key: &str) -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap();
    for l in s.lines() {
        if let Some(r) = l.strip_prefix(key) {
            return r.trim().trim_end_matches(" kB").trim().parse().unwrap();
        }
    }
    0
}

fn main() {
    let mut out = String::new();
    let mut rec = |name: &str, out: &mut String| {
        let _ = write!(out, "\"{name}\":{{\"rss_kb\":{},\"hwm_kb\":{}}},", status("VmRSS:"), status("VmHWM:"));
    };
    rec("0_start", &mut out);
    let mut ts = text::TextSystem::new("zh-CN");
    rec("1_fontcontext_registered", &mut out);
    let st = text::TextStyle { size: 14.0, weight: 400, line_height: 19.0, align: 0 };
    let _l = ts.layout("Create account Hello", &st, Some(300.0));
    rec("2_first_latin_layout", &mut out);
    let _l = ts.layout("创建账户 你好，世界", &st, Some(300.0));
    rec("3_first_cjk_layout", &mut out);
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../specimens/F01.pvca")).unwrap();
    let mut rt = Runtime::from_signed(&bytes, |b| pvc_ir::load_signed(b, &pvc_ir::sign::testkey::verifying_key()), Options { locale: "zh-CN".into(), width: 1366.0, height: 768.0, scale: 1.0 }).unwrap();
    rt.frame();
    rec("4_f01_loaded_first_frame", &mut out);
    #[cfg(feature = "cpu")]
    {
        let _png = render::render_cpu_rgba(rt.display_list());
        rec("5_after_vello_cpu_render", &mut out);
    }
    let _ = write!(out, "\"bundled_font_bytes\":{},\"cjk_regular_bytes\":{}", fonts::bundled_bytes(), fonts::CJK_SC_REGULAR.len());
    println!("{{{out}}}");
}
