//! Single-line text field on Parley's `PlainEditor` (G6 §6): caret, selection, word/line motion,
//! IME preedit (set / clear / commit), and password masking.
//!
//! Masking: `PlainEditor` has no mask mode, so the editor buffer holds the *display* string
//! (one U+2022 per character when masked) and `real` holds the true text. Every edit is a single
//! contiguous replacement, so the true text is updated from (caret, inserted length, length delta)
//! without diffing. IME preedit is not masked and not accepted while the field is masked (labelled
//! gap); committed text is masked like any other insert.

use crate::fonts;
use crate::geom::Rect;
use crate::text::{TextStyle, TextSystem};
use parley::editing::PlainEditor;
use parley::{FontFamily, FontWeight, LineHeight, StyleProperty};

const BULLET: char = '\u{2022}';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    WordLeft,
    WordRight,
    Home,
    End,
}

pub struct Field {
    pub ed: PlainEditor<()>,
    real: String,
    masked: bool,
    pub scroll_x: f32,
    size: f32,
    line_height: f32,
    locale: String,
}

fn bullets(n: usize) -> String {
    std::iter::repeat(BULLET).take(n).collect()
}

impl Field {
    pub fn new(size: f32, line_height: f32, locale: &str) -> Field {
        let mut f = Field { ed: PlainEditor::new(size), real: String::new(), masked: false, scroll_x: 0.0, size, line_height, locale: locale.into() };
        f.apply_styles();
        f.ed.set_width(None);
        f
    }

    fn apply_styles(&mut self) {
        let st = self.ed.edit_styles();
        st.insert(StyleProperty::FontFamily(FontFamily::Source(fonts::STACK.into())));
        st.insert(StyleProperty::FontSize(self.size));
        st.insert(StyleProperty::FontWeight(FontWeight::new(400.0)));
        st.insert(StyleProperty::LineHeight(LineHeight::Absolute(self.line_height)));
        if let Ok(l) = fontique::Language::parse(&self.locale) {
            st.insert(StyleProperty::Locale(Some(l)));
        }
    }

    pub fn set_locale(&mut self, locale: &str) {
        if self.locale != locale {
            self.locale = locale.into();
            self.apply_styles();
        }
    }

    pub fn style(&self) -> TextStyle {
        TextStyle { size: self.size, weight: 400, line_height: self.line_height, align: 0 }
    }

    pub fn value(&self) -> &str {
        &self.real
    }

    pub fn is_masked(&self) -> bool {
        self.masked
    }

    pub fn is_composing(&self) -> bool {
        self.ed.is_composing()
    }

    /// Text shown on screen (bullets when masked; includes any preedit).
    pub fn display_text(&self) -> &str {
        self.ed.raw_text()
    }

    fn display_for(&self, real: &str, masked: bool) -> String {
        if masked {
            bullets(real.chars().count())
        } else {
            real.to_string()
        }
    }

    /// Switch masking on/off (show/hide toggle). Keeps the caret at the same character index.
    pub fn set_masked(&mut self, ts: &mut TextSystem, masked: bool) {
        if masked == self.masked {
            return;
        }
        let caret = self.caret_char();
        if self.ed.is_composing() {
            let mut d = self.ed.driver(&mut ts.font_cx, &mut ts.layout_cx);
            d.clear_compose();
        }
        self.masked = masked;
        let disp = self.display_for(&self.real.clone(), masked);
        self.ed.set_text(&disp);
        self.move_to_char(ts, caret);
    }

    /// Replace the whole value from outside (state changed under us). Returns true if it differed.
    pub fn set_value(&mut self, ts: &mut TextSystem, v: &str) -> bool {
        if v == self.real {
            return false;
        }
        self.real = v.to_string();
        let disp = self.display_for(v, self.masked);
        self.ed.set_text(&disp);
        let n = self.real.chars().count();
        self.move_to_char(ts, n);
        true
    }

    fn char_of_byte(&self, byte: usize) -> usize {
        let t = self.ed.raw_text();
        t[..byte.min(t.len())].chars().count()
    }

    fn caret_char(&self) -> usize {
        self.char_of_byte(self.ed.raw_selection().focus().index())
    }

    fn move_to_char(&mut self, ts: &mut TextSystem, ch: usize) {
        let byte = self.ed.raw_text().char_indices().nth(ch).map(|(b, _)| b).unwrap_or(self.ed.raw_text().len());
        let mut d = self.ed.driver(&mut ts.font_cx, &mut ts.layout_cx);
        d.move_to_byte(byte);
    }

    /// Apply an edit closure; keep `real` in sync. `inserted` is the text the closure inserts.
    /// Returns true when the true value changed.
    fn edit<F: FnOnce(&mut parley::editing::PlainEditorDriver<'_, ()>)>(&mut self, ts: &mut TextSystem, inserted: &str, f: F) -> bool {
        if !self.masked {
            let before = self.real.clone();
            {
                let mut d = self.ed.driver(&mut ts.font_cx, &mut ts.layout_cx);
                f(&mut d);
            }
            self.real = self.ed.text().to_string();
            return self.real != before;
        }
        let n0 = self.ed.raw_text().chars().count();
        let k = inserted.chars().count();
        {
            let mut d = self.ed.driver(&mut ts.font_cx, &mut ts.layout_cx);
            f(&mut d);
        }
        let n1 = self.ed.raw_text().chars().count();
        let c1 = self.caret_char();
        let start = c1.saturating_sub(k);
        let removed = (n0 + k).saturating_sub(n1);
        let chars: Vec<char> = self.real.chars().collect();
        let mut out = String::new();
        out.extend(chars[..start.min(chars.len())].iter());
        out.push_str(inserted);
        out.extend(chars[(start + removed).min(chars.len())..].iter());
        let changed = out != self.real;
        self.real = out;
        changed
    }

    pub fn insert(&mut self, ts: &mut TextSystem, s: &str) -> bool {
        // Single-line: drop line breaks.
        let s: String = s.chars().filter(|c| *c != '\n' && *c != '\r').collect();
        if s.is_empty() {
            return false;
        }
        let shown = if self.masked { bullets(s.chars().count()) } else { s.clone() };
        // A commit replaces any preedit first.
        self.edit(ts, &s, |d| {
            if d.editor.is_composing() {
                d.clear_compose();
            }
            d.insert_or_replace_selection(&shown);
        })
    }

    pub fn backspace(&mut self, ts: &mut TextSystem, word: bool) -> bool {
        self.edit(ts, "", |d| if word { d.backdelete_word() } else { d.backdelete() })
    }

    pub fn delete(&mut self, ts: &mut TextSystem, word: bool) -> bool {
        self.edit(ts, "", |d| if word { d.delete_word() } else { d.delete() })
    }

    pub fn motion(&mut self, ts: &mut TextSystem, m: Motion, extend: bool) {
        let mut d = self.ed.driver(&mut ts.font_cx, &mut ts.layout_cx);
        match (m, extend) {
            (Motion::Left, false) => d.move_left(),
            (Motion::Left, true) => d.select_left(),
            (Motion::Right, false) => d.move_right(),
            (Motion::Right, true) => d.select_right(),
            (Motion::WordLeft, false) => d.move_word_left(),
            (Motion::WordLeft, true) => d.select_word_left(),
            (Motion::WordRight, false) => d.move_word_right(),
            (Motion::WordRight, true) => d.select_word_right(),
            (Motion::Home, false) => d.move_to_line_start(),
            (Motion::Home, true) => d.select_to_line_start(),
            (Motion::End, false) => d.move_to_line_end(),
            (Motion::End, true) => d.select_to_line_end(),
        }
    }

    pub fn select_all(&mut self, ts: &mut TextSystem) {
        self.ed.driver(&mut ts.font_cx, &mut ts.layout_cx).select_all();
    }

    pub fn selected_text(&self) -> Option<String> {
        if self.masked {
            return None;
        }
        self.ed.selected_text().map(|s| s.to_string())
    }

    pub fn click(&mut self, ts: &mut TextSystem, x: f32, y: f32, extend: bool) {
        let x = x + self.scroll_x;
        let mut d = self.ed.driver(&mut ts.font_cx, &mut ts.layout_cx);
        if extend {
            d.extend_selection_to_point(x, y);
        } else {
            d.move_to_point(x, y);
        }
    }

    pub fn double_click(&mut self, ts: &mut TextSystem, x: f32, y: f32) {
        let x = x + self.scroll_x;
        self.ed.driver(&mut ts.font_cx, &mut ts.layout_cx).select_word_at_point(x, y);
    }

    /// IME preedit (winit `Ime::Preedit`). Empty text clears the composition. `cursor` is a byte
    /// range inside `text`, or `None` to hide the caret. Ignored while masked.
    pub fn ime_preedit(&mut self, ts: &mut TextSystem, text: &str, cursor: Option<(usize, usize)>) {
        if self.masked {
            return;
        }
        let mut d = self.ed.driver(&mut ts.font_cx, &mut ts.layout_cx);
        if text.is_empty() {
            d.clear_compose();
        } else {
            d.set_compose(text, cursor);
        }
    }

    pub fn ime_clear(&mut self, ts: &mut TextSystem) {
        self.ed.driver(&mut ts.font_cx, &mut ts.layout_cx).clear_compose();
    }

    /// IME commit: replaces the preedit (if any) with `text`. Returns true if the value changed.
    pub fn ime_commit(&mut self, ts: &mut TextSystem, text: &str) -> bool {
        self.insert(ts, text)
    }

    /// Make sure layout is current and the caret is inside the visible width; returns scroll_x.
    pub fn layout_and_scroll(&mut self, ts: &mut TextSystem, visible_w: f32) {
        self.ed.refresh_layout(&mut ts.font_cx, &mut ts.layout_cx);
        let total = self.ed.try_layout().map(|l| l.full_width()).unwrap_or(0.0);
        if let Some(c) = self.ed.cursor_geometry(1.5) {
            let cx0 = c.x0 as f32;
            if cx0 - self.scroll_x > visible_w - 2.0 {
                self.scroll_x = cx0 - visible_w + 2.0;
            }
            if cx0 - self.scroll_x < 0.0 {
                self.scroll_x = cx0;
            }
        }
        if total <= visible_w {
            self.scroll_x = 0.0;
        }
        self.scroll_x = self.scroll_x.max(0.0);
    }

    /// Rectangle of the caret / preedit area in field-content coordinates (before scroll), for the
    /// IME candidate window and the web overlay.
    pub fn ime_area(&self) -> Rect {
        let b = self.ed.ime_cursor_area();
        Rect::new(b.x0 as f32 - self.scroll_x, b.y0 as f32, (b.x1 - b.x0) as f32, (b.y1 - b.y0) as f32)
    }

    /// Selection range in chars of the display string: (anchor, focus).
    pub fn selection_chars(&self) -> (usize, usize) {
        let s = self.ed.raw_selection();
        (self.char_of_byte(s.anchor().index()), self.char_of_byte(s.focus().index()))
    }
}

impl Field {
    /// Selection as UTF-16 code-unit offsets into the *real* value (matches DOM `selectionStart/End`).
    pub fn selection_utf16(&self) -> (usize, usize) {
        let (a, b) = self.selection_chars();
        let conv = |ci: usize| -> usize { self.real.chars().take(ci).map(|c| c.len_utf16()).sum() };
        (conv(a), conv(b))
    }

    /// Set the selection from UTF-16 offsets into the real value (anchor=start, focus=end).
    pub fn set_selection_utf16(&mut self, ts: &mut TextSystem, start: usize, end: usize) {
        let to_char = |u: usize| -> usize {
            let mut acc = 0;
            for (i, c) in self.real.chars().enumerate() {
                if acc >= u {
                    return i;
                }
                acc += c.len_utf16();
            }
            self.real.chars().count()
        };
        let (cs, ce) = (to_char(start), to_char(end));
        let byte = |ch: usize| self.ed.raw_text().char_indices().nth(ch).map(|(b, _)| b).unwrap_or(self.ed.raw_text().len());
        let (bs, be) = (byte(cs), byte(ce));
        self.ed.driver(&mut ts.font_cx, &mut ts.layout_cx).select_byte_range(bs, be);
    }

    /// Caret rectangle in field-content coordinates (scroll applied), if the caret is shown.
    pub fn caret_rect(&self) -> Option<Rect> {
        self.ed.cursor_geometry(1.5).map(|g| Rect::new(g.x0 as f32 - self.scroll_x, g.y0 as f32, (g.x1 - g.x0) as f32, (g.y1 - g.y0) as f32))
    }
}
