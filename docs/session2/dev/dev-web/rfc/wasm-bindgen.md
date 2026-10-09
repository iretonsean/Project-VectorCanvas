# RFC note: `wasm-bindgen =0.2.129` (direct dependency of `pvc-web`)

**What:** the Rust↔JavaScript binding generator for `wasm32-unknown-unknown`. Pre-approved by Supervisor decision D-10.
**Exact version:** `=0.2.129` (the version already resolved in the G7 pin graph via `vello_gpu`'s `webgl` feature, FINDINGS F11). The matching CLI, `wasm-bindgen-cli 0.2.129`, is a build tool installed under `/workspace/dev_work/dev-web/tools` (not a shipped dependency; it pulls `ureq` for its own self-update/test-runner code, which is never linked into `pvc_web.wasm`).
**Why nothing pinned covers it:** no crate in the G7 table can create DOM elements, attach event listeners or call `requestAnimationFrame`. `vello_gpu/webgl` already depends on `wasm-bindgen`'s sibling crates (`js-sys`, `web-sys`); the host needs to call the same APIs for the mirror, the `<input>` overlay and the frame loop.
**What it pulls in:** `wasm-bindgen-macro`, `wasm-bindgen-shared`, `wasm-bindgen-macro-support`, `once_cell`, `cfg-if` (all already in the graph). No HTTP, socket, or LLM crate.
**Hard-wall check:** `cargo tree -e normal --target wasm32-unknown-unknown` is saved at `/workspace/dev_work/dev-web/cargo_tree_wasm32.txt`; `wasm-bindgen-futures` (and with it `tokio`, FINDINGS F12) is **not** in the tree because `pvc-web` does not depend on it.
