use pvc_runtime::geom::Rgba;
use pvc_runtime::render::*;
fn main() {
    let dl = text_display_list("Jane Doe Connect 你好", "en", 40.0, 700, Some(600.0), 1.0, Rgba::rgb(20, 20, 20), Rgba::rgb(255, 255, 255), 8.0, (620.0, 60.0));
    std::fs::write("/tmp/text_probe.png", render_cpu_png(&dl)).unwrap();
}
