//! Shared by the native binary and the wasm32 export: runs the same scenarios and returns one text.
use pvc_runtime::{Key, Mods, Options, Runtime};

const L01: &[u8] = include_bytes!("../L01.pvca");
const F01: &[u8] = include_bytes!("../F01.pvca");

fn load(bytes: &[u8], locale: &str, w: f32, h: f32) -> Runtime {
    Runtime::from_signed(
        bytes,
        |b| pvc_ir::load_signed(b, &pvc_ir::sign::testkey::verifying_key()),
        Options { locale: locale.into(), width: w, height: h, scale: 1.0 },
    )
    .expect("load")
}

fn click(rt: &mut Runtime, id: &str) {
    let r = rt.rect_of(id).expect("rect");
    let (x, y) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
}

pub fn dump_all() -> String {
    let mut out = String::new();
    for (w, h) in [(1366.0f32, 768.0f32), (360.0, 640.0)] {
        for (name, bytes, locale) in [("L01", L01, "en"), ("F01", F01, "en"), ("F01", F01, "zh-CN")] {
            let mut rt = load(bytes, locale, w, h);
            out.push_str(&format!("## {name} {locale} {w}x{h} initial\n{}", rt.geometry_dump()));
            if name == "F01" {
                rt.frame();
                click(&mut rt, "email");
                rt.text("not-an-email");
                rt.frame();
                click(&mut rt, "password");
                rt.text("pw");
                rt.frame();
                out.push_str(&format!("## {name} {locale} {w}x{h} email+password errors shown\n{}", rt.geometry_dump()));
                click(&mut rt, "name");
                rt.ime_preedit("nihao", Some((5, 5)));
                rt.ime_commit("你好，世界");
                rt.key(Key::Left, Mods::default());
                rt.frame();
                out.push_str(&format!("## {name} {locale} {w}x{h} after commit\n{}", rt.geometry_dump()));
                let (a, b) = (rt.text_input_info("name").unwrap(), rt.caret_rect("name").unwrap());
                out.push_str(&format!("caret {:?} {:?} sel {:?}\n", (b.x * 64.0).round(), (b.h * 64.0).round(), a.selection_utf16));
            }
        }
    }
    out
}

thread_local! { static BUF: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) }; }

#[no_mangle]
pub extern "C" fn d3_run() -> u32 {
    let s = dump_all();
    BUF.with(|b| *b.borrow_mut() = s.into_bytes());
    BUF.with(|b| b.borrow().len() as u32)
}

#[no_mangle]
pub extern "C" fn d3_byte(i: u32) -> u32 {
    BUF.with(|b| b.borrow()[i as usize] as u32)
}
