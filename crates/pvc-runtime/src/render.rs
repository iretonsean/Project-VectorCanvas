//! Replaying a [`DisplayList`] on the vello renderers. One macro body so the CPU and GPU scene
//! types are driven by literally the same code (same scene from the same layout).

use crate::geom::Rgba;
use crate::paint::{DisplayList, DrawOp};

macro_rules! replay_impl {
    ($name:ident, $ctx:ty, $res:ty, $kurbo:path, $peniko:path) => {
        pub fn $name(dl: &DisplayList, ctx: &mut $ctx, res: &mut $res) {
            use $kurbo as kurbo;
            use $peniko as peniko;
            use kurbo::{Affine, Circle, RoundedRect, Shape, Stroke};
            let color = |c: Rgba| {
                peniko::Color::from_rgba8(c.0[0], c.0[1], c.0[2], c.0[3])
            };
            let s = dl.scale as f64;
            ctx.reset_transform();
            ctx.set_transform(Affine::scale(s));
            // Background.
            if !dl.background.is_clear() {
                ctx.set_paint(color(dl.background));
                ctx.fill_rect(&kurbo::Rect::new(0.0, 0.0, dl.width as f64, dl.height as f64));
            }
            for op in &dl.ops {
                match op {
                    DrawOp::FillRRect { rect, radius, color: c } => {
                        ctx.set_paint(color(*c));
                        let r = kurbo::Rect::new(
                            rect.x as f64,
                            rect.y as f64,
                            rect.right() as f64,
                            rect.bottom() as f64,
                        );
                        if *radius > 0.0 {
                            ctx.fill_path(&RoundedRect::from_rect(r, *radius as f64).to_path(0.1));
                        } else {
                            ctx.fill_rect(&r);
                        }
                    }
                    DrawOp::StrokeRRect { rect, radius, width, color: c } => {
                        ctx.set_paint(color(*c));
                        ctx.set_stroke(Stroke::new(*width as f64));
                        let h = *width as f64 / 2.0;
                        let r = kurbo::Rect::new(
                            rect.x as f64 + h,
                            rect.y as f64 + h,
                            rect.right() as f64 - h,
                            rect.bottom() as f64 - h,
                        );
                        let rr = RoundedRect::from_rect(r, (*radius as f64 - h).max(0.0));
                        ctx.stroke_path(&rr.to_path(0.1));
                    }
                    DrawOp::FillCircle { cx, cy, r, color: c } => {
                        ctx.set_paint(color(*c));
                        ctx.fill_path(&Circle::new((*cx as f64, *cy as f64), *r as f64).to_path(0.1));
                    }
                    DrawOp::PushClip { rect, radius } => {
                        let r = kurbo::Rect::new(
                            rect.x as f64,
                            rect.y as f64,
                            rect.right() as f64,
                            rect.bottom() as f64,
                        );
                        ctx.push_clip_path(&RoundedRect::from_rect(r, *radius as f64).to_path(0.1));
                    }
                    DrawOp::PopClip => ctx.pop_clip(),
                    DrawOp::PushOpacity(o) => ctx.push_layer(None, None, Some(*o), None, None),
                    DrawOp::PopLayer => ctx.pop_layer(),
                    DrawOp::Glyphs(g) => {
                        ctx.set_paint(color(g.color));
                        let glyphs: Vec<glifo::Glyph> = g
                            .glyphs
                            .iter()
                            .map(|&(id, x, y)| glifo::Glyph { id, x, y })
                            .collect();
                        let _ = ctx
                            .glyph_run(res, &g.font)
                            .font_size(g.size)
                            .hint(g.hint && dl.scale < 1.5)
                            .normalized_coords(&g.coords)
                            .fill_glyphs(glyphs.into_iter());
                    }
                }
            }
        }
    };
}

#[cfg(feature = "cpu")]
replay_impl!(replay_cpu, vello_cpu::RenderContext, vello_cpu::Resources, vello_cpu::kurbo, vello_cpu::peniko);

#[cfg(any(feature = "gl", feature = "webgl"))]
replay_impl!(replay_gpu, vello_gpu::Scene, vello_gpu::Resources, vello_gpu::kurbo, vello_gpu::peniko);

/// Tier 0 (c): render the display list with `vello_cpu` into an RGBA8 buffer (premultiplied →
/// un-premultiplied on the way out). Returns (width, height, rgba bytes).
#[cfg(feature = "cpu")]
pub fn render_cpu_rgba(dl: &DisplayList) -> (u16, u16, Vec<u8>) {
    let (w, h) = dl.pixel_size();
    let mut ctx = vello_cpu::RenderContext::new(w, h);
    let mut res = vello_cpu::Resources::new();
    replay_cpu(dl, &mut ctx, &mut res);
    ctx.flush();
    let mut pm = vello_cpu::Pixmap::new(w, h);
    ctx.render(&mut pm, &mut res);
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    for p in pm.data() {
        let (r, g, b, a) = (p.r as u32, p.g as u32, p.b as u32, p.a as u32);
        if a == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            out.extend_from_slice(&[
                ((r * 255 + a / 2) / a).min(255) as u8,
                ((g * 255 + a / 2) / a).min(255) as u8,
                ((b * 255 + a / 2) / a).min(255) as u8,
                a as u8,
            ]);
        }
    }
    (w, h, out)
}

/// Tier 0 (c) → PNG bytes (the golden-reference encoder path).
#[cfg(feature = "cpu")]
pub fn render_cpu_png(dl: &DisplayList) -> Vec<u8> {
    let (w, h) = dl.pixel_size();
    let mut ctx = vello_cpu::RenderContext::new(w, h);
    let mut res = vello_cpu::Resources::new();
    replay_cpu(dl, &mut ctx, &mut res);
    ctx.flush();
    let mut pm = vello_cpu::Pixmap::new(w, h);
    ctx.render(&mut pm, &mut res);
    pm.into_png().expect("png encode")
}

/// Text-render entry point for the G6 text bar (dev-measure A8): one string, one style, drawn
/// with the bundled fonts and Parley/glifo/vello_cpu at `scale`, on an opaque background.
/// Returns the display list; feed it to [`render_cpu_rgba`]/[`render_cpu_png`].
pub fn text_display_list(
    text: &str,
    locale: &str,
    size_px: f32,
    weight: u16,
    max_width: Option<f32>,
    scale: f32,
    fg: Rgba,
    bg: Rgba,
    pad: f32,
    canvas: (f32, f32),
) -> DisplayList {
    let mut ts = crate::text::TextSystem::new(locale);
    let layout = ts.layout(text, &crate::text::TextStyle { size: size_px, weight, line_height: crate::text::line_height_for(size_px), align: 0 }, max_width);
    let mut dl = DisplayList { width: canvas.0, height: canvas.1, scale, background: bg, ops: vec![] };
    dl.push_layout(&layout, pad, pad, fg);
    dl
}

/// Encode an opaque RGBA8 buffer (e.g. from the GL readback or the browser) as PNG with the same
/// encoder as the Tier 0 path.
#[cfg(feature = "cpu")]
pub fn png_from_rgba(w: u16, h: u16, rgba: Vec<u8>) -> Vec<u8> {
    use vello_cpu::peniko::ImageAlphaType;
    let _ = ImageAlphaType::Alpha;
    let pm = vello_cpu::Pixmap::from_parts(rgba, w, h, Default::default());
    pm.into_png().expect("png")
}
