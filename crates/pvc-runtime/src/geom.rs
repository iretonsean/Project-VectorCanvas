//! Geometry helpers. Layout runs in logical px, snapped to 1/64 px (G4 §5.1, §14 D3).

/// Axis-aligned rectangle in logical px.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }
    pub fn union(&self, o: &Rect) -> Rect {
        let x0 = self.x.min(o.x);
        let y0 = self.y.min(o.y);
        let x1 = self.right().max(o.right());
        let y1 = self.bottom().max(o.bottom());
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }
}

/// Snap a logical-px value to the 1/64 grid. Values on the grid are exact in f32.
#[inline]
pub fn snap(v: f32) -> f32 {
    (v * 64.0).round() / 64.0
}

/// 8-bit sRGB colour, non-premultiplied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgba(pub [u8; 4]);

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Rgba([r, g, b, 255])
    }
    pub const TRANSPARENT: Rgba = Rgba([0, 0, 0, 0]);
    pub fn is_clear(&self) -> bool {
        self.0[3] == 0
    }
}
