//! Built-in `std@0.1` themes and scale tokens (G4 §3 Theme). Colour tokens stay symbolic in nodes
//! (so a theme swap is possible); spacing/radius/type tokens are resolved to explicit lengths at expansion.

use crate::ast::{Len, Theme};

fn hex(s: &str) -> [u8; 4] {
    let v = u32::from_str_radix(s, 16).unwrap();
    [(v >> 16) as u8, (v >> 8) as u8, v as u8, 255]
}

/// Colour token names every theme defines.
pub const COLOR_TOKENS: &[&str] = &[
    "accent", "bg", "blue", "body", "border", "border-soft", "border-strong", "error", "focus", "fg",
    "ink-dark", "ink-light", "input-bg", "muted", "surface", "teal",
];

pub fn builtin(name: &str) -> Option<Theme> {
    let rows: &[(&str, &str)] = match name {
        "dark" => &[
            ("bg", "0f172a"),
            ("surface", "1e293b"),
            ("input-bg", "0f172a"),
            ("fg", "f8fafc"),
            ("body", "cbd5e1"),
            ("muted", "94a3b8"),
            ("border", "64748b"),
            ("border-soft", "334155"),
            ("border-strong", "94a3b8"),
            ("accent", "14b8a6"),
            ("blue", "2563eb"),
            ("teal", "14b8a6"),
            ("error", "fca5a5"),
            ("focus", "38bdf8"),
            ("ink-light", "ffffff"),
            ("ink-dark", "0f172a"),
        ],
        "light" => &[
            ("bg", "ffffff"),
            ("surface", "f8fafc"),
            ("input-bg", "ffffff"),
            ("fg", "0f172a"),
            ("body", "334155"),
            ("muted", "475569"),
            ("border", "64748b"),
            ("border-soft", "cbd5e1"),
            ("border-strong", "334155"),
            ("accent", "0f766e"),
            ("blue", "2563eb"),
            ("teal", "0d9488"),
            ("error", "b91c1c"),
            ("focus", "1d4ed8"),
            ("ink-light", "ffffff"),
            ("ink-dark", "0f172a"),
        ],
        _ => return None,
    };
    let mut colors: Vec<(String, [u8; 4])> =
        rows.iter().map(|(n, h)| (n.to_string(), hex(h))).collect();
    colors.sort_by(|a, b| a.0.cmp(&b.0));
    Some(Theme { name: name.to_string(), colors })
}

pub fn theme_names() -> &'static [&'static str] {
    &["dark", "light"]
}

/// Spacing scale (logical px).
pub const SPACE: &[(&str, i32)] = &[("none", 0), ("xs", 4), ("sm", 8), ("md", 16), ("lg", 24), ("xl", 32)];
/// Radius scale (logical px).
pub const RADIUS: &[(&str, i32)] = &[("none", 0), ("sm", 4), ("md", 8), ("lg", 12)];

pub fn space(name: &str) -> Option<Len> {
    SPACE.iter().find(|(n, _)| *n == name).map(|(_, v)| Len::from_px(*v))
}
pub fn radius(name: &str) -> Option<Len> {
    RADIUS.iter().find(|(n, _)| *n == name).map(|(_, v)| Len::from_px(*v))
}

/// WCAG 2.x relative luminance of an sRGB colour.
pub fn luminance(c: [u8; 4]) -> f64 {
    fn ch(v: u8) -> f64 {
        let s = v as f64 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])
}

/// WCAG contrast ratio between two opaque colours.
pub fn contrast(a: [u8; 4], b: [u8; 4]) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// The text-on-fill rule used by `std`: whichever of `ink-light`/`ink-dark` has the higher
/// contrast against the fill (tie -> `ink-light`).
pub fn best_on_token(fill: [u8; 4], theme: &Theme) -> &'static str {
    let get = |n: &str| theme.colors.iter().find(|(k, _)| k == n).map(|(_, v)| *v).unwrap();
    let l = contrast(get("ink-light"), fill);
    let d = contrast(get("ink-dark"), fill);
    if d > l {
        "ink-dark"
    } else {
        "ink-light"
    }
}
