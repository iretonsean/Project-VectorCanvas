# RFC note: `js-sys =0.3.106` (direct dependency of `pvc-web`)

**What:** bindings to JavaScript's built-in objects (`Array`, `Object`, `Reflect`, `Function`). Pre-approved by D-10.
**Exact version:** `=0.3.106` (already in the graph through `vello_gpu`'s `webgl` feature).
**Why nothing pinned covers it:** the host publishes `window.__pvcFrames` rows and `__pvcMeta` (dev-measure METHOD.md §3, §11) as plain JS objects, and creates the WebGL context options object. `serde` is forbidden in runtime/host crates, so `js_sys::Object`/`Reflect` is the dependency-free way to build them.
**What it pulls in:** `wasm-bindgen`. No HTTP/socket/LLM crate.
**Hard-wall check:** see `/workspace/dev_work/dev-web/cargo_tree_wasm32.txt`.
