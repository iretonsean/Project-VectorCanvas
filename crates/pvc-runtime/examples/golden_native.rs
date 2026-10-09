//! Renders the four A7 states (TOLERANCE.md §2) on Tier 0 (vello_cpu) and gl-llvmpipe, writes PNGs and
//! the display-list fingerprints. Usage: golden_native <out_dir>
use pvc_runtime::*;

const SPEC: &str = "/workspace/dev_work/dev-ir/specimens";

fn load(name: &str, locale: &str) -> Runtime {
    let bytes = std::fs::read(format!("{SPEC}/{name}.pvca")).unwrap();
    Runtime::from_signed(&bytes, |b| pvc_ir::load_signed(b, &pvc_ir::sign::testkey::verifying_key()), Options { locale: locale.into(), width: 1366.0, height: 768.0, scale: 1.0 }).unwrap()
}

fn click(rt: &mut Runtime, id: &str) {
    rt.frame();
    let r = rt.rect_of(id).unwrap();
    let (x, y) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame();
}

fn state(name: &str) -> DisplayListOwned {
    let mut rt;
    match name {
        "L01_idle" => {
            rt = load("L01", "en");
        }
        "F01_empty" => {
            rt = load("F01", "en");
        }
        "F01_error" => {
            rt = load("F01", "en");
            click(&mut rt, "email");
            rt.text("not-an-email");
        }
        "F01_zh_preedit" => {
            rt = load("F01", "zh-CN");
            click(&mut rt, "name");
            rt.ime_commit("张伟");
            click(&mut rt, "email");
            rt.ime_preedit("ni hao", Some((6, 6)));
        }
        _ => panic!("unknown state"),
    }
    rt.frame();
    DisplayListOwned(rt.display_list().clone())
}

struct DisplayListOwned(paint::DisplayList);

fn main() {
    let out = std::env::args().nth(1).expect("out dir");
    std::fs::create_dir_all(&out).unwrap();
    let h = render_gl::new_headless().expect("GL adapter");
    let mut fp = String::new();
    for s in ["L01_idle", "F01_empty", "F01_error", "F01_zh_preedit"] {
        let dl = state(s).0;
        std::fs::write(format!("{out}/{s}.tier0c-vello_cpu.png"), render::render_cpu_png(&dl)).unwrap();
        let (w, hh, rgba) = render_gl::render_gl_rgba(&h, &dl);
        std::fs::write(format!("{out}/{s}.gl-llvmpipe.png"), render::png_from_rgba(w, hh, rgba)).unwrap();
        fp.push_str(&format!("{s} {} ops={}\n", dl.fingerprint(), dl.ops.len()));
        std::fs::write(format!("{out}/{s}.displaylist.txt"), dl.dump()).unwrap();
    }
    fp.push_str(&format!("gl adapter: {:?}\n", h.adapter_info));
    std::fs::write(format!("{out}/fingerprints.native.txt"), fp).unwrap();
}
