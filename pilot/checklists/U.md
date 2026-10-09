# U1–U5 — universal checklist (stack-neutral)

Source of truth for the checklist items: `docs/G2_Benchmark_Corpus.md` §5 (main @ 5fdd412). Item order and wording below follow the G2 row. **A** = automatable (decided by the checker from observables), **H** = human-judged: *not judged (D-5)*; pass is decided on A items only.

Applies to every specimen in both stacks (G2 §3.1). Widths tested: 320, 480, 768, 1024, 1280 px (height 800) unless stated; text scale 1 unless stated.

## U1 — Every interactive element has an accessible name exposed to the platform accessibility API.  `[A]`

**Operational definition**

- Snapshots at widths 320, 480, 768, 1024, 1280 (height 800, text scale 1).
- For every visible, enabled node whose `role` is interactive (button, link, checkbox, switch, radio, slider, stepper, any text input role, combo-box, list-box, option, tab, menu-item, tree-item): `name` is non-empty after trimming.
- Additionally, every visible node with `focusable` = true and `role` = null (a focusable element that exposes no role) fails, except the document root.
- Accessibility tree source: baseline = Chromium `Accessibility.getFullAXTree` (computed role/name); PVC = AccessKit-bound `role`/`name` in the Snapshot.

**Standardised failure text:** `Not met: every interactive element must have an accessible name exposed to the accessibility API.`

## U2 — Every action is operable by keyboard alone.  `[A]`

**Operational definition**

- (a) Reachability: from document start, press Tab repeatedly (≤200) and record the focused node each time. Every visible, enabled interactive node (same set as U1) must appear in the sequence at 1280 px and at 320 px. For roving groups (`tab`, `radio`) at least one member per group must be reached, the rest by arrow keys (checked in (b)). No keyboard trap: the Tab sequence must end (focus leaves or cycles) before 200 presses.
- (b) Trace replay: every scripted-interaction A item of the specimen that PASSED in pointer mode is re-run with `mode=keyboard`: `activate(node)` = reach the node with Tab/arrow keys, then Enter (button, link, tab) or Space (checkbox, switch, radio); `type(node, text)` = reach it with Tab, then type. The same expectations must hold. Items that failed in pointer mode are not double-counted here.

**Standardised failure text:** `Not met: every action must be operable by keyboard alone.`

## U3 — Text contrast meets WCAG 2.2 AA.  `[A]`

**Operational definition**

- Snapshots at 1280 px and 320 px. Colours are the resolved `style.fg` and the backdrop composited from the nearest chain of `style.bg` fills up to white page background (alpha composited; ancestor `style.opacity` multiplies fg alpha).
- Every visible text node (own `text` non-empty, `visible_frac`>0, not disabled): contrast ratio ≥ 4.5:1, or ≥ 3:1 for large text (font size ≥ 24 px, or ≥ 18.66 px with weight ≥ 700). Ratio = (L1+0.05)/(L2+0.05), WCAG relative luminance.
- UI components (non-text, WCAG 1.4.11): for text inputs, combo-box and custom (non-`native`) checkbox/radio/switch/slider: the control boundary (`style.border` colour if width ≥ 1, else the node's own fill) against its parent backdrop ≥ 3:1. Buttons that have a text label and natively drawn controls are exempt.
- Text over a gradient/image (`bg_unknown`) cannot be resolved: it is skipped and counted in the verbose log (not a failure).
- Disabled controls and their text are exempt (WCAG inactive component). Placeholder text is not examined (not in the Snapshot).

**Standardised failure text:** `Not met: text contrast must meet WCAG 2.2 AA (4.5:1 text, 3:1 large text and UI components).`

## U4 — At a 320-px-wide viewport there is no overlap, clipping of meaningful content, or horizontal scroll (unless the specimen asks for it).  `[A]`

**Operational definition**

- Snapshot at viewport 320×800, text scale 1 (see `u4Check`).
- (i) Horizontal scroll: `meta.scroll_overflow_x` ≤ 1 px.
- (ii) Clipping: every visible node with own text, every visible interactive node and every visible non-decorative `image` has `visible_frac` ≥ 0.95 (i.e. not cut off by a clip container, by an ellipsis/line-clamp truncation, or by the viewport's left/right edges).
- (iii) Overlap: content boxes = per-line text boxes, interactive-node rects, non-decorative image rects. Two boxes from different nodes where neither node is an ancestor of the other, and whose intersection area ≥ 25% of the smaller box (and ≥ 3 px in both dimensions), is an overlap. Purely decorative nodes (`a11y_hidden` with no text) are ignored.
- No specimen in the pilot set asks for horizontal scroll.

**Standardised failure text:** `Not met: at a 320-px-wide viewport there must be no overlap, clipping of meaningful content, or horizontal scroll.`

## U5 — At 200% text scale, U4 still holds.  `[A]`

**Operational definition**

- Same as U4 but with text scale 2.0 at 320 px width. Baseline: the checker sets `html{font-size:200% !important}` (every rem/em/% sized length scales; px-sized type does not — nearest a web author can be tested against; stated deviation). PVC: `text_scale: 2.0` scales all text.

**Standardised failure text:** `Not met: at 200% text scale, there must still be no overlap, clipping of meaningful content, or horizontal scroll at a 320-px-wide viewport.`
