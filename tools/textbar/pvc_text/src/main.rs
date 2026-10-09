//! A8 tool: render corpus.json through pvc_runtime's text API (Parley 0.11.1 + vello_cpu 0.3.0, grayscale AA),
//! write one PNG per (script,size,scale) and the line breaks.
//! usage: pvc_text <corpus.json> <out_dir>
use pvc_runtime::geom::Rgba;
use pvc_runtime::render::{render_cpu_rgba, text_display_list};
use pvc_runtime::text::{line_height_for, TextStyle, TextSystem};
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let corpus: serde_json::Value = serde_json::from_str(&fs::read_to_string(&args[1]).unwrap()).unwrap();
    let out = &args[2];
    fs::create_dir_all(out).unwrap();
    let wrap_w = corpus["wrap_width_css_px"].as_f64().unwrap() as f32;
    let mut all = serde_json::Map::new();
    all.insert("backend".into(), "pvc_runtime::render::text_display_list + render_cpu_rgba (dev-render text API; Parley 0.11.1, vello_cpu 0.3.0)".into());
    let mut cells = serde_json::Map::new();

    for script in ["latin", "zh-CN"] {
        let locale = if script == "zh-CN" { "zh-CN" } else { "en" };
        let items: Vec<&serde_json::Value> = corpus["items"].as_array().unwrap().iter().filter(|i| i["script"] == script).collect();
        let mut ts = TextSystem::new(locale);
        for size in [11u32, 13, 16, 24] {
            let lh = line_height_for(size as f32);
            for scale in [1u32, 2] {
                let sc = scale as f32;
                let mut total_h = 0.0f32;
                let mut slots = vec![];
                for it in &items {
                    let slot = it["slot_lines"].as_f64().unwrap() as f32 * lh + 8.0;
                    slots.push((total_h, slot));
                    total_h += slot;
                }
                let wpx = (800.0 * sc) as usize;
                let hpx = (total_h * sc) as usize;
                let mut page = vec![255u8; wpx * hpx * 4];
                let mut breaks = serde_json::Map::new();
                for (it, (top, slot)) in items.iter().zip(slots.iter()) {
                    let text = it["text"].as_str().unwrap();
                    let weight = it["weight"].as_u64().unwrap() as u16;
                    let wrap = it["kind"] == "paragraph";
                    let maxw = if wrap { Some(wrap_w) } else { None };
                    // item canvas: 792 x (slot-4) logical px, text at pad 0; pasted at (8, top+4) logical px
                    let (cw, ch) = (792.0f32, slot - 4.0);
                    let dl = text_display_list(text, locale, size as f32, weight, maxw, sc, Rgba::rgb(0, 0, 0), Rgba::rgb(255, 255, 255), 0.0, (cw, ch));
                    let (iw, ih, rgba) = render_cpu_rgba(&dl);
                    let (ox, oy) = ((8.0 * sc) as usize, ((top + 4.0) * sc) as usize);
                    for y in 0..ih as usize {
                        for x in 0..iw as usize {
                            let (px, py) = (ox + x, oy + y);
                            if px < wpx && py < hpx {
                                let s = (y * iw as usize + x) * 4;
                                let d = (py * wpx + px) * 4;
                                page[d..d + 4].copy_from_slice(&rgba[s..s + 4]);
                            }
                        }
                    }
                    // line starts from the same layout the renderer used
                    let layout = ts.layout(text, &TextStyle { size: size as f32, weight, line_height: lh, align: 0 }, maxw);
                    let mut starts: Vec<usize> = vec![];
                    let mut line_tops: Vec<f32> = vec![];
                    for line in layout.lines() {
                        let r = line.text_range();
                        let seg = &text[r.start..r.end];
                        if seg.chars().all(|c| c.is_whitespace()) { continue; }
                        let mut off16 = text[..r.start].encode_utf16().count();
                        for ch in seg.chars() {
                            if ch.is_whitespace() { off16 += ch.len_utf16(); } else { break; }
                        }
                        starts.push(off16);
                        line_tops.push(line.metrics().baseline);
                    }
                    breaks.insert(it["id"].as_str().unwrap().to_string(), serde_json::json!({"starts": starts, "lineTops": line_tops}));
                }
                let name = format!("{script}_s{size}_x{scale}");
                let file = fs::File::create(format!("{out}/{name}.png")).unwrap();
                let mut enc = png::Encoder::new(std::io::BufWriter::new(file), wpx as u32, hpx as u32);
                enc.set_color(png::ColorType::Rgba);
                enc.set_depth(png::BitDepth::Eight);
                enc.write_header().unwrap().write_image_data(&page).unwrap();
                cells.insert(name.clone(), serde_json::json!({"png": format!("{name}.png"), "height": total_h, "lh": lh, "breaks": breaks}));
            }
        }
    }
    all.insert("cells".into(), cells.into());
    fs::write(format!("{out}/breaks_pvc.json"), serde_json::to_string_pretty(&all).unwrap()).unwrap();
    println!("ok");
}
