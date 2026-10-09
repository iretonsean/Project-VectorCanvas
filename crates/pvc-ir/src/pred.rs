//! Pure predicates shared by the validator, the checker and the runtime (single implementation).

/// `is-empty(text)`: true iff the text has zero characters. (No trimming: a field containing only
/// spaces is not empty. Authors who want trimming say so with `length`.)
pub fn is_empty(s: &str) -> bool {
    s.is_empty()
}

/// Number of Unicode scalar values.
pub fn length(s: &str) -> i64 {
    s.chars().count() as i64
}

/// `is-email(text)` — the exact PVC-IR v0 email-shape rule (GRAMMAR.md §6). ASCII-only, no IDN,
/// not a full RFC 5322 parser:
/// * exactly one `@`;
/// * local part: 1..=64 chars from `A-Za-z0-9` and ``!#$%&'*+/=?^_`{|}~.-``; may not start or end
///   with `.`, may not contain `..`;
/// * domain: total length ≤ 253; at least two dot-separated labels; each label 1..=63 chars of
///   `A-Za-z0-9-`, not starting or ending with `-`;
/// * the last label (TLD) is at least 2 chars and all ASCII letters.
pub fn is_email(s: &str) -> bool {
    let mut parts = s.split('@');
    let (local, domain) = match (parts.next(), parts.next(), parts.next()) {
        (Some(l), Some(d), None) => (l, d),
        _ => return false,
    };
    if local.is_empty() || local.len() > 64 {
        return false;
    }
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }
    let local_ok = local
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+/=?^_`{|}~.-".contains(&b));
    if !local_ok {
        return false;
    }
    if domain.is_empty() || domain.len() > 253 {
        return false;
    }
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    for l in &labels {
        if l.is_empty() || l.len() > 63 || l.starts_with('-') || l.ends_with('-') {
            return false;
        }
        if !l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return false;
        }
    }
    let tld = labels[labels.len() - 1];
    tld.len() >= 2 && tld.bytes().all(|b| b.is_ascii_alphabetic())
}

/// NFC-normalise (G4 §3 canonical ordering: strings in Unicode NFC).
pub fn nfc(s: &str) -> String {
    use icu_normalizer::ComposingNormalizerBorrowed;
    ComposingNormalizerBorrowed::new_nfc().normalize(s).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn email_rule() {
        for ok in ["a@b.co", "jane.doe+x@example.com", "x_y@sub.example.org", "a@b-c.io"] {
            assert!(is_email(ok), "{ok}");
        }
        for bad in [
            "", "a", "a@", "@b.co", "a@b", "a@b.c", "a@@b.co", "a b@c.de", ".a@b.co", "a.@b.co",
            "a..b@c.de", "a@-b.co", "a@b-.co", "a@b.c0", "a@b..co", "a@b.co.", "名前@b.co",
        ] {
            assert!(!is_email(bad), "{bad}");
        }
    }
    #[test]
    fn nfc_composes() {
        assert_eq!(nfc("e\u{301}"), "\u{e9}");
        assert_eq!(nfc("\u{e9}"), "\u{e9}");
    }
}
