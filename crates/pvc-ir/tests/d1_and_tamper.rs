//! D1 determinism, round-trip, signing, tamper refusal (G2 X24).

use ed25519_dalek::SigningKey;
use pvc_ir::canon::{module_to_val, parse_val, write_val, Val};
use pvc_ir::sign::{self, testkey, LoadError};
use pvc_ir::*;

fn specimen(name: &str) -> String {
    std::fs::read_to_string(format!("{}/../specimens/{}.pvc", env!("CARGO_MANIFEST_DIR"), name)).unwrap()
}
fn artifact(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/../specimens/{}.pvca", env!("CARGO_MANIFEST_DIR"), name)).unwrap()
}

#[test]
fn d1_same_input_same_bytes_across_runs() {
    for s in ["L01", "F01"] {
        let a = canonicalize(&compile_terse(&specimen(s)).unwrap().module);
        let b = canonicalize(&compile_terse(&specimen(s)).unwrap().module);
        assert_eq!(a, b, "{s}: two compiles differ");
        // signing is deterministic (Ed25519) too
        let m = compile_terse(&specimen(s)).unwrap().module;
        assert_eq!(sign::sign(&m, &testkey::signing_key()), sign::sign(&m, &testkey::signing_key()));
    }
}

#[test]
fn committed_artifacts_match_a_fresh_compile() {
    for s in ["L01", "F01"] {
        let m = compile_terse(&specimen(s)).unwrap().module;
        assert_eq!(artifact(s), sign::sign(&m, &testkey::signing_key()), "{s}.pvca is stale: re-run pvc-compile");
        let canon = std::fs::read(format!("{}/../specimens/{}.canonical.json", env!("CARGO_MANIFEST_DIR"), s)).unwrap();
        assert_eq!(canon, canonicalize(&m), "{s}.canonical.json is stale");
    }
}

#[test]
fn whitespace_and_layout_of_terse_input_does_not_change_canonical_bytes() {
    let a = compile_terse(&specimen("L01")).unwrap().module;
    // same program, re-spaced, comments added, `column` alias, props reordered
    let src = specimen("L01")
        .replace("card w=320 border=soft pad=lg gap=md {", "card   pad=lg gap=md \\\n   border=soft w=320 {   // c\n")
        .replace("col {", "column {");
    let b = compile_terse(&src).unwrap().module;
    assert_eq!(canonicalize(&a), canonicalize(&b));
}

#[test]
fn canonical_roundtrip_is_exact() {
    for s in ["L01", "F01"] {
        let m = compile_terse(&specimen(s)).unwrap().module;
        let bytes = canonicalize(&m);
        let back = from_canonical(&bytes).unwrap();
        assert_eq!(canonicalize(&back), bytes);
        let mut stripped = m.clone();
        stripped.provenance.clear();
        assert_eq!(back, stripped);
    }
}

#[test]
fn nfc_strings_in_canonical_form() {
    let src = "pvc 0.1 use std@0.1\ntext \"cafe\u{301}\"\n";
    let m = parse_terse(src).unwrap();
    let canon = String::from_utf8(canonicalize(&m)).unwrap();
    assert!(canon.contains("caf\u{e9}"), "{canon}");
    assert!(!canon.contains("e\u{301}"));
}

#[test]
fn load_signed_accepts_good_artifact() {
    for s in ["L01", "F01"] {
        let m = load_signed(&artifact(s), &testkey::verifying_key()).unwrap();
        assert_eq!(m.header.ir, (0, 1));
    }
}

fn split_body(art: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut nl = 0;
    let mut i = 0;
    while nl < 2 {
        if art[i] == b'\n' {
            nl += 1;
        }
        i += 1;
    }
    (art[..i].to_vec(), art[i..].to_vec())
}

fn with_mic(body: &[u8]) -> Vec<u8> {
    // G2 X24: add `mic` to the manifest after signing.
    let mut v = parse_val(body).unwrap();
    if let Val::Obj(top) = &mut v {
        let manifest = top.iter_mut().find(|(k, _)| k == "manifest").unwrap();
        if let Val::Obj(mf) = &mut manifest.1 {
            let caps = mf.iter_mut().find(|(k, _)| k == "caps").unwrap();
            if let Val::Arr(c) = &mut caps.1 {
                c.push(Val::Obj(vec![
                    ("id".into(), Val::Str("mic.main".into())),
                    ("kind".into(), Val::Str("mic".into())),
                    ("hosts".into(), Val::Arr(vec![])),
                    ("methods".into(), Val::Arr(vec![])),
                    ("max_request_bytes".into(), Val::Int(0)),
                    ("max_daily_bytes".into(), Val::Int(0)),
                ]));
            }
        }
    }
    let mut out = Vec::new();
    write_val(&v, &mut out);
    out
}

#[test]
fn x24_tampered_manifest_is_refused() {
    let art = artifact("L01");
    let (hdr, body) = split_body(&art);
    let tampered_body = with_mic(&body);
    assert_ne!(tampered_body, body);
    let mut tampered = hdr.clone();
    tampered.extend_from_slice(&tampered_body);
    assert_eq!(load_signed(&tampered, &testkey::verifying_key()), Err(LoadError::BadSignature));

    // Control: the *same edit*, legitimately re-signed, loads. So the refusal above is the
    // signature check, not a parse problem with the edit.
    let resigned = sign::sign_canonical(&tampered_body, &testkey::signing_key());
    let m = load_signed(&resigned, &testkey::verifying_key()).expect("re-signed edit should load");
    assert!(m.manifest.caps.iter().any(|c| c.kind == "mic"));
}

#[test]
fn single_flipped_byte_is_refused() {
    let art = artifact("F01");
    let (_, body) = split_body(&art);
    for pos in [0usize, body.len() / 2, body.len() - 1] {
        let mut t = art.clone();
        let off = art.len() - body.len() + pos;
        t[off] ^= 0x01;
        assert!(load_signed(&t, &testkey::verifying_key()).is_err(), "flip at {pos} accepted");
    }
}

#[test]
fn wrong_signer_is_refused() {
    let m = compile_terse(&specimen("L01")).unwrap().module;
    let other = SigningKey::from_bytes(&[7u8; 32]);
    let art = sign::sign(&m, &other);
    assert_eq!(load_signed(&art, &testkey::verifying_key()), Err(LoadError::KeyMismatch));
    // signature swapped in from a different key but header claims the trusted key
    let good = artifact("L01");
    let (hdr, body) = split_body(&good);
    let forged = sign::sign_canonical(&body, &other);
    let (_, _fhdr_body) = split_body(&forged);
    let hdr_s = String::from_utf8(hdr).unwrap();
    let mut lines = hdr_s.lines();
    let h1 = lines.next().unwrap().to_string();
    let forged_sig = String::from_utf8(forged.clone()).unwrap_or_default();
    let sig2 = forged_sig.lines().nth(1).unwrap_or("").to_string();
    let mut out = format!("{}\n{}\n", h1, sig2).into_bytes();
    out.extend_from_slice(&body);
    assert_eq!(load_signed(&out, &testkey::verifying_key()), Err(LoadError::BadSignature));
}

#[test]
fn non_canonical_but_signed_body_is_refused() {
    // Signed by the trusted key, but with a whitespace-padded body: not canonical.
    let m = compile_terse(&specimen("L01")).unwrap().module;
    let mut body = Vec::new();
    write_val(&module_to_val(&m), &mut body);
    body.push(b' ');
    let art = sign::sign_canonical(&body, &testkey::signing_key());
    assert!(matches!(load_signed(&art, &testkey::verifying_key()), Err(LoadError::Canonical(_))));
}

#[test]
fn garbage_frames_are_refused() {
    for bad in [&b""[..], b"hello", b"PVC-SIGNED 1 ed25519 00\nzz\n{}", b"PVC-SIGNED 2 ed25519 aa\nbb\n{}"] {
        assert!(load_signed(bad, &testkey::verifying_key()).is_err());
    }
}
