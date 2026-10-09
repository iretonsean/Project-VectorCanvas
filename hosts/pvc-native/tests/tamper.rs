//! A tampered artifact must be refused by the native host *before* any window/GPU is touched.
//! Uses dev-ir's signed specimens (read-only path) and edits copies: (a) one byte of the manifest text,
//! (b) a single flipped byte. Control: the untouched artifact loads.
use std::path::{Path, PathBuf};
use std::process::Command;

const SPEC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../crates/specimens");

fn tmp(name: &str, bytes: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join("pvc-native-tamper-test");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

fn run_host(artifact: &Path) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_pvc-native"))
        .arg(artifact)
        .env_remove("DISPLAY") // proves the refusal precedes window creation: no X server is reachable
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .unwrap();
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn control_untampered_loads() {
    for s in ["F01", "L01"] {
        let l = pvc_native::loader::load(Path::new(&format!("{SPEC}/{s}.pvca"))).expect("untampered artifact must load");
        assert_eq!(l.sha256.len(), 64);
    }
}

#[test]
fn manifest_edit_without_resign_is_refused() {
    let mut b = std::fs::read(format!("{SPEC}/F01.pvca")).unwrap();
    let needle = b"\"min_runtime\":[0,1]";
    let pos = b.windows(needle.len()).position(|w| w == needle).expect("manifest field present");
    b[pos + needle.len() - 3] = b'0'; // [0,1] -> [0,0]
    let p = tmp("F01_manifest_edit.pvca", &b);
    assert!(pvc_native::loader::load(&p).is_err());
    let (code, err) = run_host(&p);
    assert_eq!(code, 3, "stderr: {err}");
    assert!(err.contains("refusing artifact"), "stderr: {err}");
    assert!(!err.contains("create_window") && !err.contains("event loop"), "host must not reach the windowing layer: {err}");
}

#[test]
fn single_flipped_byte_is_refused() {
    let mut b = std::fs::read(format!("{SPEC}/L01.pvca")).unwrap();
    let n = b.len();
    b[n - 40] ^= 1;
    let p = tmp("L01_flip.pvca", &b);
    let (code, err) = run_host(&p);
    assert_eq!(code, 3, "stderr: {err}");
    assert!(err.contains("refusing artifact"), "stderr: {err}");
}

#[test]
fn control_untampered_gets_past_the_loader() {
    // With no DISPLAY the host must get *past* the loader and fail later, in the windowing layer (exit 1, not 3).
    let (code, err) = run_host(Path::new(&format!("{SPEC}/F01.pvca")));
    assert_eq!(code, 1, "stderr: {err}");
    assert!(err.contains("artifact verified"), "stderr: {err}");
}
