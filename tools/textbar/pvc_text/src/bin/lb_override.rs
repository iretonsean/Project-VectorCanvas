//! Diagnostic (informational, NOT part of the bar): does Parley's CHROMIUM_LINE_BREAK_OVERRIDE change PVC's line breaks on the corpus?
//! Uses pvc_runtime's font context and the same style as pvc_runtime::text::TextSystem::layout, plus the override.
//! usage: lb_override <corpus.json> <chromium_breaks.json(default)> <chromium_breaks.json(nohint)>
use parley::{Alignment, AlignmentOptions, FontFamily, FontWeight, Layout, LayoutContext, LineHeight, StyleProperty};
use std::fs;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let corpus: serde_json::Value = serde_json::from_str(&fs::read_to_string(&a[1]).unwrap()).unwrap();
    let ch_def: serde_json::Value = serde_json::from_str(&fs::read_to_string(&a[2]).unwrap()).unwrap();
    let ch_nh: serde_json::Value = serde_json::from_str(&fs::read_to_string(&a[3]).unwrap()).unwrap();
    let mut font_cx = pvc_runtime::fonts::new_font_context();
    let mut lcx: LayoutContext<()> = LayoutContext::new();
    let (mut n, mut mis_plain_def, mut mis_ovr_def, mut mis_plain_nh, mut mis_ovr_nh) = (0, 0, 0, 0, 0);
    for script in ["latin", "zh-CN"] {
        for size in [11u32, 13, 16, 24] {
            let cell = format!("{script}_s{size}_x1");
            for it in corpus["items"].as_array().unwrap().iter().filter(|i| i["script"] == script && i["kind"] == "paragraph") {
                let text = it["text"].as_str().unwrap();
                let id = it["id"].as_str().unwrap();
                let mut res = vec![];
                for ovr in [false, true] {
                    let mut b = lcx.ranged_builder(&mut font_cx, text, 1.0, true);
                    b.push_default(StyleProperty::FontFamily(FontFamily::Source(pvc_runtime::fonts::STACK.into())));
                    b.push_default(StyleProperty::FontSize(size as f32));
                    b.push_default(StyleProperty::FontWeight(FontWeight::new(it["weight"].as_f64().unwrap() as f32)));
                    b.push_default(StyleProperty::LineHeight(LineHeight::Absolute(pvc_runtime::text::line_height_for(size as f32))));
                    b.push_default(StyleProperty::Locale(parley::fontique::Language::parse(if script == "zh-CN" { "zh-CN" } else { "en" }).ok()));
                    if ovr { b.set_line_break_override(Some(parley::CHROMIUM_LINE_BREAK_OVERRIDE)); }
                    let mut l: Layout<()> = b.build(text);
                    l.break_all_lines(Some(corpus["wrap_width_css_px"].as_f64().unwrap() as f32));
                    l.align(Alignment::Start, AlignmentOptions::default());
                    let mut starts = vec![];
                    for line in l.lines() {
                        let r = line.text_range(); let seg = &text[r.start..r.end];
                        if seg.chars().all(|c| c.is_whitespace()) { continue; }
                        let mut off = text[..r.start].encode_utf16().count();
                        for ch in seg.chars() { if ch.is_whitespace() { off += ch.len_utf16(); } else { break; } }
                        starts.push(off);
                    }
                    res.push(starts);
                }
                let get = |j: &serde_json::Value| -> Vec<usize> { j["cells"][&cell]["breaks"][id]["starts"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect() };
                let (d, nh) = (get(&ch_def), get(&ch_nh));
                n += 1;
                if res[0] != d { mis_plain_def += 1; } if res[1] != d { mis_ovr_def += 1; }
                if res[0] != nh { mis_plain_nh += 1; } if res[1] != nh { mis_ovr_nh += 1; }
                if res[0] != res[1] || res[0] != nh { println!("{cell} {id}: plain {:?} override {:?} chromium-default {:?} chromium-nohint {:?}", res[0], res[1], d, nh); }
            }
        }
    }
    println!("wrapped items x sizes (scale 1) = {n}; mismatches vs Chromium-default: plain {mis_plain_def}, override {mis_ovr_def}; vs Chromium-nohint: plain {mis_plain_nh}, override {mis_ovr_nh}");
}
