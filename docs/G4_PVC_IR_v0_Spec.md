# G4 — PVC-IR v0: Conceptual Specification

**Gate:** G4 in §8 of `PVC_Adversarial_Analysis_and_Architecture.md` ("PVC-IR v0 spec on paper").
**Status:** draft for supervisor red-team, 2026-10-08. **Normative level:** conceptual. This document fixes *what the IR must express and guarantee*. It does not fix a concrete grammar, a data layout, or an implementation. The snippets are illustrative and **not normative syntax**. The concrete grammar is a build-session deliverable, and it must conform to this document.
**Depends on:** A§4.1 (hard wall), A§4.2 (scene algebra), A§4.4/A§4.4.1 (tiers), A§4.5–A§4.7; decisions Q1, Q4–Q7; G7 S.1, S.3, S.7.
**Terminology owner:** this document. G1, G2, G5 and G6 use its terms and diagnostic codes.
**Reference convention:** `A§n` = the main analysis document; bare `§n` = this document; `[§9.n]` = the analysis's verified findings; `[G7 S.n]` = G7's supplementary verified findings; `G5 §n` etc. = other gate documents.

---

## 1. What PVC-IR is, in one paragraph

PVC-IR is **the program**. A coding agent, any agent and hosted or local (Q1, A§4.7.1), turns a person's intent into PVC-IR. The **validator** checks it and either accepts it in canonical form or returns structured diagnostics the agent can repair. The **runtime** on the end user's device renders the canonical, signed IR deterministically, offline, with no language model (A§4.1). The IR has five layers: **structure** (node tree), **layout** (constraints), **paint** (SDF algebra plus raster), **behaviour** (typed state transitions and capability-scoped effects), and **semantics** (roles, names, states for accessibility and explanation). It also carries a **manifest** (versions, capabilities, fonts, theme) and a **strings table** (all human-language text, kept apart from structure).

## 2. Design principles (each is a constraint, and each names what it rules out)

| # | Principle | Rules out |
|---|---|---|
| P1 | **Hard wall.** Nothing in the IR requires interpretation at runtime. No field may hold prose that the runtime must understand. Strings are data to display, never instructions | A runtime LLM; "smart" fields such as `layout: "make it look nice"` |
| P2 | **Validator is the source of truth** (A§4.7.1). Validity is whatever the validator accepts, and it is decidable. The validator is deterministic and makes no network calls | Validity that depends on a model's judgment; schema-only checking without semantic rules |
| P3 | **Two forms, one meaning.** Agents write the **terse form**: defaults omitted, standard components and theme tokens used. The validator expands it to the **canonical form**: fully explicit, ordered, hashable, and signed. The two are equivalent by construction, and only the canonical form ships | One verbose form that costs tokens every time an agent writes it (§2.1 explains why this is the token thesis); a terse-only form that cannot be signed or diffed reliably |
| P4 | **Language-neutral structure.** Every human-visible string sits in a per-locale strings table referenced by key. Identifiers are opaque tokens, not English words with meaning | English-only programs; i18n added later (A§4.7.3) |
| P5 | **Semantics are not optional.** Every node that is visible and meaningful carries a role. Every interactive node has an accessible name. The validator enforces both | Drawing lists with no a11y tree (A§3 risk 4) |
| P6 | **Capabilities are declared, not inferred.** Each effect names a capability declared in the manifest. The runtime enforces it at one dispatcher (A§4.6) | Model-judged permissions; ambient authority |
| P7 | **Ambiguity is visible.** When the IR records a default that came from an ambiguous phrase, it records it as an *acknowledged* or *unacknowledged* default with a diagnostic. A release build may contain no unacknowledged defaults | Silent guesses baked into shipped apps (A§2.1) |
| P8 | **Tier-independent meaning.** Tiers may differ only in documented paint effects (§12). Layout, text content, semantics, hit-testing, behaviour and capability enforcement are identical on every tier | GPU-only features without a floor (A§6 item 1) |
| P9 | **Extensible by addition.** New product classes arrive as namespaced node kinds behind the same validator and capability model (Q7) | Forking the IR per product class |

### 2.1 Why P3 is the token thesis (read this before G2)

The README compares *HTML that an agent writes* with *prose that a human writes*. Under the hard wall that comparison does not apply. In both stacks the human writes roughly the same request, and the **agent** writes the program: HTML/CSS/JS in the legacy stack, PVC-IR in PVC. PVC therefore saves tokens only in four places:

1. **Agent output.** Terse IR must be shorter than the equivalent legacy code. Standard components, theme tokens and defaults carry the load.
2. **Repair turns.** A validator diagnostic with a machine-applicable fix is cheaper than a screenshot, a bug report and a re-prompt.
3. **Edit context.** To change an app, the agent reads the terse IR of the affected subtree, not a component tree spread across many files plus a CSS cascade.
4. **Fixed overhead.** This one works *against* PVC. The schema summary, standard library and dialect pack must sit in the agent's context. That is a per-session cost the legacy stack does not pay, because models already know HTML.

G2's metric counts all four. A single illustrative data point, not evidence: the README's profile card, written as terse IR by hand in §15, is **118 tokens** (`o200k_base`), against **185** for the README's HTML [G7 S.4]. That is 36% fewer, before item 4. If item 4 turns out to cost more than items 1–3 save over a realistic session, the vision's "fewer tokens" claim fails, and no amount of renderer quality rescues it. This is the single largest open risk in the concept, and §16 says so.

## 3. Document model

A PVC-IR **module** has these parts. All are required in canonical form; in terse form most may be omitted and take defaults.

| Part | Contents | Notes |
|---|---|---|
| Header | IR spec version (major.minor), standard-library version, dialect-pack ID + version + signature digest, minimum runtime version | Pinning (§11) |
| Manifest | Capability declarations (§10), resource boundaries, font set (by content hash), theme reference, supported viewport range, supported locales | Signed together with the body |
| Theme | Design tokens: colour roles, spacing scale, type scale, radii, elevation, motion durations | Agents refer to tokens, not raw values, so terse IR stays short and consistent |
| Strings table | Per-locale map of key → text, with ICU-style plural/select forms | P4 |
| State schema | Typed state record and initial values (§7) | |
| Node tree | One `surface` root per window or screen, containing nodes (§4) | Every node has a stable ID |
| Behaviour | Event handlers, guards, updates, effect bindings (§7) | |
| Acknowledgements | Defaults the author accepted on purpose, each tied to the diagnostic it silences (§9.2) | P7 |
| Provenance (optional, stripped on publish) | Per-node link to the intent source: source hash + span, never the prose itself | Lets the explainer and the developer trace a node back to the sentence that produced it. Stripped on publish so shipped artifacts carry no authoring text (G5) |

**Stable node IDs.** Every node has an ID that stays the same across recompiles unless the node is deleted. The author may set it; otherwise the validator derives it deterministically from the node's path and kind. Three things need stable IDs: AccessKit, which requires them [§9.6]; diffs ("prose → IR diff", A§2.10); and preserving state across hot reload while authoring.

**Canonical ordering.** Canonical form has one textual serialisation: fields in a fixed order, numbers normalised, strings in Unicode NFC. The same meaning therefore always produces the same bytes. That is what makes signing, hashing and golden tests meaningful.

## 4. Node kinds (core v0)

The core is deliberately small. *Kind* says how a node lays out and paints. *Role* (§8) says what it means. A button is a `box` with role `button` and a `press` handler, not a separate kind. Richer vocabulary such as "card", "avatar" or "data grid" lives in the **standard component library** (`std`, §4.1). Each component expands deterministically into core kinds.

| Kind | Purpose | Children | Notes per tier |
|---|---|---|---|
| `surface` | Root of a window or screen; owns the viewport variables | nodes | One per platform surface. Hosts on iOS/Android/web are thin shells (A§1.7) |
| `box` | Container and paint carrier (fill, border, shadow, clip) | nodes | All paint is in the SDF algebra (§6) |
| `text` | Read-only text run(s) from the strings table, with style spans | — | Shaping/rasterisation per G6 |
| `text-input` | Editable text: single-line, multi-line, password, numeric, email, search | — | IME and selection per G6 §6. On the web, a hidden native input is overlaid [G7 S.3] |
| `image` | Raster image from a bundled asset (content-hashed) | — | First-class on all tiers (A§1.9 correction) |
| `shape` | Free SDF geometry (§6) that is not a container: icons, dividers, decorative marks | — | Paths beyond the analytic primitives use the raster backend (§6.1) |
| `scroll` | Clipped viewport over content larger than itself; one or two axes | one | Retained-mode damage tracking; virtualises `view` children |
| `view` | Data-bound repetition: one template instantiated per element of a state list, with declarative sort/filter | template | Virtualised when inside `scroll`. No unbounded computation per item (§7.3) |
| `slot` | Shows exactly one of several child variants, chosen by a state expression | variants | Screens, tabs, empty/loading/error states, and pre-compiled variants for the optional Tier 2 assistant (A§4.6) |
| `layer` | Content above the normal flow: dialog, popover, toast, menu | nodes | Owns focus trapping and dismissal semantics |
| `ext:<ns>/<kind>` | Extension node kind (§13) | per extension | Must declare a Tier 0 behaviour |

### 4.1 Standard component library (`std`)

`std` is versioned with the IR spec and changes only through the RFC process (Q5). It is **not** a dialect pack. Dialect packs map *human phrasing* to IR (interpreter side, Q4). `std` defines *what a component expands to* (language side). Keeping them separate means a community dialect pack can change how "a soft border" is phrased in Tagalog without changing what any shipped app draws.

v0 `std` must cover the six G2 categories at minimum: card, avatar, button (primary/secondary/destructive/link), icon button, checkbox, switch, radio group, slider, select/combo box, text field with label/help/error, form, tabs, accordion/disclosure, dialog, toast, menu, list, data table (sortable header, sticky header, virtualised body, row selection), pagination, progress (determinate/indeterminate), badge, divider, navigation bar, image with caption, empty state, and skeleton/loading.

Every `std` component specifies its expansion, its required semantics, its keyboard interaction, and its theme-token dependencies. The validator rejects a `std` component used without the fields it requires, such as a checkbox without a label (`PVC-E-A11Y-NAME`).

## 5. Layout: constraint vocabulary

### 5.1 Variables, units, priorities

- **Variables per node:** `left`, `top`, `width`, `height`. Derived: `right`, `bottom`, `center-x`, `center-y`, and `first-baseline`/`last-baseline` for text-bearing nodes. Parent *content box* variables are the parent's box minus its padding.
- **Units:** `px` means logical (device-independent) pixels. Also `%` of the parent content box, `em` relative to the node's resolved font size, `fr` grid fractions, and theme tokens (`space.md`, `radius.lg`, ...). Device-pixel snapping happens in the runtime, after the solve. The solve snaps to 1/64 logical px, which is the determinism rule in §14.
- **Relations:** `=`, `≤`, `≥` between linear expressions of variables and constants. This is the Cassowary class the analysis names (A§4.2). *Solver choice: general knowledge; not a sourced fact in this pass.*
- **Priorities:** `required` > `strong` > `medium` > `weak`. A `required` constraint is never violated. Lower priorities are satisfied as far as possible in priority order.
- **Intrinsic sizes:** `text`, `image` and `ext` nodes contribute *intrinsic* min/preferred/max sizes as `weak`/`medium` constraints. Text intrinsic height depends on width, which is not linear. §5.4 explains how that is handled.

### 5.2 Layout sugar

`row`, `column`, `stack` (z-overlay), `grid` (tracks in `px`/`fr`/`auto`), `wrap` (flow with line breaking), `align`, `justify`, `gap`, `pad` and `distribute` are *sugar*. The validator expands each into constraints in canonical form. Authors and agents almost always use the sugar, which is where terse-form savings come from. Raw constraints exist for the cases sugar cannot express.

### 5.3 Vocabulary: intent phrase families → constraints

This is the core of the "small, finite vocabulary" in A§4.2. The *phrase family* column is illustrative. Actual phrasings per language live in dialect packs. The *IR meaning* column is normative.

| Phrase family (English illustrations) | IR meaning | Default priority | Default when under-specified | Diagnostic if ambiguous |
|---|---|---|---|---|
| "N px wide/tall" | `width`/`height = N` | required | — | `PVC-E-CONSTRAINT-CONFLICT` if it contradicts another required constraint |
| "full-width", "stretch" | `left = parent.content.left`, `right = parent.content.right` | required | — | — |
| "at least / at most N" | `≥` / `≤` | required | — | — |
| "about N", "roughly N" | `= N` | medium | — | `PVC-I-SOFT-VALUE` (info) |
| "next to", "beside" | sibling in a `row`, gap = `space.sm` | strong | After the referenced sibling, in reading direction (so RTL flips it, P8) | `PVC-W-AMBIG-ANCHOR` if the referent is unclear |
| "below", "under", "above" | sibling in a `column`, ordered | strong | — | `PVC-W-AMBIG-ANCHOR` if the referent is unclear |
| "at the bottom/top" | **ambiguous**: end of parent flow, *or* pinned to the parent edge, *or* pinned to the viewport | strong | End of the parent's flow (not pinned) | `PVC-W-AMBIG-ANCHOR` with three machine-applicable fixes |
| "centered" | `center-x = parent.center-x` (and `center-y` if "in the middle") | strong | Horizontal only | `PVC-W-AMBIG-AXIS` |
| "fill the rest", "take remaining space" | grow factor 1 in the parent row/column | medium | — | — |
| "N columns" | `grid` with N `1fr` tracks | required | gap = `space.md` | `PVC-W-AMBIG-RESPONSIVE` if no narrow-viewport behaviour is given |
| "sticky", "pinned while scrolling" | constraint against the `scroll` viewport, not the content | required | — | `PVC-E-STICKY-NO-SCROLL` if there is no scroll ancestor |
| "soft/comfortable/tight spacing" | spacing token (`space.lg`/`md`/`sm`) | — | Dialect-pack mapping, pinned | `PVC-I-CULTURAL-DEFAULT` (info: the value comes from a locale-specific pack) |
| "responsive", "on small screens…" | constraint sets keyed by viewport-width ranges (breakpoints are theme tokens) | — | `std` breakpoints | `PVC-W-AMBIG-RESPONSIVE` |
| "square", "16:9" | `width = k × height` | required | — | — |

### 5.4 Satisfiability: what the validator can and cannot prove

- The manifest declares a **supported viewport range** (minimum and maximum width and height) and a **text-scale range** (for accessibility font scaling).
- For the linear part of the system, the set of viewport sizes at which all `required` constraints can be met is convex: it is a linear projection of a convex polyhedron. The validator therefore **proves** satisfiability over the whole declared range by checking the corners of the range. *This is an analytical claim by the author, to be checked by the red-team and the build session.*
- Text breaks linearity, because wrapped height is a step function of width. The validator therefore treats each text node's height as bounded by its min/max over the declared width range, using shaped text measured at the range endpoints and at each breakpoint. It proves satisfiability against those bounds. **Residual risk:** a pathological string could wrap differently at an intermediate width and overflow. The runtime handles this deterministically: `required` text-height constraints are never authored by `std`; overflow clips with an ellipsis and raises a runtime diagnostic in the developer preview, never in end-user UI.
- Required-vs-required conflicts found anywhere in the declared range are **errors**. That is a validator rule, so prose cannot create an unrenderable app.

## 6. Paint: the SDF algebra

### 6.1 Primitives

| Primitive | Parameters | Exact distance? | Notes |
|---|---|---|---|
| `rect` | size | yes | |
| `rounded-rect` | size, four corner radii | yes | The workhorse; covers about all `box` paint (*estimate*) |
| `circle` / `ellipse` | radius / radii | circle yes; ellipse approximate | Ellipse distance is an approximation. The AA formula (§6.4) tolerates this within tier tolerance |
| `capsule` / `segment` | endpoints, radius | yes | Dividers, sliders, progress bars |
| `ring` | radius, thickness | yes | Spinners, radio buttons |
| `polygon` (convex, ≤16 vertices) | vertices | yes | Chevrons, arrows, simple icons |
| `path` | Bézier path (content-hashed asset or inline) | **no analytic SDF in v0** | Rendered through the raster backend (Vello-class [§9.4]) on all tiers. Analytic path SDFs are a Tier 2 research item, not v0 |
| `glyph-run` | shaped glyph IDs + positions (from G6 text layout) | n/a | Rasterised per G6. Never evaluated as analytic SDF below the display-type threshold (G6 §4) |
| `image` | texture asset, fit mode, sampling | n/a | |

### 6.2 Operations (closed algebra)

- **Combination:** `union` (min), `intersect` (max), `subtract`, `smooth-union(k)` (k in logical px, bounded ≤ 32 *decision*).
- **Modification:** `offset(r)` (grow/shrink, rounds corners), `annulus(w)` (stroke of width w centred on the edge), `inset-stroke`, `outset-stroke`.
- **Transform:** translate, rotate, **uniform** scale. Non-uniform scale distorts distances, so it is a validator error (`PVC-E-NONUNIFORM-SCALE`). The fix is to change the primitive's parameters instead.
- **Clip:** clip a subtree to an SDF (rounded-rect clip is the common case).

### 6.3 Shading

Fill with a solid colour, linear gradient or radial gradient (≤8 stops in v0); stroke; **analytic drop shadow** for rect, rounded-rect and circle (closed form, cheap on every tier); opacity; blend mode `normal` on all tiers (`multiply`/`screen` Tier 2 only, otherwise degraded per §12); backdrop blur (Tier 2 only, otherwise degraded).
Colours are authored as theme tokens or sRGB values. **Compositing space (decision, flagged for prototype verification):** v0 composites in sRGB-encoded space, because the quality bar is "matches Chromium" (A§4.3, G6) and matching the reference's blending matters more than physical correctness. *Unverified in this pass: the exact blending behaviour of the Chromium reference. The text-parity suite (G6) will expose a mismatch.*

### 6.4 One anti-aliasing formula on every tier

Coverage is a single specified function of the signed distance at the pixel centre and the pixel footprint in distance units. It is a clamped linear ramp one pixel wide, centred on the edge, and the footprint comes from the local transform. Tier 0 (CPU), Tier 1 (fragment) and Tier 2 compute the same function. **They do not produce bit-identical pixels.** GPU fragment precision on GL ES 3.0 (medium-precision floats are permitted on that class of hardware, *general knowledge, unverified in this pass*) and CPU SIMD arithmetic differ in the last bits. The guarantee is therefore **parity within a documented tolerance** (§14 level D4). See §17 for the correction this implies to analysis A§4.2.

## 7. Behaviour model

### 7.1 State
A typed record declared in the module. Types: `bool`, `int` (64-bit), `decimal` (fixed-point, for money and counts shown to users), `text` (a strings-table key *or* user-entered text), `enum`, `optional<T>`, `list<record>` with a declared maximum length, and `record`. **No floats in state.** Geometry uses floats inside the runtime; state does not, so state transitions are bit-reproducible on every architecture.

### 7.2 Events
`press`, `long-press`, `double-press`, `hover-enter`/`hover-leave` (pointer devices only, never the only path to an action: `PVC-E-HOVER-ONLY`), `focus`/`blur`, `change` (inputs), `submit`, `key(<named key>)`, `back` (platform back gesture or button), `scroll-end-reached`, `appear`/`disappear`, `timer(<id>)` (declared, bounded ≥ 100 ms *decision*), `effect-ok(<effect>)` / `effect-error(<effect>)`, and `a11y-action(<AccessKit action>)` (§8). Every pointer event has a keyboard and assistive-technology equivalent. The validator enforces this for `std` components and requires it for custom handlers.

### 7.3 Guards and updates
- **Guards** are pure boolean expressions over state and event payload: comparisons, `and`/`or`/`not`, `is-empty`, `length`, `contains`, enum tests.
- **Updates** are a sequence of assignments of pure expressions to state fields, plus bounded list operations: `append`, `remove-where`, `update-where`, `sort-by`, `clear`.
- **Totality:** no user-defined functions, no recursion, no loops other than the implicit, bounded iteration of list operations over declared maximum lengths. Every handler terminates in time bounded by the declared sizes. That is what keeps the expression language small and safe for a weak model to emit correctly.
- **Transitions:** a handler may start declarative animations (tween a paint or constraint value over a duration with a named easing). Animations respect the platform's *reduce motion* setting, which turns them into instant jumps. They never change logical state, only presentation.

### 7.4 Effects
An effect is a **named, asynchronous, capability-scoped** request: HTTP request, read/write app storage, pick a file, copy to clipboard, open an external URL, share, notify, camera/mic capture, read location. Each effect declaration names exactly one capability from the manifest (§10). Results come back only as `effect-ok`/`effect-error` events with typed payloads, so all asynchrony goes through the same deterministic event queue. `'connect_user'` in the README is exactly this: an effect bound to the button's `press` event.

**Not in v0:** arbitrary code, plugin scripts, author-supplied shaders. The README's WGSL emphasis concerns the *runtime's* implementation, not authors. Author shaders would break tier parity (P8) and the capability sandbox (P6). If wanted later, they arrive as an `ext` with its own RFC.

## 8. Semantics and the accessibility mapping

Every node has: `role` (required unless the node is marked `decorative`, which hides it from assistive technology); `name` (strings key, or derived from content for `text`); optional `description`; state flags `disabled`, `checked` (true/false/mixed), `selected`, `expanded`, `pressed`, `required`, `invalid` (with an error-message relation), `busy`; `value`/`min`/`max`/`step` for range controls; `heading-level`; `live` (`off`/`polite`/`assertive`); relations `labelled-by`, `described-by`, `controls`, `error-message`; and a `focus-order` override (rarely needed, because default order is reading order and follows direction).

### 8.1 Mapping table (PVC role → AccessKit 0.25.1 `Role` [G7 S.7] → WAI-ARIA role for the web mirror)

ARIA role names are from WAI-ARIA as general knowledge (*not re-fetched in this pass*).

| PVC role | AccessKit `Role` | ARIA (web mirror) | Required fields beyond name |
|---|---|---|---|
| `button` | `Button` | `button` | — |
| `link` | `Link` | `link` | target (an effect `open-url` or a `slot` change) |
| `checkbox` | `CheckBox` | `checkbox` | `checked` |
| `switch` | `Switch` | `switch` | `checked` |
| `radio-group` / `radio` | `RadioGroup` / `RadioButton` | `radiogroup` / `radio` | `checked` on radio |
| `slider` | `Slider` | `slider` | value, min, max, step |
| `stepper` | `SpinButton` | `spinbutton` | value, min, max |
| `text-field` | `TextInput` | `textbox` | — |
| `text-area` | `MultilineTextInput` | `textbox` (multiline) | — |
| `password-field` | `PasswordInput` | `textbox` (masked input) | — |
| `search-field` | `SearchInput` | `searchbox` | — |
| `email-field` / `number-field` | `EmailInput` / `NumberInput` | `textbox` / `spinbutton` | — |
| `combo-box` | `ComboBox` | `combobox` | `expanded`, `controls` |
| `list-box` / `option` | `ListBox` / `ListBoxOption` | `listbox` / `option` | `selected` |
| `list` / `list-item` | `List` / `ListItem` | `list` / `listitem` | — |
| `table` / `row` / `cell` / `column-header` | `Table` / `Row` / `Cell` / `ColumnHeader` | `table` / `row` / `cell` / `columnheader` | sort state on headers |
| `grid` / `grid-cell` | `Grid` / `GridCell` | `grid` / `gridcell` | for interactive data grids |
| `tab-list` / `tab` / `tab-panel` | `TabList` / `Tab` / `TabPanel` | `tablist` / `tab` / `tabpanel` | `selected`, `controls` |
| `dialog` / `alert-dialog` | `Dialog` / `AlertDialog` | `dialog` / `alertdialog` | focus trap; dismiss action |
| `alert` / `status` | `Alert` / `Status` | `alert` / `status` | `live` |
| `heading` | `Heading` | `heading` | `heading-level` |
| `paragraph` / `label` | `Paragraph` / `Label` | `paragraph` / (native label) | — |
| `image` | `Image` | `img` | name, or `decorative` |
| `progress` | `ProgressIndicator` | `progressbar` | value or indeterminate |
| `meter` | `Meter` | `meter` | value, min, max |
| `menu` / `menu-item` | `Menu` / `MenuItem` | `menu` / `menuitem` | — |
| `toolbar` | `Toolbar` | `toolbar` | — |
| `tooltip` | `Tooltip` | `tooltip` | not the only carrier of essential information |
| `navigation` / `main` / `region` / `form` | `Navigation` / `Main` / `Region` / `Form` | `navigation` / `main` / `region` / `form` | `region` needs a name |
| `group` / `container` | `Group` / `GenericContainer` | `group` / (none) | — |
| `scroll-area` | `ScrollView` | (native scrolling element) | — |
| `tree` / `tree-item` | `Tree` / `TreeItem` | `tree` / `treeitem` | `expanded` |
| `canvas-graphic` (charts, `ext`) | `Canvas` or `Image` | `img` + description | text alternative **required** |

**Actions.** AccessKit actions [G7 S.7] map onto IR events: `Click` → `press`; `Focus`/`Blur` → `focus`/`blur`; `Expand`/`Collapse` → the component's expand handler; `Increment`/`Decrement`/`SetValue` → `change` on range controls; `ReplaceSelectedText`/`SetTextSelection` → text-input editing (G6); `ScrollIntoView`/`SetScrollOffset` → `scroll`. An interactive role whose required action has no handler is `PVC-E-A11Y-ACTION`.

### 8.2 Validator-enforced accessibility rules (release-blocking, A§4.7.4)

Accessible name present (`PVC-E-A11Y-NAME`); images named or marked decorative (`PVC-E-A11Y-IMG`); text contrast ≥ 4.5:1 and large-text/UI-component contrast ≥ 3:1 against the resolved background in every theme variant (`PVC-E-A11Y-CONTRAST`; thresholds from WCAG 2.2 AA, which EN 301 549 v4.1.1 incorporates [§9.8]; the ratio values are *general knowledge, not re-fetched*); pointer targets ≥ 24×24 logical px (`PVC-W-A11Y-TARGET`, WCAG 2.2 *general knowledge*); no hover-only actions; focus order follows reading order unless overridden; `live` set on status content that changes without user action (`PVC-W-A11Y-LIVE`); text scaling up to 200% keeps the layout satisfiable (§5.4, `PVC-E-A11Y-REFLOW`).

### 8.3 Web semantics mirror (new requirement from G7 S.1)

AccessKit has **no shipped web adapter** [G7 S.1]. On web targets the runtime must therefore generate a hidden, positioned DOM tree with ARIA roles and states from the same IR semantics, following the pattern Flutter's web engine uses [G7 S.3]. Unlike Flutter, the mirror is **on by default**. Its update cost is part of the Tier 1 web frame budget (16.6 ms, A§4.4.1), and the budget is measured with the mirror on. This is a requirement on the runtime, not on authors: the IR already holds everything the mirror needs.

## 9. Diagnostics

### 9.1 Shape of a diagnostic
Each diagnostic has: **code** (stable, never reused); **severity** (`error` blocks acceptance; `warning` is accepted in draft mode but blocks release mode unless acknowledged; `info` never blocks); **location** (node ID and IR path, plus the intent-source span when provenance is present); **message key and typed parameters**, rendered by localisable templates (A§4.7.3), so no diagnostic is ever English-only prose; and zero or more **fixes**, each a structured IR edit the agent can apply without free-form reasoning. Fixes are what make "a weak model plus a strict validator" (A§4.7.1) work in practice.

### 9.2 Modes
- **Draft:** errors block; warnings are reported and their defaults applied.
- **Release:** errors block; any **unacknowledged** warning blocks. An acknowledgement is an explicit IR entry: "I accept default X for diagnostic Y at node Z". The agent may propose it, but release-mode acceptance needs a human confirmation step, recorded in the publish step, for capability-related acknowledgements (§9.4, G5).
- **Publish** = produce the signed canonical artifact. It requires release-mode acceptance, a signed dialect pack (Q5: IR compiled against an unsigned pack is "local-only" and cannot be published), and provenance stripped.

### 9.3 v0 code catalogue

| Code | Sev. | Meaning | Typical machine fix |
|---|---|---|---|
| `PVC-E-PARSE` | E | Terse/canonical text not well-formed | Location + expected token class |
| `PVC-E-SCHEMA` | E | Unknown field, wrong type, missing required field | Field list for the kind |
| `PVC-E-UNKNOWN-KIND` | E | Node kind not in core, `std` or a declared `ext` | Nearest `std` component(s) |
| `PVC-E-UNKNOWN-TOKEN` | E | Theme token not defined | Nearest token |
| `PVC-E-REF` | E | Reference to an undefined node, state field, string key or effect | Candidates |
| `PVC-E-ID-COLLISION` | E | Two nodes share an ID | Renamed ID |
| `PVC-E-CONSTRAINT-CONFLICT` | E | Required constraints unsatisfiable somewhere in the declared viewport/text-scale range | Demote one constraint; show the viewport where it fails |
| `PVC-E-STICKY-NO-SCROLL` | E | Sticky/pinned without a scroll ancestor | Wrap in `scroll` |
| `PVC-E-NONUNIFORM-SCALE` | E | Non-uniform scale on an SDF | Change primitive parameters |
| `PVC-E-TYPE` | E | Expression type error in a guard or update | Expected type |
| `PVC-E-UNBOUNDED` | E | List without declared maximum, or timer < 100 ms | Add bound |
| `PVC-E-HOVER-ONLY` | E | Action reachable only by hover | Add press/focus path |
| `PVC-E-CAP-UNDECLARED` | E | Effect uses a capability not in the manifest | Declare it (needs human confirmation on publish, §9.4) |
| `PVC-E-CAP-SCOPE` | E | Effect exceeds the declared scope (host, path, method) | Narrow the effect or widen the declaration (human-confirmed) |
| `PVC-E-CAP-IN-STRING` | E | A capability, effect or handler reference appears where only display data is allowed, e.g. a string asking to "open the camera" bound to an effect slot | Remove it; strings are data (P1) |
| `PVC-E-A11Y-NAME` | E | Interactive or meaningful node without an accessible name | Suggested name from visible text |
| `PVC-E-A11Y-IMG` | E | Image neither named nor decorative | Mark decorative or add name |
| `PVC-E-A11Y-CONTRAST` | E | Contrast below threshold in some theme variant | Nearest passing token |
| `PVC-E-A11Y-ACTION` | E | Interactive role lacks a handler for its required action | Stub handler |
| `PVC-E-A11Y-REFLOW` | E | Layout unsatisfiable at 200% text scale | Allow wrap / grow |
| `PVC-E-TIER-NO-FLOOR` | E | Feature has no Tier 0 rendering and no declared degradation | Add degradation (§12) |
| `PVC-E-VERSION` | E | Header pins an IR, `std`, dialect or runtime version this validator does not support | Supported versions |
| `PVC-E-EXT-UNDECLARED` | E | `ext` kind used without its schema pinned in the header | Add pin |
| `PVC-E-PACK-UNSIGNED` | E (publish only) | Dialect pack unsigned or signature invalid | Use a signed pack, or stay local-only |
| `PVC-E-STRING-MISSING` | E | String key missing for a declared locale | Placeholder key (blocks release) |
| `PVC-W-AMBIG-ANCHOR` | W | "Bottom/top/next to" with more than one plausible referent | One fix per reading |
| `PVC-W-AMBIG-AXIS` | W | "Centered" without an axis | Horizontal / both |
| `PVC-W-AMBIG-RESPONSIVE` | W | Multi-column layout with no narrow-viewport behaviour | Stack below breakpoint `bp.sm` |
| `PVC-W-AMBIG-REFERENT` | W | Intent refers to an element that does not exist ("the other button") | Candidates |
| `PVC-W-CONTRADICTION` | W | Intent contained contradictory statements and one was preferred ("full width, 200px wide") | Both alternatives |
| `PVC-W-UNDERSPECIFIED` | W | Intent too thin to determine structure ("make a dashboard"); the agent used a `std` template | Questions to ask the human, as structured choices |
| `PVC-W-CAP-CHANGED` | W | Capability manifest differs from the previously published version | Requires human confirmation; never auto-acknowledged |
| `PVC-W-INSTRUCTION-IN-DATA` | W | A display string reads like an instruction to an AI ("ignore previous…"). It is harmless at runtime (P1) but flagged because it may be prompt-injection residue | Show the string; confirm or remove |
| `PVC-W-SECRET-PLAIN` | W | Credential-typed data written to non-`secret` storage | Switch to `storage.secret` |
| `PVC-W-A11Y-TARGET` | W | Pointer target < 24×24 px | Grow padding |
| `PVC-W-A11Y-LIVE` | W | Content changes without user action but is not `live` | Set `polite` |
| `PVC-W-TIER-DEGRADED` | W | Effect will be degraded on Tier 0/1 per §12 | Acknowledge |
| `PVC-W-SYSTEM-FONT` | W | Module opts into system fonts, so cross-device text output is not guaranteed (§14) | Bundle the font set |
| `PVC-I-SOFT-VALUE` | I | "About N" compiled as a medium-priority constraint | — |
| `PVC-I-CULTURAL-DEFAULT` | I | Value came from a locale-specific dialect mapping | — |
| `PVC-I-DEFAULT-APPLIED` | I | A `std` default was used where the intent was silent | — |

### 9.4 Diagnostics that a human, not the agent, must clear
`PVC-W-CAP-CHANGED`, `PVC-E-CAP-UNDECLARED` and `PVC-E-CAP-SCOPE` at publish time, and `PVC-W-INSTRUCTION-IN-DATA`. These are the paths by which prompt-injected content could widen what an app may do. The validator only reports them. The **publish** step requires a human confirmation that the agent cannot synthesise, for example an interactive confirmation in the publishing tool. G5 §4 treats this as trust boundary TB7.

## 10. Capability manifest

| Capability | Scope parameters | Granted when | Notes |
|---|---|---|---|
| `net` | hosts (exact or suffix), methods, max request size, max daily data | Install / first use | No raw sockets in v0 |
| `storage` | app-private key-value, quota; a separate `secret` class for credentials, backed by the platform keystore where one exists (*per-platform availability unverified in this pass*) | Implicit at install, shown to the user | Never shared across apps. Writing a `password-field` value or a field typed as a credential to non-`secret` storage raises `PVC-W-SECRET-PLAIN` |
| `file.pick` | read / write, MIME filters | Each use, via the system picker | No ambient filesystem paths in v0 |
| `clipboard.write` | — | Implicit, shown | |
| `clipboard.read` | — | Each use, only during a user gesture | |
| `camera`, `mic` | — | First use, OS prompt | Indicator required while active |
| `location` | coarse / precise | First use, OS prompt | |
| `notify` | — | First use | |
| `open-url` | allowed URL patterns | Implicit, shown | Opens the system browser; never an embedded web view |
| `share` | — | Each use, system sheet | |

**Resource boundaries** (also manifest-declared and user-narrowable): maximum memory (default inside the 150 MB Tier 0/1 envelope, A§4.4.1 rule 4), frame-rate cap, maximum tier, network data cap.

**Enforcement rules:** (1) the runtime checks every effect against the granted set at the single effect dispatcher; there is no other path to the platform (A§4.6). (2) The user's **boundary file** can only *narrow* grants and is never writable by apps. (3) Dialect packs and `std` cannot add capabilities. (4) A tampered manifest fails the signature check, and the runtime refuses to start the app. (5) The optional Tier 2 on-device assistant (A§4.6) can switch `slot` variants and adjust *non-required* constraint priorities within authored bounds. It cannot emit effects, edit the manifest or change state outside declared assistant hooks.

## 11. Versioning and pinning

- **IR spec:** semver. **Minor** versions are additive only: new optional fields, new `std` components, new diagnostics. **Major** versions may change meaning and require the RFC process (Q4, Q5).
- **Runtime compatibility:** a runtime that supports IR `N.m` executes any `N.k` with k ≤ m. It refuses `N.k` with k > m unless every feature it does not know is marked with a declared fallback, and refuses any other major with a clear message.
- **What a module pins:** IR version, `std` version, dialect-pack ID + version + signature digest, `ext` schemas + versions, font set content hashes, and the theme. Re-pinning anything is an explicit act and yields a new artifact. **A shipped app never changes without recompilation** (Q4).
- **Two kinds of signature:** the *dialect pack* is signed by its governing body (maintainers in Phase 1, the steering group in Phase 2; per-org forks are signed by the org) (Q5). The *published app artifact* is signed by its publisher, and the runtime verifies that signature. Unsigned artifacts run only in developer preview mode, with a visible banner.
- **Runtime rendering changes are versioned too.** A dependency upgrade that changes pixels (G7 §2, pinning policy item 3) bumps the runtime minor version, and golden images are re-baselined in the open.

## 12. Tier degradation (what Tier 0/1 may and may not degrade)

| Feature | Tier 2 | Tier 1 | Tier 0 |
|---|---|---|---|
| Layout, text content, semantics, hit-testing, behaviour, capability enforcement, contrast | full | **identical** | **identical** |
| Solid/gradient fill, stroke, rounded rect, analytic shadow, opacity, clip | full | full | full |
| `smooth-union` | full | full | full (CPU cost bounded by the k ≤ 32 cap) |
| Backdrop blur | full | solid translucent scrim of the same average luminance | same as Tier 1 |
| Non-`normal` blend modes | full | `normal` | `normal` |
| HDR / wide gamut | where supported | SDR | SDR |
| Large display text | analytic/MSDF permitted (G6) | atlas | atlas |
| Animations | display rate | 60 FPS target | 30 FPS floor; may skip frames, never skip end state |
| Compute-tiled heavy vector scenes | permitted | not available (raster backend) | CPU raster backend |
| `ext` kinds | per extension | per extension's declared fallback | per extension's declared Tier 0 fallback (mandatory) |

A degradation that would change *meaning* is forbidden. A blur conveying "disabled", for example, must also set `disabled` and contrast. The validator emits `PVC-W-TIER-DEGRADED` so the author sees each degradation once.

## 13. Extension mechanism (Q7: "UI first, not UI only")

An **extension** is a namespaced package `ext:<namespace>/<kind>@<version>`, adopted through the RFC process. Experimental extensions may be used in draft mode only. It must declare:

1. **Schema:** its fields and types, which the validator loads (pinned in the header).
2. **Layout contract:** how it participates in constraints, as a box with intrinsic sizes. It cannot reach into its parent's layout.
3. **Paint contract:** what it draws, per tier, with a **mandatory Tier 0 behaviour**. For example, a 3D viewport on Tier 0 might show a static poster frame plus a text alternative.
4. **Semantics contract:** its role mapping (§8) and text alternative. No extension is exempt from §8.2.
5. **Capability needs:** declared capabilities only. An extension cannot hold ambient authority.
6. **Determinism class:** *D-exact* (as core), *D-tolerance* (pixels within tolerance), or *D-seeded* (stochastic, with the seed in state, e.g. particle effects). Physics extensions must use a fixed timestep to qualify as D-seeded. Non-deterministic extensions are not admissible.
7. **Behaviour surface:** the events it emits and the state it reads. It cannot write state except through declared events.

**Honest limit.** This mechanism suits *visual* product classes: data visualisation, games/3D viewports, embedded displays. Non-visual product classes such as backend services, CLIs and pipelines need things v0 deliberately forbids: long-running computation, unbounded loops, ambient I/O. Supporting them would almost certainly need a new IR **major** with a different behaviour model, not an `ext`. That is consistent with Q7, which keeps them out of release one. But the owner's "any digital product" is served by *this* mechanism only for visual products, and the README must not imply more (G1).

## 14. Determinism guarantees

| Level | Guarantee | Conditions |
|---|---|---|
| **D1 Validation** | Same input text + same validator version + same pinned `std`/dialect/`ext` versions → byte-identical canonical IR and an identical diagnostic list | No network, no clock, no randomness in the validator |
| **D2 Behaviour** | Same canonical IR + same event sequence + same effect results → identical state sequence, on every platform | No floats in state (§7.1); effect results are inputs, recorded for replay in tests |
| **D3 Layout** | Same IR + same viewport + same text-scale and locale settings + **same font bytes** → identical solved geometry (snapped to 1/64 px) on every tier and platform | Layout runs on the CPU on all tiers with the same algorithm. Shaping uses the pinned shaper version (G7) |
| **D4 Pixels** | Rendered output within a documented per-tier tolerance of the Tier 2 reference (A§4.4.1 rule 1). **Not bit-identical** | Tolerance measured with the G6 metric |

**Named sources of variation, and how each is handled:**
- **System fonts.** They break D3 across devices, so modules bundle a content-hashed font set by default. Opting into system fonts is allowed (`PVC-W-SYSTEM-FONT`). It is useful for very small apps on devices with good system fonts, but cross-device text output is then not guaranteed. G6 §3 covers bundle size.
- **Locale formatting** (dates, numbers): deterministic given the declared locale data version, pinned with the runtime.
- **Time:** only through `timer` events and a `now` effect, both recorded in test traces.
- **Randomness:** only through a seeded effect.
- **User settings** (text scale, reduce motion, high contrast, forced tier, frame cap): inputs to D3/D4, not violations. They change output on purpose.

## 15. Illustrative example (non-normative syntax)

The README's profile card in **terse form**, written by hand by the author of this document as one data point (§2.1). It is not evidence of anything and is measured only to make the token argument concrete:

```text
pvc 0.1 use std@0.1 theme dark
card w=320 border=soft pad=lg gap=md {
  row gap=sm align=center {
    avatar "JD" size=40 fill=blue
    col { text "Jane Doe" bold size=14; text "Product Designer" muted size=12 }
  }
  text "Building the future of spatial computing via GPU pipelines." body size=14
  button "Connect" w=fill fill=teal on.press=connect_user
}
effect connect_user uses net.main
```

118 tokens (`o200k_base`), against 185 for the README's HTML [G7 S.4]. Several things are absent from the HTML and are made explicit here, or by the validator, at no authoring cost: the button's role and accessible name (from its label), the avatar's text alternative (`std` uses the person's name when the avatar sits next to it, and otherwise raises `PVC-E-A11Y-NAME`), contrast checks against the dark theme, and the capability the button's effect needs. In **canonical form** the validator moves the inline strings to the strings table, so a Spanish-language version of the same app differs *only* in that table. A Spanish-speaking author's prose request is also accepted (G1 shows it), and it compiles to the same structure. One thing is missing from this snippet: the manifest entry declaring `net.main`. The validator would report `PVC-E-CAP-UNDECLARED`. It is left out deliberately to show that the snippet is not a valid program by itself.

## 16. Residual risks and unknowns (hostile self-assessment)

1. **The token thesis may fail on fixed overhead (§2.1 item 4).** If the schema + `std` + dialect-pack context an agent needs is, say, 8–15k tokens (*estimate*, unmeasured), short sessions may cost *more* than writing HTML that the model already knows. Mitigations are design hypotheses, not facts: a compact "agent card" summary of the schema; validator fixes that teach the agent in context; provider prompt caching where available (*unverified*: availability and pricing vary by vendor). G2 measures this with the overhead included.
2. **Model familiarity.** Today's models have seen billions of lines of HTML and zero lines of PVC-IR. First-attempt accuracy may be poor until dialect packs and examples mature. This is the main risk to "any agent" (A§4.7.1) and the reason for the weak-model arm in G2.
3. **`std` breadth is the real scope.** "Small core" is honest only because `std` absorbs the complexity. A thin `std` pushes agents into raw constraints, which costs tokens and errors. A fat `std` is a UI toolkit by another name, which PVC claims to bypass. v0's list (§4.1) is the minimum for the G2 corpus, not a promise of breadth.
4. **Satisfiability proof (§5.4) rests on an analytical argument** that has not been externally checked, and on text-height bounds that can be wrong for pathological strings.
5. **Web semantics mirror cost (§8.3)** is unbudgeted in engineering terms, and its per-frame cost on Tier 1 web is unknown. Flutter reports repositioning can be expensive [G7 S.3].
6. **Compositing space (§6.3)** is a decision made without measuring the reference.
7. **Diagnostics catalogue completeness.** 41 codes cover the G2 adversarial corpus by design. Real use will find gaps. Codes are append-only, so gaps cost a minor version, not a break.
8. **Human confirmation at publish (§9.4)** is the anti-injection backstop. It assumes the publishing human reads what they confirm, and confirmation fatigue is a known failure mode (*general knowledge*).
9. **Non-visual products** are outside what the extension mechanism can carry (§13).

## 17. Findings for the main analysis (inconsistencies this spec exposed)

- **A§4.2 item 2 says CPU and GPU "produce bit-comparable output".** That overclaims, and it conflicts with A§4.4.1 rule 1 ("parity within a documented tolerance"). The correct guarantee is D3 (identical geometry) plus D4 (pixels within tolerance), per §14.
- **A§2.5/A§9.6 say AccessKit ships a web adapter.** It does not [G7 S.1]. The web target needs the §8.3 mirror.
- **The README/A§2.6 token story compares human prose with agent-written HTML.** Under the hard wall the agent writes IR in both directions (§2.1). The token thesis therefore depends on terse-IR compactness and context overhead, and A§2.6 should say so.
- **A§1.10 "Add LICENSE"** is fine. But the README must also stop implying that the *intent prose* is the shipped program: per A§4.1 the prose is source and the IR is the program (G1 handles this).
