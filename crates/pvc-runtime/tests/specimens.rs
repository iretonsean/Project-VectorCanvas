//! Conformance tests against the real signed dev-ir specimens.
#![cfg(feature = "cpu")]
mod common;
use common::*;
use pvc_runtime::*;

#[test]
fn unsigned_or_tampered_is_refused() {
    let mut bytes = std::fs::read(format!("{SPECIMENS}/L01.pvca")).unwrap();
    let n = bytes.len();
    bytes[n - 20] ^= 1;
    let r = Runtime::from_signed(&bytes, |b| pvc_ir::load_signed(b, &pvc_ir::sign::testkey::verifying_key()), Options { locale: "en".into(), width: 360.0, height: 640.0, scale: 1.0 });
    assert!(r.is_err());
}

#[test]
fn l01_layout_and_idle() {
    let mut rt = load("L01", "en", 1366.0, 768.0);
    assert!(rt.needs_frame());
    assert!(rt.load_warnings.is_empty(), "manifest fonts must all be in the bundle: {:?}", rt.load_warnings);
    rt.frame();
    assert!(!rt.needs_frame(), "0 frames requested when idle");
    let card = rt.rect_of("surface0/card0").unwrap();
    let btn = rt.rect_of("connect_button").unwrap();
    // G2 L01 A-items: button spans card width (inside card padding); avatar is circular, left of name.
    assert_eq!(btn.w, card.w - 2.0 * 24.0);
    let av = rt.rect_of("surface0/card0/row0/avatar0").unwrap();
    let name = rt.rect_of("surface0/card0/row0/col0/text0").unwrap();
    assert_eq!(av.w, av.h);
    assert!(av.right() <= name.x);
    println!("{}", rt.geometry_dump());
    let dl = rt.display_list().clone();
    std::fs::create_dir_all("/tmp/pvc_render").unwrap();
    std::fs::write("/tmp/pvc_render/L01.png", render::render_cpu_png(&dl)).unwrap();
}

#[test]
fn f01_renders_all_states() {
    let mut rt = load("F01", "en", 1366.0, 768.0);
    rt.frame();
    std::fs::create_dir_all("/tmp/pvc_render").unwrap();
    std::fs::write("/tmp/pvc_render/F01_empty.png", render::render_cpu_png(rt.display_list())).unwrap();
    println!("{}", rt.geometry_dump());
    type_into(&mut rt, "email", "not-an-email");
    std::fs::write("/tmp/pvc_render/F01_error.png", render::render_cpu_png(rt.display_list())).unwrap();
    println!("{}", rt.debug_state());
}
