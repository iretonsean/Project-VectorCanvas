//! Text shaping/layout through Parley 0.11.1 (HarfRust shaping, Fontique fonts, ICU4X
//! segmentation). PVC writes no shaper, bidi, line breaker or font parser (G6 §1).
//!
//! All layouts are built at scale 1.0 in logical px with `quantize = true`, so line metrics are
//! whole logical px and the result does not depend on the device scale (D3). The device scale is
//! applied by the renderer as a transform.

use crate::fonts;
use parley::{
    Alignment, AlignmentOptions, FontContext, FontFamily, FontWeight, Layout, LayoutContext,
    LineHeight, StyleProperty,
};
use std::collections::HashMap;
use std::rc::Rc;

/// Text style as the runtime uses it.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub size: f32,
    pub weight: u16,
    /// Absolute line height in logical px (from the IR `line_height`).
    pub line_height: f32,
    /// 0 = start, 1 = center, 2 = end (relative to the wrap width).
    pub align: u8,
}

/// Default line height for callers that do not have one (text-bar entry point): integer px.
pub fn line_height_for(size: f32) -> f32 {
    (size * 1.35).round()
}

pub struct TextSystem {
    pub font_cx: FontContext,
    pub layout_cx: LayoutContext<()>,
    pub locale: String,
    cache: HashMap<(String, u32, u16, u32, u8, u32, String), Rc<Layout<()>>>,
}

impl TextSystem {
    pub fn new(locale: &str) -> Self {
        TextSystem {
            font_cx: fonts::new_font_context(),
            layout_cx: LayoutContext::new(),
            locale: locale.to_string(),
            cache: HashMap::new(),
        }
    }

    pub fn set_locale(&mut self, locale: &str) {
        if self.locale != locale {
            self.locale = locale.to_string();
            self.cache.clear();
        }
    }

    fn language(&self) -> Option<fontique::Language> {
        fontique::Language::parse(&self.locale).ok()
    }

    /// Lay out `text` wrapped to `max_width` (logical px) or unwrapped when `None`.
    pub fn layout(&mut self, text: &str, style: &TextStyle, max_width: Option<f32>) -> Rc<Layout<()>> {
        let key = (
            text.to_string(),
            style.size.to_bits(),
            style.weight,
            style.line_height.to_bits(),
            style.align,
            max_width.map(|w| w.to_bits()).unwrap_or(u32::MAX),
            self.locale.clone(),
        );
        if let Some(l) = self.cache.get(&key) {
            return l.clone();
        }
        if self.cache.len() > 512 {
            self.cache.clear();
        }
        let lang = self.language();
        let mut b = self.layout_cx.ranged_builder(&mut self.font_cx, text, 1.0, true);
        b.push_default(StyleProperty::FontFamily(FontFamily::Source(fonts::STACK.into())));
        b.push_default(StyleProperty::FontSize(style.size));
        b.push_default(StyleProperty::FontWeight(FontWeight::new(style.weight as f32)));
        b.push_default(StyleProperty::LineHeight(LineHeight::Absolute(style.line_height)));
        if let Some(l) = lang {
            b.push_default(StyleProperty::Locale(Some(l)));
        }
        let mut layout: Layout<()> = b.build(text);
        layout.break_all_lines(max_width);
        let al = match style.align {
            1 => Alignment::Center,
            2 => Alignment::End,
            _ => Alignment::Start,
        };
        layout.align(al, AlignmentOptions::default());
        let rc = Rc::new(layout);
        self.cache.insert(key, rc.clone());
        rc
    }

    /// (min-content, max-content) widths in logical px.
    pub fn content_widths(&mut self, text: &str, style: &TextStyle) -> (f32, f32) {
        let l = self.layout(text, style, None);
        let w = l.calculate_content_widths();
        (w.min, w.max)
    }
}
