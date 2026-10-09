# PVC_CHECKER_NEEDS — what the PVC fidelity checker needs from `pvc_runtime` / `pvc_ir`

Author: dev-baseline. Status: **v1, published at prep stage (D-6). No pilot run has happened.**
Audience: the Orchestrator (to brief dev-render and dev-ir), the Supervisor.

## 0. Why this shape (read first)

The B4 fidelity checks (G2 §3.1, §5, rows L01–L10 and G01–G05) are written **once**, stack-neutral, as predicates over a *Snapshot* of the running app. The Snapshot is a JSON document. The Chromium baseline produces it from the DOM + Chromium's accessibility tree (`baseline/probe-browser.mjs`). **PVC must produce the same JSON from `pvc_runtime`, headless.** The very same predicate code (`core/specs/*.mjs`, `core/universal.mjs`) then runs on both stacks, so "correct" means the same thing for both.

Nothing here needs pixels. It needs the **solved layout, the semantics tree, the text-line breakdown, resolved colours, and an event-dispatch/state loop**, which `pvc_runtime` computes anyway for rendering and for AccessKit.

The checker talks to the runtime through a small **JSON-lines process protocol** (§2), not through Rust linkage, so the checker (Node) and the runtime (Rust) stay decoupled. A Rust trait that the protocol wraps is in §5 for dev-render's convenience. If dev-render prefers a different transport (e.g. a library + thin CLI), only `pvc/probe-pvc.mjs` changes; the Snapshot schema is the contract.

## 1. The Snapshot (the contract)

`snapshot` returns one JSON object:

```jsonc
{
  "meta": {
    "viewport": [1280, 800],        // logical px, as requested
    "text_scale": 1.0,              // as requested (1.0 or 2.0 are used)
    "direction": "ltr",             // "ltr" | "rtl", as requested
    "doc_size": [1280, 1432],       // laid-out content extent in logical px (== scrollWidth/scrollHeight analogue)
    "scroll_overflow_x": 0,         // max(0, content_right_edge - viewport_w); >0 means horizontal scroll would be needed
    "focused": "n17",               // node id with keyboard focus, or null
    "locale": "en"
  },
  "nodes": [ /* pre-order (document/reading order), parents before children */ ]
}
```

One entry per **IR node that has a layout box** (including purely structural `box`/`row`/`column` nodes, because the checks look at cards, columns, grid cells), pre-order:

| field | type | meaning |
|---|---|---|
| `id` | string | stable node id (IR node id; opaque to the checker) |
| `parent` | string \| null | parent node id |
| `kind` | string | IR node kind / `std` component name (diagnostics only) |
| `role` | string \| null | **G4 §8.1 PVC role name** (`button`, `link`, `checkbox`, `switch`, `radio`, `radio-group`, `slider`, `text-field`, `text-area`, `search-field`, `email-field`, `combo-box`, `list`, `list-item`, `tab-list`, `tab`, `tab-panel`, `dialog`, `alert-dialog`, `alert`, `status`, `heading`, `paragraph`, `label`, `image`, `navigation`, `main`, `region`, `form`, `group`, …). `null` for nodes with no semantic role (pure layout boxes) **and** for `decorative` nodes |
| `name` | string \| null | accessible name after strings-table resolution (null if none) |
| `name_source` | string \| null | `"label"` (strings key / visible label relation), `"aria"` (explicit name/`labelled-by`), `"content"` (derived from text), `"placeholder"`, `"none"` |
| `states` | object | any of `checked` (true/false/"mixed"), `disabled`, `selected`, `expanded`, `pressed`, `required`, `invalid`, `busy`, `level` (heading level), `live` (`off`/`polite`/`assertive`), `value` (text value of an input as typed so far; number for ranges), `min`, `max`. Absent = not applicable |
| `a11y_hidden` | bool | true if the node is excluded from the AccessKit tree (decorative or hidden) |
| `visible` | bool | participates in layout with non-zero box and is not `display:none`-equivalent (hidden nodes may be omitted entirely; if omitted, "absent" is treated as "hidden") |
| `text` | string | the node's **own** text content after strings resolution, whitespace-collapsed and trimmed, `""` if none. A `text` node's own text is its string; a container's own text is `""`. |
| `lines` | `[[x,y,w,h],…]` | one rect per **laid-out text line** of this node's own text (document coordinates, logical px). `[]` if no text |
| `line_chars` | `[n,…]` | number of characters (Unicode scalar values, excluding the trailing break/space) on each of those lines |
| `rect` | `[x,y,w,h]` | border box in document coordinates, logical px, after layout at the requested viewport/text scale |
| `visible_frac` | number 0–1 | fraction of the node's `lines` (or `rect` if no text) that is not cut off by an ancestor clip (`scroll`/clip container) or by the horizontal viewport bounds `[0, viewport_w]`. Vertical extent beyond the viewport does **not** count as cut off |
| `focusable` | bool | can receive keyboard focus |
| `style.fg` | `[r,g,b,a]` 0–255 / 0–1 | resolved text colour (final colour after theme/token resolution) |
| `style.bg` | `[r,g,b,a]` \| null | this node's **own** fill colour (null = none) |
| `style.bg_unknown` | bool | own fill is a gradient/image that cannot be reduced to one colour |
| `style.border` | `{"w":px,"color":[r,g,b,a]}` \| null | uniform border |
| `style.radius` | number | largest corner radius, px |
| `style.font_size` | number | px **after** text scale |
| `style.font_weight` | number | 100–900 (bold = 700) |
| `style.shadow` | bool | has an elevation shadow |
| `visual` | `"image"` \| `"graphic"` \| `null` | `image`: raster/`image` node; `graphic`: vector `shape`/icon/svg-like leaf. `null` otherwise |
| `native` | bool | control drawn by the platform (always `false` for PVC; present for the DOM) |

Rules:
* Pre-order, parents first; children follow their parent contiguously; coordinates are **document** coordinates (origin = top-left of the scrollable content), not viewport coordinates.
* Colours are the *final* colours after theme/token resolution; the checker composites alpha itself.
* `rect` for text-bearing nodes is the node's box; `lines` is the tight per-line text boxes (Parley line boxes are fine).
* Direction: with `direction:"rtl"`, layout sugar (`row`, `align`, anchors "start/end") must resolve direction-relatively (G4 §5.3) and the returned rects must reflect the mirrored layout.
* Text scale: `text_scale` multiplies **all** text sizes; layout re-solves.

## 2. Process protocol (`pvc-probe`)

Binary: `pvc-probe <file.pvc>` (dev-render; may be a `[[bin]]` in `pvc-runtime` or a tiny crate depending on `pvc_ir` and `pvc_runtime`; no window, no GPU, no network). Reads one JSON request per line on stdin, writes exactly one JSON response line per request on stdout. All requests carry `"op"`. Responses: `{"ok":true,…}` or `{"ok":false,"error":"<text>"}`.

| op | extra fields | effect / response |
|---|---|---|
| `load` | `locale` (default `"en"`), `fresh_storage` (bool) | parse, validate, **verify signature or accept unsigned in checker mode**, build the app instance. Response `{"ok":true,"diagnostics":[…]}`. If validation errors exist, `ok:false` with the diagnostics list (code, severity, path, message, fix). The checker prints these as the "validator tool" output. Implicitly sent once at start |
| `snapshot` | `viewport:[w,h]`, `text_scale`, `direction` (all optional; they persist until changed) | lay out and return `{"ok":true,"snapshot":<Snapshot §1>}` |
| `click` | `id` | pointer press+release at the centre of the node's current `rect`, **hit-tested** (so an occluded node is not clicked). Response `{"ok":true,"hit":"<id>"}` |
| `type` | `id`, `text`, `replace` (bool) | focus the node as a pointer click would, then commit `text` through the **same code path as IME-commit/keyboard text input** (`replace:true` selects all first). One call = one committed string; per-character `input`/`change` events are expected |
| `type_focused` | `text` | commit `text` to the node that currently has keyboard focus (no pointer click); used by the keyboard-only replay (U2) |
| `key` | `key` ∈ `Tab`, `Shift+Tab`, `Enter`, `Space`, `Escape`, `ArrowUp/Down/Left/Right`, `Home`, `End`, `Control+a` | deliver to the focused node; Tab/Shift+Tab move focus in focus order (reading order, direction-aware). Response includes `"focused"` |
| `focused` | — | `{"ok":true,"focused":"<id>"|null}` |
| `blur_all` | — | clear focus so the next `Tab` starts from the beginning |
| `new_session` | — | discard in-memory state and rebuild the app instance **keeping persisted app storage** (the `storage` capability's data) — the analogue of "close the tab and open a new one" |
| `reset` | — | discard all state **and** persisted storage |

State changes are applied synchronously: after the response of `click`/`type`/`key`, a following `snapshot` shows the post-event state (effects such as the declared `connect_user` stub may be inert, never networked).

Timers/animation: none of L01–L10, G01–G05 uses them; no clock control is needed.

## 3. Validator interface (`pvc_ir`)

`check-pvc` first runs `pvc-validate --json <file.pvc>` (or `pvc-probe`'s `load`) and needs, per diagnostic: `code`, `severity` (`error`/`warning`/`info`), `path`, `message`, `fix_hint`. Any `error` ⇒ the specimen fails with the standardised `BUILD` item (the validator text is the "tool output" for the PVC arm, as html-validate/tsc are for the baseline). Needed rule set for the pilot specimens is dev-ir's (D-2); from my side the only requirement is that valid files load.

## 4. What each specimen needs from the runtime (so dev-render can scope the headless solver)

Sizes in viewport px; all A-items are evaluated from the Snapshot at one or more of: 320 / 480 / 768 / 1024 / 1280 wide, 320 wide at `text_scale` 2.0, and `direction:"rtl"` (L08).

| Specimen | Runtime capability the A-items observe |
|---|---|
| L01 | card, avatar (circle), row/column sugar, text weight/size, full-width button, fill colours; roles `button`, text |
| L02 | breakpoint-driven reflow 3 columns ↔ 1 column; `list`/`list-item`; `button`/`link`; paint (fill/border) on cards |
| L03 | breakpoint reflow (side-by-side ↔ stacked), `heading` level 1, `image`/`graphic` leaf |
| L04 | landmarks (`navigation`, `main`, and a way to express header/aside/footer landmarks — **see §6 item 1**), breakpoint hiding of sidebars, a disclosure control (`button` with `expanded`) that reveals the nav on `press`, sticky-footer (footer at the viewport bottom on short content) |
| L05 | `heading` levels, text wrapping with a max reading width (`line_chars`), `paragraph` roles |
| L06 | rows of label + control with end-alignment, `heading` per group; controls `checkbox`/`switch`/`slider`/`text-field`/`combo-box` |
| L07 | 4-up → 2×2 reflow, equal-height panels, `search-field` focusable and accepting typed text (`states.value`) |
| L08 | direction-relative anchoring (`direction:"rtl"` mirrors), decorative icon (`a11y_hidden`), `link` role |
| L09 | 4 link columns ↔ stack/accordion (disclosure buttons with `expanded`, hidden content), icon-only links with names, labelled `email-field` (`name_source` = `label`/`aria`) |
| L10 | grid 3×2 ↔ 1×6 with equal gaps, `heading` per cell, decorative or named icons |
| G01 | boolean/session state, list state with `append`/`remove-where`, guard on dismissal, `new_session` that keeps persisted storage |
| G02 | `list<record>` ops (`append`, `update-where`, `remove-where`), derived counts shown as text, filter enum, per-row `checkbox`+`button` |
| G03 | `tab-list`/`tab`/`tab-panel` with `selected`, dirty flag, `dialog`/`alert-dialog` node appearing on guarded transition, cancel/confirm buttons |
| G04 | `decimal` (fixed-point) arithmetic, `stepper`/`text-field` quantity input, promo-code `text-field` + button, error text appearing |
| G05 | enum step state with skip logic, `radio`/`radio-group` or buttons, back-stack |

All specimens use only the G4 v0 node kinds plus responsive/direction behaviour; no pixels, no scrolling beyond `scroll_overflow_x`, no animation.

## 5. Rust trait sketch (optional convenience; the protocol is the contract)

```rust
pub trait CheckerHost {
    fn load(&mut self, ir: &[u8], locale: &str, fresh_storage: bool) -> Result<Vec<Diagnostic>, Vec<Diagnostic>>;
    fn snapshot(&mut self, viewport: (f32, f32), text_scale: f32, rtl: bool) -> Snapshot;   // §1
    fn click(&mut self, id: &str) -> Option<String>;                 // returns hit-tested node id
    fn type_text(&mut self, id: &str, text: &str, replace: bool);
    fn type_focused(&mut self, text: &str);
    fn key(&mut self, key: Key) -> Option<String>;                   // returns focused id
    fn focused(&self) -> Option<String>;
    fn blur_all(&mut self);
    fn new_session(&mut self);
    fn reset(&mut self);
}
```

## 6. Things I flag (inconsistencies / decisions that need an owner)

1. **Landmarks.** G2 L04 requires five landmark regions (header, nav, main, right sidebar, footer). **G4 §8.1's role table has no `banner`, `contentinfo` or `complementary`** (only `navigation`, `main`, `region`, `form`). My spec therefore accepts, identically on both stacks, either the ARIA landmark role **or** a `region` whose name matches /header|banner/, /footer/, /sidebar|aside|related|complementary/ respectively. dev-ir/dev-render: either add the three roles to v0 or confirm `region`+name is the intended encoding.
2. **Text scale on the baseline** is applied as `html{font-size:200%}` (rem-relative text scales; px-sized text does not), the nearest thing a web page's author can be tested against. PVC scales all text. This slightly favours the baseline on U5 and is stated in the spec.
3. **Intent fixtures.** G2's one-line intents for G01–G05 (and L-rows that need strings) do not contain the data a state trace needs (item names, prices, tax rate, promo code, step titles). `checklists/<ID>.md` §"Fixture" lists the exact sentence(s) that **must be appended to the intent text for both stacks** (dev-pilot). Without them the checks cannot be evaluated.
4. `new_session` semantics (G01): in-memory state discarded, persisted storage kept. Web analogue: a new tab in the same browser context (fresh `sessionStorage`, shared `localStorage`).

## 7. Change log
* v1 — initial.
* v1.1 — added `type_focused` and `Control+a`; snapshot response is wrapped as `{ok,snapshot}`; `new_session` keeps the same process (the checker does not restart `pvc-probe` for a new session). The checker also passes `fresh_storage:true` on the first `load` of every run.
