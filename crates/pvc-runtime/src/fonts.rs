//! Bundled, content-hashed fonts (G6 §3). System fonts are never enumerated: the Fontique
//! collection is created with `system_fonts: false` and the `system` Cargo feature of
//! parley/fontique is off, so authored text cannot silently pick up a system font.

use fontique::{Blob, Collection, CollectionOptions, SourceCache};
use parley::FontContext;
use std::sync::Arc;

pub const LATIN_REGULAR: &[u8] = include_bytes!("../fonts/DejaVuSans.ttf");
pub const LATIN_BOLD: &[u8] = include_bytes!("../fonts/DejaVuSans-Bold.ttf");
pub const CJK_SC_REGULAR: &[u8] = include_bytes!("../fonts/NotoSansCJKsc-Regular.otf");
/// Authored-only bold face, subset to the glyphs of the specimens' strings tables (G6 §3).
pub const CJK_SC_BOLD_SUBSET: &[u8] = include_bytes!("../fonts/NotoSansCJKsc-Bold.subset.otf");

/// Family names used in font stacks. Authored text uses `STACK`.
pub const LATIN_FAMILY: &str = "DejaVu Sans";
pub const CJK_FAMILY: &str = "Noto Sans CJK SC";
pub const STACK: &str = "DejaVu Sans, Noto Sans CJK SC";

pub struct BundledFont {
    pub file: &'static str,
    pub family: &'static str,
    pub bytes: &'static [u8],
    pub sha256: &'static str,
    pub license: &'static str,
}

/// The bundle. `sha256` values are checked against the embedded bytes by a unit test.
pub const BUNDLE: [BundledFont; 4] = [
    BundledFont {
        file: "DejaVuSans.ttf",
        family: LATIN_FAMILY,
        bytes: LATIN_REGULAR,
        sha256: "ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280",
        license: "Bitstream Vera / DejaVu free license (DejaVu fonts 2.37)",
    },
    BundledFont {
        file: "DejaVuSans-Bold.ttf",
        family: LATIN_FAMILY,
        bytes: LATIN_BOLD,
        sha256: "5c1247acef7f2b8522a31742c76d6adcb5569bacc0be7ceaa4dc39dd252ce895",
        license: "Bitstream Vera / DejaVu free license (DejaVu fonts 2.37)",
    },
    BundledFont {
        file: "NotoSansCJKsc-Regular.otf",
        family: CJK_FAMILY,
        bytes: CJK_SC_REGULAR,
        sha256: "6ead94f36c0f9423a88356f5a58b2b1b8180a3ff553ecab2f7a816990b306c60",
        license: "SIL OFL 1.1 (Noto Sans CJK; SC face extracted from NotoSansCJK-Regular.ttc index 2, glyphs unchanged, NOT subset: user-input face)",
    },
    BundledFont {
        file: "NotoSansCJKsc-Bold.subset.otf",
        family: CJK_FAMILY,
        bytes: CJK_SC_BOLD_SUBSET,
        sha256: "a44805f531bccdd9eb28df4a5446132d88cf93326ed96b38366c45c2e4e3c5f9",
        license: "SIL OFL 1.1 (Noto Sans CJK Bold SC, fontTools subset to the 143 characters in fonts/NotoSansCJKsc-Bold.subset.chars.txt: authored-only face)",
    },
];

/// Total bytes of font data embedded (and registered with Fontique).
pub fn bundled_bytes() -> usize {
    BUNDLE.iter().map(|f| f.bytes.len()).sum()
}

/// A Parley `FontContext` containing exactly the bundled fonts and no system fonts.
pub fn new_font_context() -> FontContext {
    let mut collection = Collection::new(CollectionOptions { shared: false, system_fonts: false });
    for f in BUNDLE.iter() {
        let blob = Blob::new(Arc::new(f.bytes));
        let reg = collection.register_fonts(blob, None);
        assert!(!reg.is_empty(), "bundled font {} did not register", f.file);
    }
    FontContext { collection, source_cache: SourceCache::default() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn bundle_hashes_match() {
        for f in BUNDLE.iter() {
            let h = Sha256::digest(f.bytes);
            let hex: String = h.iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(hex, f.sha256, "{}", f.file);
        }
    }

    #[test]
    fn no_system_fonts() {
        let mut cx = new_font_context();
        let names: Vec<String> = cx.collection.family_names().map(|s| s.to_string()).collect();
        assert_eq!(names.len(), 2, "families: {names:?}");
        assert!(names.contains(&LATIN_FAMILY.to_string()) && names.contains(&CJK_FAMILY.to_string()));
    }
}
