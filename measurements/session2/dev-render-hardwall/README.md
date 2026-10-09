# Hard-wall check (COMMON §3): no HTTP / socket / LLM client crate in pvc-runtime's normal dependency tree
Commands (run in `pvc-runtime/`, `CARGO_TARGET_DIR=/workspace/cargo-target`):
- `cargo tree -e normal --features cpu,gl --target x86_64-unknown-linux-gnu` → `cargo_tree_native_cpu_gl.txt`
- `cargo tree -e normal --features webgl --target wasm32-unknown-unknown` → `cargo_tree_wasm32_webgl.txt`
- `cargo tree -e normal --target x86_64-unknown-linux-gnu` (no features) → `cargo_tree_native_default.txt`
Grep over all three for `reqwest|hyper|ureq|curl|openai|llm|tokio|socket|h2 |http|rustls|native-tls|isahc|surf |attohttpc|mio |async-std` → no match (checked).
Source grep (`grep -rniE "reqwest|hyper|openai|llm|tokio::net|TcpStream|UdpSocket|std::net" pvc-runtime/src`) → see source_grep.txt.
Note: the `gl` feature pulls wgpu (GL backend via glow/EGL loaded dynamically): a GPU API, not a network crate. FINDINGS F12: tokio appears only in Cargo.lock via wasm-bindgen-futures (not in this normal tree).
