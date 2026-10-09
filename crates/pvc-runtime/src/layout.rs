//! Layout: a direct solver for the sugar G4 §5.2 lists for L01/F01 — row/col, pad, gap, cross-axis
//! align, main-axis justify, fill/hug/fixed sizes — with all values snapped to 1/64 logical px
//! (G4 §5.1, D3). **No general (Cassowary-class) constraint solve is performed** (Supervisor
//! decision D-4): frame numbers measured with this module exclude a general solve.
//!
//! Layout is independent of rendering and of the host: it takes the flattened tree, a visibility
//! mask and the text content, and returns absolute rectangles in logical px.

use crate::geom::{snap, Rect};
use crate::model::Flat;
use crate::text::{TextStyle, TextSystem};
use pvc_ir::ast::{Align, Dir, Justify, NodeKind, Size};
use std::collections::HashMap;

pub struct Solved {
    pub rects: Vec<Rect>,
}

pub struct Ctx<'a> {
    pub flat: &'a Flat,
    pub vis: &'a [bool],
    /// Display text per node (Text nodes). `None` for others.
    pub texts: &'a [Option<String>],
    pub ts: &'a mut TextSystem,
    memo_h: HashMap<(usize, u32), f32>,
}

struct Child {
    idx: usize,
    rect: Rect,
}

fn text_style(n: &pvc_ir::ast::Node, align: u8) -> Option<TextStyle> {
    match &n.kind {
        NodeKind::Text(t) => Some(TextStyle { size: t.size.px(), weight: t.weight, line_height: t.line_height.px(), align }),
        _ => None,
    }
}

fn align_code(n: &pvc_ir::ast::Node) -> u8 {
    match &n.kind {
        NodeKind::Text(t) => match t.align {
            pvc_ir::ast::TextAlign::Start => 0,
            pvc_ir::ast::TextAlign::Center => 1,
            pvc_ir::ast::TextAlign::End => 2,
        },
        _ => 0,
    }
}

/// Solve the visible tree for a viewport. The root fills the viewport.
pub fn solve(flat: &Flat, vis: &[bool], texts: &[Option<String>], ts: &mut TextSystem, vw: f32, vh: f32) -> Solved {
    let mut cx = Ctx { flat, vis, texts, ts, memo_h: HashMap::new() };
    let mut rects = vec![Rect::default(); flat.len()];
    cx.place(0, 0.0, 0.0, snap(vw), snap(vh), &mut rects);
    Solved { rects }
}

impl<'a> Ctx<'a> {
    fn pad(&self, i: usize) -> (f32, f32, f32, f32) {
        let p = &self.flat.nodes[i].layout.pad;
        (p.top.px(), p.right.px(), p.bottom.px(), p.left.px())
    }

    fn vis_kids(&self, i: usize) -> Vec<usize> {
        self.flat.kids[i].iter().copied().filter(|&c| self.vis[c]).collect()
    }

    fn leaf_text(&mut self, i: usize, wrap: Option<f32>) -> Option<std::rc::Rc<parley::Layout<()>>> {
        let n = &self.flat.nodes[i];
        let st = text_style(n, align_code(n))?;
        let s = self.texts[i].clone().unwrap_or_default();
        Some(self.ts.layout(&s, &st, wrap))
    }

    /// Width the node wants when hugging, given the available width.
    fn intrinsic_w(&mut self, i: usize, avail: f32) -> f32 {
        let n = &self.flat.nodes[i];
        if let Size::Px(l) = n.layout.w {
            return l.px();
        }
        let (pt, pr, pb, pl) = self.pad(i);
        let _ = (pt, pb);
        match &n.kind {
            NodeKind::Text(_) => {
                // Hug text always measures with Start alignment so the width is the widest line.
                let st = TextStyle { align: 0, ..text_style(n, 0).unwrap() };
                let s = self.texts[i].clone().unwrap_or_default();
                let l = self.ts.layout(&s, &st, Some((avail - pl - pr).max(0.0)));
                snap(l.width().ceil() + pl + pr)
            }
            NodeKind::TextInput(_) => avail,
            NodeKind::Box | NodeKind::Surface | _ => {
                let kids = self.vis_kids(i);
                let dir = n.layout.dir;
                let gap = n.layout.gap.px();
                let inner = (avail - pl - pr).max(0.0);
                let mut acc = 0.0f32;
                for (k, &c) in kids.iter().enumerate() {
                    let w = self.child_hug_w(c, inner);
                    match dir {
                        Dir::Row => acc += w + if k > 0 { gap } else { 0.0 },
                        Dir::Col | Dir::Grid => acc = acc.max(w),
                    }
                }
                snap((acc + pl + pr).min(avail.max(0.0)))
            }
        }
    }

    /// A child's hug width regardless of its Fill setting (Fill children in a hugging container
    /// contribute their content width).
    fn child_hug_w(&mut self, c: usize, avail: f32) -> f32 {
        match self.flat.nodes[c].layout.w {
            Size::Px(l) => l.px().min(avail.max(l.px())),
            _ => self.intrinsic_w(c, avail),
        }
    }

    /// Height the node wants when laid out at width `w` (hugging vertically).
    fn height_for(&mut self, i: usize, w: f32) -> f32 {
        if let Size::Px(l) = self.flat.nodes[i].layout.h {
            return l.px();
        }
        let key = (i, w.to_bits());
        if let Some(&h) = self.memo_h.get(&key) {
            return h;
        }
        let (pt, pr, pb, pl) = self.pad(i);
        let n = &self.flat.nodes[i];
        let h = match &n.kind {
            NodeKind::Text(_) => {
                let l = self.leaf_text(i, Some((w - pl - pr).max(0.0))).unwrap();
                snap(l.height() + pt + pb)
            }
            NodeKind::TextInput(p) => snap(p.line_height.px() + pt + pb),
            NodeKind::Box | NodeKind::Surface | _ => {
                let (_, ch) = self.arrange(i, w, None);
                ch
            }
        };
        self.memo_h.insert(key, h);
        h
    }

    /// Arrange the visible children of container `i` at width `w` and optional height `h`.
    /// Returns child rects relative to the node origin and the node's (content-derived) height.
    fn arrange(&mut self, i: usize, w: f32, h: Option<f32>) -> (Vec<Child>, f32) {
        let (pt, pr, pb, pl) = self.pad(i);
        let lay = self.flat.nodes[i].layout.clone();
        let kids = self.vis_kids(i);
        let k = kids.len();
        let gap = lay.gap.px();
        let cw = (w - pl - pr).max(0.0);
        let gaps = if k > 1 { gap * (k as f32 - 1.0) } else { 0.0 };
        let mut out = Vec::with_capacity(k);
        match lay.dir {
            Dir::Col | Dir::Grid => {
                let ch_avail = h.map(|h| (h - pt - pb).max(0.0));
                let mut widths = vec![0.0f32; k];
                let mut heights: Vec<Option<f32>> = vec![None; k];
                for (j, &c) in kids.iter().enumerate() {
                    let cl = &self.flat.nodes[c].layout;
                    let (cwid_rule, chei_rule) = (cl.w, cl.h);
                    widths[j] = match cwid_rule {
                        Size::Px(p) => p.px().min(cw),
                        Size::Fill => cw,
                        Size::Hug => {
                            if lay.align == Align::Stretch {
                                cw
                            } else {
                                self.intrinsic_w(c, cw).min(cw)
                            }
                        }
                    };
                    widths[j] = snap(widths[j]);
                    heights[j] = match chei_rule {
                        Size::Px(p) => Some(p.px()),
                        Size::Hug => Some(self.height_for(c, widths[j])),
                        Size::Fill => {
                            if ch_avail.is_some() {
                                None
                            } else {
                                Some(self.height_for(c, widths[j]))
                            }
                        }
                    };
                }
                let fixed: f32 = heights.iter().flatten().sum();
                let nfill = heights.iter().filter(|x| x.is_none()).count();
                let mut lead = 0.0f32;
                let mut between = gap;
                if let Some(avail) = ch_avail {
                    let remaining = (avail - fixed - gaps).max(0.0);
                    if nfill > 0 {
                        let share = remaining / nfill as f32;
                        for hh in heights.iter_mut().filter(|x| x.is_none()) {
                            *hh = Some(share);
                        }
                    } else {
                        match lay.justify {
                            Justify::Start => {}
                            Justify::Center => lead = remaining / 2.0,
                            Justify::End => lead = remaining,
                            Justify::Between => {
                                if k > 1 {
                                    between = gap + remaining / (k as f32 - 1.0)
                                }
                            }
                        }
                    }
                }
                let mut y = pt + lead;
                let mut total = 0.0f32;
                for (j, &c) in kids.iter().enumerate() {
                    let hh = snap(heights[j].unwrap_or(0.0));
                    let x = match lay.align {
                        Align::Start | Align::Stretch => pl,
                        Align::Center => pl + (cw - widths[j]) / 2.0,
                        Align::End => pl + cw - widths[j],
                    };
                    out.push(Child { idx: c, rect: Rect::new(snap(x), snap(y), widths[j], hh) });
                    y += hh + between;
                    total += hh;
                }
                let content = total + gaps + pt + pb;
                (out, snap(h.unwrap_or(content)))
            }
            Dir::Row => {
                let mut widths = vec![0.0f32; k];
                let mut is_fill = vec![false; k];
                let mut used = 0.0f32;
                for (j, &c) in kids.iter().enumerate() {
                    match self.flat.nodes[c].layout.w {
                        Size::Px(p) => widths[j] = p.px(),
                        Size::Hug => widths[j] = self.intrinsic_w(c, cw).min(cw),
                        Size::Fill => is_fill[j] = true,
                    }
                    used += widths[j];
                }
                let nfill = is_fill.iter().filter(|x| **x).count();
                let remaining = (cw - used - gaps).max(0.0);
                let mut lead = 0.0f32;
                let mut between = gap;
                if nfill > 0 {
                    let share = remaining / nfill as f32;
                    for j in 0..k {
                        if is_fill[j] {
                            widths[j] = share;
                        }
                    }
                } else {
                    match lay.justify {
                        Justify::Start => {}
                        Justify::Center => lead = remaining / 2.0,
                        Justify::End => lead = remaining,
                        Justify::Between => {
                            if k > 1 {
                                between = gap + remaining / (k as f32 - 1.0)
                            }
                        }
                    }
                }
                for w_ in widths.iter_mut() {
                    *w_ = snap(*w_);
                }
                let mut hug_h = vec![0.0f32; k];
                for (j, &c) in kids.iter().enumerate() {
                    hug_h[j] = match self.flat.nodes[c].layout.h {
                        Size::Px(p) => p.px(),
                        _ => self.height_for(c, widths[j]),
                    };
                }
                let row_h = match h {
                    Some(hh) => (hh - pt - pb).max(0.0),
                    None => hug_h.iter().cloned().fold(0.0, f32::max),
                };
                let mut x = pl + lead;
                for (j, &c) in kids.iter().enumerate() {
                    let rule = self.flat.nodes[c].layout.h;
                    let stretch = matches!(rule, Size::Fill) || (rule == Size::Hug && lay.align == Align::Stretch);
                    let (hh, yoff) = if stretch {
                        (row_h, 0.0)
                    } else {
                        let hh = hug_h[j];
                        let off = match lay.align {
                            Align::Start | Align::Stretch => 0.0,
                            Align::Center => (row_h - hh) / 2.0,
                            Align::End => row_h - hh,
                        };
                        (hh, off)
                    };
                    out.push(Child { idx: c, rect: Rect::new(snap(x), snap(pt + yoff), widths[j], snap(hh)) });
                    x += widths[j] + between;
                }
                (out, snap(h.unwrap_or(row_h + pt + pb)))
            }
        }
    }

    fn place(&mut self, i: usize, x: f32, y: f32, w: f32, h: f32, rects: &mut [Rect]) {
        rects[i] = Rect::new(x, y, w, h);
        if !matches!(self.flat.nodes[i].kind, NodeKind::Text(_) | NodeKind::TextInput(_)) {
            let (kids, _) = self.arrange(i, w, Some(h));
            for c in kids {
                self.place(c.idx, snap(x + c.rect.x), snap(y + c.rect.y), c.rect.w, c.rect.h, rects);
            }
        }
    }
}
