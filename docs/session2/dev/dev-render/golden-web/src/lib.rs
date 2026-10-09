use pvc_runtime::{Options, Runtime};
use vello_gpu::peniko::color::{AlphaColor, Srgb};
use vello_gpu::{RenderSettings, RenderSize, Scene, WebGlRenderer, WebGlTextureBindings};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

fn run_state(state: &str, l01: &[u8], f01: &[u8]) -> Result<Runtime, String> {
    let load = |bytes: &[u8], locale: &str| {
        Runtime::from_signed(
            bytes,
            |b| pvc_ir::load_signed(b, &pvc_ir::sign::testkey::verifying_key()),
            Options { locale: locale.into(), width: 1366.0, height: 768.0, scale: 1.0 },
        )
        .map_err(|e| format!("{e}"))
    };
    let click = |rt: &mut Runtime, id: &str| {
        rt.frame();
        let r = rt.rect_of(id).unwrap();
        let (x, y) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame();
    };
    let mut rt;
    match state {
        "L01_idle" => rt = load(l01, "en")?,
        "F01_empty" => rt = load(f01, "en")?,
        "F01_error" => {
            rt = load(f01, "en")?;
            click(&mut rt, "email");
            rt.text("not-an-email");
        }
        "F01_zh_preedit" => {
            rt = load(f01, "zh-CN")?;
            click(&mut rt, "name");
            rt.ime_commit("张伟");
            click(&mut rt, "email");
            rt.ime_preedit("ni hao", Some((6, 6)));
        }
        _ => return Err(format!("unknown state {state}")),
    }
    rt.frame();
    Ok(rt)
}

/// Render `state` on a WebGL2 canvas with vello_gpu (webgl feature). Returns JSON
/// {fingerprint, ops, width, height, renderer}. The JS side reads the canvas pixels.
#[wasm_bindgen]
pub fn golden(canvas_id: &str, state: &str, l01: &[u8], f01: &[u8]) -> Result<String, JsValue> {
    let rt = run_state(state, l01, f01).map_err(|e| JsValue::from_str(&e))?;
    let dl = rt.display_list();
    let (w, h) = dl.pixel_size();
    let doc = web_sys::window().unwrap().document().unwrap();
    let canvas: HtmlCanvasElement = doc.get_element_by_id(canvas_id).ok_or("no canvas")?.unchecked_into();
    canvas.set_width(w as u32);
    canvas.set_height(h as u32);
    let settings = RenderSettings::default();
    let (mut renderer, mut resources) = match WebGlRenderer::new_with(&canvas, settings, true) {
        Ok(v) => v,
        Err(first) => {
            // vello_gpu asks for failIfMajorPerformanceCaveat:true; SwiftShader may refuse, so create the
            // context ourselves with the caveat check off (same fallback as dev-web's host).
            let opts = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&opts, &"antialias".into(), &JsValue::FALSE);
            let _ = js_sys::Reflect::set(&opts, &"depth".into(), &JsValue::TRUE);
            let _ = js_sys::Reflect::set(&opts, &"failIfMajorPerformanceCaveat".into(), &JsValue::FALSE);
            let _ = js_sys::Reflect::set(&opts, &"preserveDrawingBuffer".into(), &JsValue::TRUE);
            canvas.get_context_with_context_options("webgl2", &opts).map_err(|e| format!("{first}; {e:?}"))?;
            WebGlRenderer::new_with(&canvas, settings, true).map_err(|e| format!("first: {first}; second: {e}"))?
        }
    };
    let mut scene = Scene::new_with(w, h, settings.level);
    pvc_runtime::render::replay_gpu(dl, &mut scene, &mut resources);
    let clear = AlphaColor::<Srgb>::new([1.0, 1.0, 1.0, 1.0]);
    renderer
        .render(&scene, &mut resources, &RenderSize { width: w, height: h }, &WebGlTextureBindings::new(), clear)
        .map_err(|e| JsValue::from_str(&format!("render: {e}")))?;
    Ok(format!("{{\"fingerprint\":\"{}\",\"ops\":{},\"width\":{},\"height\":{}}}", dl.fingerprint(), dl.ops.len(), w, h))
}
