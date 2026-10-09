# RFC note: `web-sys =0.3.106` (direct dependency of `pvc-web`)

**What:** generated bindings to Web APIs. Pre-approved by D-10.
**Exact version:** `=0.3.106` (already in the graph through `vello_gpu`'s `webgl` feature).
**Why nothing pinned covers it:** the hidden DOM/ARIA mirror and the `<input>` overlay (G4 §8.3, G6 §6) are DOM code; no G7 crate provides DOM access. `vello_gpu` enables only the WebGL2 subset of web-sys features it needs.
**Features enabled by `pvc-web`** (each is a Cargo feature gate, not extra code): `Window, Document, Element, HtmlElement, HtmlInputElement, HtmlCanvasElement, Node, NodeList, CssStyleDeclaration, Event, EventTarget, KeyboardEvent, CompositionEvent, InputEvent, FocusEvent, FocusOptions, MouseEvent, PointerEvent, Performance, DomRect, HtmlCollection, WebGl2RenderingContext, console`. Notably absent: `Request`, `Response`, `XmlHttpRequest`, `WebSocket`, `Fetch*`, `RequestInit`, `EventSource`, `Navigator.sendBeacon` — the crate cannot call the network APIs even in principle (the page's own JS bootstrap `www/boot.js` performs the same-origin static-asset `fetch`es for wasm, artifact and key).
**What it pulls in:** `js-sys`, `wasm-bindgen`. No HTTP/socket/LLM crate.
**Hard-wall check:** see `/workspace/dev_work/dev-web/cargo_tree_wasm32.txt`.
