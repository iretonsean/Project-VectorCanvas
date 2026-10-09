//! Headless per-stage frame timings on Tier 0 (c) (vello_cpu, native release). NOT a host measurement:
//! no window, no present, no input event path. Layout excludes a general constraint solve (D-4).
//! usage: frame_bench <out_csv> ; prints a summary JSON line. Run under the exclusive CPU lock.
use pvc_runtime::*;
use std::io::Write;

fn pct(v: &mut Vec<f64>, p: f64) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[((v.len() as f64 - 1.0) * p).round() as usize]
}

fn main() {
    let out = std::env::args().nth(1).unwrap();
    let spec = concat!(env!("CARGO_MANIFEST_DIR"), "/../specimens");
    let mut csv = std::fs::File::create(&out).unwrap();
    writeln!(csv, "scenario,frame_idx,update_ms,layout_ms,scene_ms,a11y_ms,render_ms,total_ms").unwrap();
    let mut summary = String::new();
    for (scenario, file, locale) in [("F01_type_en", "F01", "en"), ("F01_type_zhCN_ime", "F01", "zh-CN")] {
        let bytes = std::fs::read(format!("{spec}/{file}.pvca")).unwrap();
        let mut rt = Runtime::from_signed(&bytes, |b| pvc_ir::load_signed(b, &pvc_ir::sign::testkey::verifying_key()), Options { locale: locale.into(), width: 1366.0, height: 768.0, scale: 1.0 }).unwrap();
        let t0 = std::time::Instant::now();
        rt.frame();
        let _ = render::render_cpu_rgba(rt.display_list());
        let first_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let r = rt.rect_of("email").unwrap();
        rt.pointer_down(r.x + 10.0, r.y + 10.0);
        rt.pointer_up(r.x + 10.0, r.y + 10.0);
        let mut cols: Vec<Vec<f64>> = vec![vec![]; 6];
        let words = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
        for i in 0..210 {
            if scenario.contains("ime") {
                rt.ime_preedit(["n", "ni", "ni h", "ni ha", "ni hao"][i % 5], Some((1, 1)));
                if i % 5 == 4 {
                    rt.ime_commit("你好");
                }
            } else {
                rt.text(words[i % 10]);
            }
            assert!(rt.needs_frame());
            rt.frame();
            let t1 = std::time::Instant::now();
            let _ = render::render_cpu_rgba(rt.display_list());
            rt.record_render_ms(t1.elapsed().as_secs_f64() * 1000.0);
            let t = rt.timings();
            if i >= 10 {
                let row = [t.update_ms, t.layout_ms, t.scene_ms, t.a11y_ms, t.render_ms, t.total_ms];
                writeln!(csv, "{scenario},{i},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4}", row[0], row[1], row[2], row[3], row[4], row[5]).unwrap();
                for (c, v) in cols.iter_mut().zip(row) {
                    c.push(v);
                }
            }
            let _ = rt.take_tree_update();
        }
        let names = ["update", "layout", "scene", "a11y", "render", "total"];
        summary.push_str(&format!("\"{scenario}\":{{\"first_frame_ms\":{first_ms:.2},\"n\":{}", cols[0].len()));
        for (n, c) in names.iter().zip(cols.iter_mut()) {
            summary.push_str(&format!(",\"{n}_p50\":{:.3},\"{n}_p95\":{:.3},\"{n}_max\":{:.3}", pct(c, 0.5), pct(c, 0.95), pct(c, 1.0)));
        }
        summary.push_str("},");
        // idle check
        rt.frame();
        let _ = rt.take_tree_update();
        assert!(!rt.needs_frame());
    }
    println!("{{{summary}\"idle_needs_frame\":false}}");
}
