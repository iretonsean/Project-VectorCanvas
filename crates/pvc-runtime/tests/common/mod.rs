#![allow(dead_code)]
use pvc_runtime::*;

pub const SPECIMENS: &str = "/workspace/dev_work/dev-ir/specimens";

pub fn load(name: &str, locale: &str, w: f32, h: f32) -> Runtime {
    let bytes = std::fs::read(format!("{SPECIMENS}/{name}.pvca")).expect("specimen");
    Runtime::from_signed(
        &bytes,
        |b| pvc_ir::load_signed(b, &pvc_ir::sign::testkey::verifying_key()),
        Options { locale: locale.into(), width: w, height: h, scale: 1.0 },
    )
    .expect("load")
}

pub fn center(rt: &Runtime, id: &str) -> (f32, f32) {
    let r = rt.rect_of(id).unwrap_or_else(|| panic!("no rect for {id}"));
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}

pub fn click(rt: &mut Runtime, id: &str) {
    rt.frame();
    let (x, y) = center(rt, id);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
}

pub fn type_into(rt: &mut Runtime, id: &str, s: &str) {
    click(rt, id);
    rt.text(s);
    rt.frame();
}
