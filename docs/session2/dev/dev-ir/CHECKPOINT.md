# dev-ir CHECKPOINT — `pvc_ir` API is usable (2026-10-08)

Crate: `/workspace/dev_work/dev-ir/pvc-ir/` (name `pvc-ir`, lib `pvc_ir`). Read-only path dependency:

```toml
pvc-ir = { path = "/workspace/dev_work/dev-ir/pvc-ir" }
```
Deps: `ed25519-dalek =2.2.0`, `sha2 =0.10.9`, `icu_normalizer =2.3.0` (RFCs in `rfc/`). No serde, no net. Builds native and wasm32.

**Status:** `cargo test` green (see REPORT later for the saved log). **Still missing at this checkpoint:** GRAMMAR.md (being written), wasm32 D1 run, REPORT. The AST below is stable for L01/F01; Part 2 (pilot constructs) will only *add* enum variants/fields (`NodeKind`, `Role`, `Expr`, `Action`...) and I will append a dated note here if any existing field changes.

## Specimens (real signed IR to load)
| file | what |
|---|---|
| `/workspace/dev_work/dev-ir/specimens/L01.pvc` / `.pvca` / `.canonical.json` | profile card: terse source / **signed artifact** / canonical bytes (for diffing) |
| `/workspace/dev_work/dev-ir/specimens/F01.pvc` / `.pvca` / `.canonical.json` | sign-up form, en + zh-CN |

Test key (TEST ONLY, seed is public, defined in `pvc_ir::sign::testkey`): public key hex
`203dab1b2df03844a5267222437297aa13837f0ab154c69c3f43ed38dd11f3f0`.
`pvc_ir::sign::testkey::verifying_key()` returns it; no key file is needed.

## Public API (all in `pvc_ir::` unless noted)
```rust
// terse -> canonical Module
pub fn parse_terse(src: &str) -> Result<Module, Vec<Diagnostic>>;      // PARSE/SCHEMA/UNKNOWN-KIND/UNKNOWN-TOKEN
pub fn compile_terse(src: &str) -> Result<Compiled, Vec<Diagnostic>>;  // parse + validate; Compiled{module, diagnostics(warnings/infos)}
pub fn validate(m: &Module) -> Vec<Diagnostic>;                        // REF/A11Y-NAME/CAP-UNDECLARED/STRING-MISSING/TYPE/...
// canonical bytes + signing
pub fn canonicalize(m: &Module) -> Vec<u8>;                            // the one byte form (JSON, fixed order, ints only, NFC)
pub fn from_canonical(b: &[u8]) -> Result<Module, CanonError>;
pub fn sign(m: &Module, key: &ed25519_dalek::SigningKey) -> Vec<u8>;   // signed artifact bytes
pub fn load_signed(bytes: &[u8], trusted: &ed25519_dalek::VerifyingKey) -> Result<Module, LoadError>; // refuses bad sig / other signer / non-canonical
pub mod sign { pub fn digest_hex(canonical:&[u8])->String; pub mod testkey { signing_key(), verifying_key(), public_hex() } }
// reference semantics (use it, do not re-implement predicates)
pub mod eval { State::init(&Module); eval(&Module,&State,&Expr)->Result<Value,_>; eval_bool(..); run_handler(&Module,&mut State,&Handler)->Result<Outcome{ran,emitted,submits},_> }
pub mod pred { is_email(&str), is_empty(&str), length(&str), nfc(&str) }
```
Runtime loading is: `let m = pvc_ir::load_signed(&std::fs::read(path)?, &pvc_ir::sign::testkey::verifying_key())?;`
(wasm: `include_bytes!`/fetch the `.pvca`, same call.) Re-run `validate(&m)` if you want diagnostics on a loaded module.

## The `Module` AST (src/ast.rs) — how to walk it
```
Module { header, manifest{locales(first=default), fonts[FontEntry{family,weight,script,sha256,file,index}], caps[Capability{id,kind,hosts,methods,..}], viewport, text_scale},
         theme{name, colors[(token,[r,g,b,a])]}, strings{tables[LocaleTable{locale, entries[(key,text)] sorted}]},
         state[StateDecl{name,ty,init}], derived[Derived{name,expr}], effects[Effect{name,cap}], root: Node, provenance (not canonical) }
Node { id, kind: Surface|Box|Text(TextProps)|TextInput(InputProps), layout, paint, sem, show: Option<Expr>, on: Vec<Handler>, children }
```
* `m.walk(&mut |node, depth| ..)` document-order DFS (also the a11y tree order); `m.find_node(id)`; `m.string(locale,key)`; `m.resolve_color(&Color)->Option<[u8;4]>`; `m.default_locale()`.
* **Lengths** are `Len(i32)` in **1/64 logical px** (`.px()` gives f32; `Len::from_px(i32)`). No floats anywhere in the AST.
* **Layout** (`Layout{dir Row|Col, gap, pad:Edges, align Start|Center|End|Stretch, justify Start|Center|End|Between, w,h: Size}`), `Size = Hug | Fill | Px(Len)`. It is the *explicit sugar record* (D-4: no constraint list, no Cassowary). Semantics, which dev-render implements:
  * flex-like stack. `dir` is the main axis. `align` is cross-axis placement; `justify` main-axis distribution.
  * `Px` = that size; **in a `Col`/surface parent a fixed `w` is capped at the parent's content width** (so the 320-wide card shrinks on a 320 viewport).
  * `Fill`: main axis → equal share of the space left after rigid siblings and gaps (min 0); cross axis → stretch to the parent's content box.
  * `Hug` = content size. A `Hug` text/row/col in a column wraps at the available width (text height depends on width).
  * Surface = root, `Fill`/`Fill` of the viewport, 24 px pad, `align Center` (children horizontally centred), `justify Start`.
  * Padding is inside the node's box; `Paint.border` is drawn inside the box edge and does **not** change layout (width 1 px = 64 units).
* **Paint:** `fill: Option<Color>`, `border: Option<{width,color}>`, `shape: RoundedRect(Len radius) | Circle`. Colours are `Token(name)` (resolve via theme) or `Rgba`.
* **Text:** `TextProps{key (strings key), family "ui", size, weight 400|700, line_height, color, align}`. Fonts: manifest `fonts` (family `ui`; entries with `script: Some("Hans")` are CJK fallbacks; hash = authoritative; `file`/`index` are hints). **The font hashes in the specimens are those of the system files** `DejaVuSans.ttf`, `DejaVuSans-Bold.ttf`, `NotoSansCJK-Regular.ttc` (index 2 = SC), `NotoSansCJK-Bold.ttc` (index 2). If dev-render bundles different files, tell me and I will re-hash and re-sign (one command: `pvc-compile`).
* **TextInput:** `InputProps{variant Single|Email|Password, bind: state field (text), masked: Expr (true ⇒ draw bullets)}`. The runtime writes the edited string to `bind` and then runs the node's `Change` handlers. Enter in a single-line input raises `Submit` on the nearest ancestor that has a `Submit` handler (F01: the `form`).
* **Semantics:** `Semantics{role, decorative, name: Option<Name::Key(k)|FromNode(id)>, heading_level, live, required, disabled/checked/pressed/invalid: Option<Expr>, labelled_by, described_by[], error_message}`. `Role::aria()` gives the web-mirror role; AccessKit mapping is yours (G4 §8.1). Rules:
  * `decorative` nodes are **not** in the a11y tree (text inside buttons/avatar/checkbox is decorative: the control carries the name).
  * `Name::FromNode(id)`: name = that text node's string (avatar ← "Jane Doe").
  * Text nodes: name = their own string. Interactive roles (`Role::is_interactive`) are focusable, in document order.
  * `show: Some(expr)`: while false the node and subtree are not laid out, painted, hit-tested or in the a11y tree (error text, checkbox mark, done message).
  * `disabled` evaluating true ⇒ ignore press/focus-activate; paint at **40 % opacity** (runtime rule; there is no opacity field in v0).
  * Checkbox: `checked` expr is `Var(bind)`; Press handler toggles it (explicit in the AST). Password toggle: `pressed` = `Var(show_pw)`, Press handler flips it.
* **Behaviour:** `state` (typed `Bool|Int|Text|Enum`), `derived` (named pure exprs, read via `Expr::Var`), `Handler{event Press|Change|Submit, guard: Option<Expr>, actions: Assign(field,expr) | Emit(effect) | Submit(node-id)}`. Use `eval::run_handler`. **`Emit` goes to a dispatcher stub that only logs. Never a network call.**
* IDs are stable/deterministic: `surface0/card0/row0/avatar0`, std children `…/label`, `…/input`, `…/error`, `…/toggle`.
* Strings: every user-visible string is a key in `strings`; inline terse strings get keys `<node-id>:<slot>`.

## Minimal example (L01 shape)
```rust
let bytes = std::fs::read("/workspace/dev_work/dev-ir/specimens/L01.pvca")?;
let m = pvc_ir::load_signed(&bytes, &pvc_ir::sign::testkey::verifying_key())?;
m.walk(&mut |n, depth| println!("{}{} {:?}", "  ".repeat(depth), n.id, n.sem.role));
let mut st = pvc_ir::eval::State::init(&m);   // F01: apply handlers with eval::run_handler
```
Dump a specimen's tree: see `specimens/*.canonical.json` (pretty-print with `python3 -m json.tool`).

## CLIs (build with `cargo build` in `pvc-ir/`)
`target/debug/pvc-validate file.pvc` (diagnostics, one per line, exit 1 on errors) and `target/debug/pvc-compile in.pvc out.pvca [--canonical out.json]` (signs with the TEST key).

## Not yet / known limits
* No `slot`, `scroll`, `grid`, `image`, `shape`, `layer`, `view` yet (Part 2 adds only what L02–L10/G01–G05 need).
* No raw constraints (D-4), no ICU plural forms, no `timer`, no `hover-*` events.
* Contrast/target-size/reflow validator rules are not implemented (only the codes listed in the brief + TYPE, ID-COLLISION, UNKNOWN-TOKEN, A11Y-ACTION, W-SYSTEM-FONT). U3 for the specimens is checked in `tests/specimens.rs`.

## Update 2026-10-08 (late): `fonts std`
Terse accepts `fonts std` = the four faces above (DejaVu Sans 400/700, Noto Sans CJK SC 400/700, hashes of the system files). Canonical bytes of L01/F01 are unchanged by it (committed `.pvca` still equal a fresh compile). Reason: writing the four `font … sha256=<64 hex>` lines costs ~250 o200k tokens per module, which the agent should not pay. `src/expand.rs::STD_FONTS` is the table; if dev-render bundles other files, only that table and the specimens' artifacts change.

## Update 2026-10-09 — Part 2 (pilot constructs, L01–L10 + G01–G05): AST additions
A1 artifacts (`specimens/*.pvca`) are **byte-identical** after these changes (all additions are omitted from the canonical form when absent). All additions are *new enum variants / new fields*; existing field meanings are unchanged. Code that matches `NodeKind`, `Role`, `Expr`, `Action`, `Ty`, `Value`, `InputVariant`, `Dir`, `Func` exhaustively needs new arms (tell me if you want `#[non_exhaustive]`).

Reference programs for every pilot specimen (compile with `pvc_ir::compile_terse`): `/workspace/dev_work/dev-ir/pilot-refs/{L01..L10,G01..G05}.pvc`. Construct → specimen map: `PILOT_COVERAGE.md`. Build the CLI with `CARGO_TARGET_DIR=/workspace/cargo-target cargo build` in `pvc-ir/` (`/workspace/cargo-target/debug/pvc-validate`).

**New node kinds** (`NodeKind`): `Shape` (painted leaf, role `Image`, `decorative` or named), `Layer` (out-of-flow dialog overlay, centred over the surface while `show` is true, traps focus; role `Dialog`/`AlertDialog`, named), `Slot(SlotProps{on: Expr, cases: Vec<String>})` (children[i] is the variant for `cases[i]`; exactly the child whose case equals the value of `on` is present; enum value or `true`/`false`), `View(ViewProps{list, alias, filter})` (the children are the *template*: one instance per row of state list `list` passing `filter`, row bound to `alias`; node ids inside a template are the template's ids, instances need a host-derived suffix).
**Layout:** `Dir::Grid` + `Layout.cols: u16` = N equal `1fr` columns (auto rows; row height = tallest cell; **every grid child is stretched to its cell in both axes** unless it has a fixed size). `Node.at: Vec<AtRule>`: breakpoint overrides: `Bp::Md` applies when viewport width ≤ 768 px, `Bp::Sm` when ≤ 480 px; apply `Md` then `Sm` (Sm wins). `AtRule{bp, patch: LayoutPatch{dir,cols,gap,pad,align,justify,w,h: Option<_>}, show: Option<bool>}` (`show: Some(false)` removes the node at that breakpoint). Direction (RTL) mirroring of `Row`/start/end is the runtime's job.
**Roles added:** `Radio, RadioGroup, TextArea, SearchField, List, ListItem, TabList, Tab, TabPanel, Dialog, AlertDialog` (`Role::aria()` maps them). `InputVariant::{Search, Multiline}` (roles search-field / text-area). `Semantics.selected: Option<Expr>` (tabs).
**Text:** `TextProps.args: Vec<(String, Expr)>` and `.value: Option<Expr>`. Content = `eval::text_content(&Module,&State,&Scope,locale,&TextProps)`: `value` shown verbatim, else the strings entry with `{name}` holes filled from `args` (`{{`/`}}` literal). Int → digits, **decimal → exactly 2 fraction digits**.
**State types:** `Ty::Decimal` / `Value::Decimal(i64 hundredths)`; `Ty::List{fields, max}` / `Value::List(rows)` (rows = `Vec<(field, Value)>` in declared order; `max` mandatory).
**Expressions:** `Expr::Dec`, `Add/Sub/Mul` (int/int, dec/dec, int*dec; int is promoted in `+ - ==  <` with a decimal; no dec*dec), `Func::{Count, Sum, Percent}`. `count(list)`, `count(list, pred)`, `sum(list, expr)` bind each row as `it` (`it.field` is `Expr::Var("it.field")`). `percent(x: decimal, p: decimal|int)` = round-half-away-from-zero(x·p/100) in i128 (`8.25` = 8.25 %, `10` = 10 %). Row fields are `Expr::Var("alias.field")` (dotted name; `alias` bound by the enclosing `View`). `is-empty`/`length` accept lists.
**Actions:** `Append(list, [(field, expr)])` (ignored when the list is at `max`; omitted fields get type defaults), `Remove(alias)` (removes the bound row), `OpenUrl(url)` (through the `open-url` capability; dispatcher stub only logs), and `Assign("alias.field", expr)` updates the bound row in place. **Multiple handlers per event are allowed and all run in order** (each with its own guard; a later guard sees earlier assignments).
**Capabilities:** `open-url` (`Capability.hosts` holds its URL *prefixes*; terse `cap open-url.main urls=[…]`). A `link … href="…"` is a press handler `OpenUrl`; validator rule `PVC-E-CAP-UNDECLARED`/`PVC-E-CAP-SCOPE`.
**Reference evaluator (use it):** `pvc_ir::eval::{State, Scope, eval_in, eval_bool_in, run_handler_in, text_content, format_value, template_holes}`; `Scope::with(alias, list, row_index)` binds a `view` row. `Outcome{ran, emitted, submits, opened, removed, appended}`. `tests/pilot_refs.rs` drives G01–G05 end to end with it (state-level versions of the G2 checklists).
**Runtime rules (not in the IR):** `view` instances: filter is evaluated per row; `slot` shows one child; `layer` overlays; disabled → 40 % opacity; `Tab`/arrows for tab/radio groups are yours.
**Diagnostics (new):** `PVC-E-A11Y-IMG, PVC-E-A11Y-CONTRAST, PVC-E-UNBOUNDED, PVC-E-CAP-SCOPE, PVC-E-A11Y-ACTION` extended to link/tab/radio, `PVC-W-SLOT-INCOMPLETE`; `pvc-validate --json` prints `{ok, diagnostics:[{code,severity,line,col,node,path,message_key,params,fixes,message,fix_hint}]}`.

## Update 2026-10-09 (late): font hashes re-pointed to dev-render's bundle
`fonts std` (src/expand.rs `STD_FONTS`) now names: DejaVuSans.ttf, DejaVuSans-Bold.ttf (hashes unchanged), `NotoSansCJKsc-Regular.otf` sha256 `6ead94f3…306c60` index 0, `NotoSansCJKsc-Bold.subset.otf` sha256 `a44805f5…c3c5f9` index 0 (as in dev-render/CHECKPOINT.md). L01/F01 `.pvca` and `.canonical.json` re-signed: canonical L01 7821 B sha256 `51dfc0bd…bb43a`, F01 18693 B sha256 `d1519615…01c`. The pilot references also use `fonts std`, so their canonical bytes changed likewise (still equal native vs wasm32). Bold CJK is subset to the L01+F01 strings: other pilot text in bold Chinese would fall back/synthesise (pilot is en only).
