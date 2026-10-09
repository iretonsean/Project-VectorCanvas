//! Tiny wasm32 harness for the D1 cross-target test. No wasm-bindgen: plain C ABI exports driven from
//! node's WebAssembly API. Test tooling only (not a shipped crate).

use pvc_ir::sign::{digest_hex, testkey};

static mut OUT: Vec<u8> = Vec::new();

#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut v: Vec<u8> = Vec::with_capacity(len);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

fn set_out(b: Vec<u8>) -> usize {
    unsafe {
        let out = &mut *std::ptr::addr_of_mut!(OUT);
        *out = b;
        out.len()
    }
}

#[no_mangle]
pub extern "C" fn out_ptr() -> *const u8 {
    unsafe { (*std::ptr::addr_of!(OUT)).as_ptr() }
}

/// terse source -> canonical bytes (length returned; 0 on compile error, error text in OUT).
#[no_mangle]
pub extern "C" fn compile_canonical(ptr: *const u8, len: usize) -> usize {
    let src = unsafe { std::slice::from_raw_parts(ptr, len) };
    let src = String::from_utf8_lossy(src).to_string();
    match pvc_ir::compile_terse(&src) {
        Ok(c) => set_out(pvc_ir::canonicalize(&c.module)),
        Err(d) => {
            set_out(d.iter().map(|x| x.render()).collect::<Vec<_>>().join("\n").into_bytes());
            0
        }
    }
}

/// terse source -> signed artifact bytes with the TEST key.
#[no_mangle]
pub extern "C" fn compile_signed(ptr: *const u8, len: usize) -> usize {
    let src = unsafe { std::slice::from_raw_parts(ptr, len) };
    let src = String::from_utf8_lossy(src).to_string();
    match pvc_ir::compile_terse(&src) {
        Ok(c) => set_out(pvc_ir::sign(&c.module, &testkey::signing_key())),
        Err(_) => 0,
    }
}

/// signed artifact -> 1 if load_signed accepts it under the TEST key, else 0.
#[no_mangle]
pub extern "C" fn verify(ptr: *const u8, len: usize) -> u32 {
    let b = unsafe { std::slice::from_raw_parts(ptr, len) };
    pvc_ir::load_signed(b, &testkey::verifying_key()).is_ok() as u32
}

/// sha256 hex of the bytes (so the harness can cross-check the hash too).
#[no_mangle]
pub extern "C" fn sha256_hex(ptr: *const u8, len: usize) -> usize {
    let b = unsafe { std::slice::from_raw_parts(ptr, len) };
    set_out(digest_hex(b).into_bytes())
}
