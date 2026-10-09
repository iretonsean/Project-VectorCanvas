//! The display list: the one scene description all three renderer paths replay (Tier 0 vello_cpu,
//! vello_gpu on wgpu-GL, vello_gpu WebGL2). Coordinates are logical px; the renderer applies the
//! device scale as a transform. No author shaders: paint is rounded-rect, circle, solid colour,
//! 1-px border, clips and glyph runs (G4 §6, restricted to what L01/F01 need).

use crate::geom::{Rect, Rgba};
use parley::{FontData, Layout, PositionedLayoutItem};

#[derive(Clone, Debug)]
pub struct GlyphRunOp {
    pub font: FontData,
    pub size: f32,
    pub coords: Vec<i16>,
    /// Glyph positions in logical px, already offset by the text origin.
    pub glyphs: Vec<(u32, f32, f32)>,
    pub color: Rgba,
    pub hint: bool,
}

#[derive(Clone, Debug)]
pub enum DrawOp {
    FillRRect { rect: Rect, radius: f32, color: Rgba },
    /// Stroke centred on the rect edge inset by width/2 (so the border stays inside `rect`).
    StrokeRRect { rect: Rect, radius: f32, width: f32, color: Rgba },
    FillCircle { cx: f32, cy: f32, r: f32, color: Rgba },
    PushClip { rect: Rect, radius: f32 },
    PopClip,
    /// Begin an isolated group composited at this opacity (disabled nodes: 0.4, G4 runtime rule from dev-ir).
    PushOpacity(f32),
    PopLayer,
    Glyphs(GlyphRunOp),
}

#[derive(Clone, Debug, Default)]
pub struct DisplayList {
    pub width: f32,
    pub height: f32,
    /// Device pixel ratio; renderer target size is `ceil(width*scale) x ceil(height*scale)`.
    pub scale: f32,
    pub background: Rgba,
    pub ops: Vec<DrawOp>,
}

impl DisplayList {
    pub fn pixel_size(&self) -> (u16, u16) {
        ((self.width * self.scale).ceil() as u16, (self.height * self.scale).ceil() as u16)
    }

    pub fn push_layout(&mut self, layout: &Layout<()>, ox: f32, oy: f32, color: Rgba) {
        for line in layout.lines() {
            for item in line.items() {
                if let PositionedLayoutItem::GlyphRun(gr) = item {
                    let run = gr.run();
                    let size = run.font_size();
                    let glyphs: Vec<(u32, f32, f32)> =
                        gr.positioned_glyphs().map(|g| (g.id, ox + g.x, oy + g.y)).collect();
                    if glyphs.is_empty() {
                        continue;
                    }
                    self.ops.push(DrawOp::Glyphs(GlyphRunOp {
                        font: run.font().clone(),
                        size,
                        coords: run.normalized_coords().to_vec(),
                        glyphs,
                        color,
                        // G6 §4: hinting at <= 16 px when device scale < 1.5 (decided per renderer
                        // call from `scale`; the flag here says "size is eligible").
                        hint: size <= 16.0,
                    }));
                }
            }
        }
    }
}

impl DisplayList {
    /// Stable textual dump of the whole list (font data is represented by its length and index).
    /// Used to prove that every renderer path replays the same scene.
    pub fn dump(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        let _ = writeln!(s, "dl {}x{}@{} bg={:?}", self.width, self.height, self.scale, self.background.0);
        for op in &self.ops {
            match op {
                DrawOp::FillRRect { rect, radius, color } => {
                    let _ = writeln!(s, "fill {:?} r={} c={:?}", rect, radius, color.0);
                }
                DrawOp::StrokeRRect { rect, radius, width, color } => {
                    let _ = writeln!(s, "stroke {:?} r={} w={} c={:?}", rect, radius, width, color.0);
                }
                DrawOp::FillCircle { cx, cy, r, color } => {
                    let _ = writeln!(s, "circle {cx} {cy} {r} c={:?}", color.0);
                }
                DrawOp::PushClip { rect, radius } => {
                    let _ = writeln!(s, "clip {:?} r={}", rect, radius);
                }
                DrawOp::PopClip => s.push_str("popclip\n"),
                DrawOp::PushOpacity(o) => {
                    let _ = writeln!(s, "opacity {o}");
                }
                DrawOp::PopLayer => s.push_str("poplayer\n"),
                DrawOp::Glyphs(g) => {
                    let _ = writeln!(
                        s,
                        "glyphs font={}:{} size={} coords={:?} c={:?} hint={} n={} {:?}",
                        g.font.data.len(),
                        g.font.index,
                        g.size,
                        g.coords,
                        g.color.0,
                        g.hint,
                        g.glyphs.len(),
                        g.glyphs
                    );
                }
            }
        }
        s
    }

    /// FNV-1a 64 of [`DisplayList::dump`], lowercase hex.
    pub fn fingerprint(&self) -> String {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in self.dump().as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        format!("{h:016x}")
    }
}
