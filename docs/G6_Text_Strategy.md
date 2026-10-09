# G6 — Text Strategy

**Gate:** G6 in §8 of the analysis ("text strategy decision with a quality bar").
**Status:** decided on paper, 2026-10-08, for supervisor red-team. Text is the highest-risk subsystem (A§2.4, A§3 risk 3). This document narrows that risk but does not remove it. The remaining risk can only be measured, and §9 says where.
**Reference convention:** `A§n` = main analysis; `G4 §n` = IR spec; `[§9.n]` = analysis verified findings; `[G7 S.n]` = G7 supplementary findings; bare `§n` = this document.

---

## 1. Decisions in one table

| Question | Decision |
|---|---|
| Who shapes, segments and lays out text? | **Parley** with HarfRust (shaping), Fontique (enumeration and fallback), Skrifa (font reading, hinted outlines) and ICU4X (segmentation, bidi, locale data) [§9.7]. Pinned versions in G7 §2. **PVC writes no shaper, no bidi algorithm, no line breaker and no font parser** |
| Where are fonts from? | **Bundled, content-hashed font sets by default**, so text layout is deterministic across devices (G4 §14 D3). System fonts are opt-in (`PVC-W-SYSTEM-FONT`), and are used for *user-typed* text in scripts the bundle lacks |
| How are glyphs rasterised? | **On the CPU into a grayscale glyph atlas on every tier.** Tier 0 composites on the CPU. Tiers 1 and 2 sample the atlas in a fragment shader, with no compute. **Display type** (≥ 48 px) may use MSDF or analytic outlines on Tier 2 only. Very large type (≥ 96 px) goes through the vector raster backend on all tiers, not the atlas |
| What is the quality bar? | Measured against **Chromium** at **11, 13 and 16 px** for **Latin, CJK, Arabic, Devanagari and emoji**, with **LDR-FLIP** [G7 S.6] as the named metric, and a *relative* threshold (§5) |
| Editing, IME, selection, clipboard | Plain-text editing on Parley's selection and editing utilities [G7 §3, row 9.7]. Platform IME through each host. On the **web**, a hidden native `<input>`/`<textarea>` overlay, as Flutter does [G7 S.3]. Clipboard through capabilities (G4 §10) |
| What is out of v0? | Vertical text, ruby, rich-text editing, LCD subpixel anti-aliasing, and the rest of §8 |

## 2. Why this stack, and what it does not solve

**Why.** The hard parts of text are shaping (ligatures, contextual Arabic forms, Indic reordering), bidirectional layout, segmentation and font fallback. Parley's stack covers them and is already used in production by Vello's `glifo` and by Dioxus [§9.7]. Parley can be run against the Web Platform Tests text-layout suite via Blitz's runner [G7 §3, row 9.7], which gives PVC a ready-made correctness check against the same standard browsers follow. Writing any of this in-house would be the largest avoidable risk in the project.

**What it does not solve**, and this document therefore owns:
1. **Small-size raster quality.** How glyphs look at 11–13 px on a 1× display. This is where SDF-only approaches fail (A§2.4) and where users compare with their browser.
2. **IME and editing integration** on five host types (Windows, macOS, Linux, Android, iOS) plus the web.
3. **Font bundling cost**, against the 150 MB memory ceiling (A§4.4.1 rule 4) and download size.
4. **Determinism.** The same text must lay out the same way everywhere (G4 §14 D3).

## 3. Fonts

- **Default: a bundled font set**, chosen at publish time and pinned by content hash in the manifest (G4 §3). The set covers the scripts in the app's strings table, plus the scripts of the app's **declared input locales**, so that user typing in those locales renders.
- **Subsetting.** Authored text is static, so authored-only faces are subset to the glyphs the strings table needs. Faces used for user input are not subset, because the user may type anything.
- **Size reality (*estimates*, to be measured):** a Latin UI family is small (hundreds of KB). A full CJK face is several MB to over 10 MB per weight. A colour emoji font is roughly 10 MB or more. An app declaring CJK *input* therefore pays a real download and memory cost. *Decision:* the publish tool shows that cost per declared input locale, so the author chooses consciously. Nothing is silently dropped.
- **User-typed text outside the bundle** falls back to system fonts through Fontique. Layout of *user* content may then differ across devices. That is acceptable, because D3 covers authored content, and the runtime records it in developer preview.
- **Emoji:** one bundled colour emoji face when the app declares emoji support, either explicitly or because the strings table contains emoji. Which colour-font formats the pinned Skrifa/raster stack renders (COLRv1, CBDT, sbix) is *unverified in this pass*. *Decision:* the bundled emoji face is chosen in a format the pinned stack is shown to render on Tier 0. That is a prototype check (§9).

## 4. Rasterisation per tier

| Aspect | Tier 0 (CPU) | Tier 1 (GL ES 3.0 / WebGL2, fragment-only) | Tier 2 |
|---|---|---|---|
| Body text (< 48 px) | CPU-rasterised grayscale atlas, CPU composite | Same atlas, uploaded as a texture, drawn as textured quads | Same as Tier 1 |
| Display text (48–95 px) | Atlas at size | Atlas at size | MSDF or analytic outlines permitted |
| Very large text (≥ 96 px) | Vector raster backend (outline as path) | Vector raster backend | Vector raster backend or analytic |
| Colour emoji | Colour bitmap or colour outline in an RGBA atlas | Same | Same |
| Subpixel positioning | Horizontal, 4 buckets (¼ px) | Same | Same |
| Hinting | Font hinting at ≤ 16 px when device scale < 1.5; otherwise unhinted | Same | Same |
| Small-size contrast | One gamma and stem-darkening curve, tuned against the bar (§5) | Same | Same |
| LCD subpixel anti-aliasing | No (out of v0) | No | No |
| Atlas memory cap (*decision*) | 16 MB, LRU eviction | 16 MB texture budget | 64 MB |

**Why the same atlas on every tier:** text pixels must be within the D4 tolerance across tiers (G4 §14). If the glyph *coverage* comes from the same CPU rasteriser everywhere, differences can arise only in compositing, which is the easy part. Vello's renderers share glyph-run support through `glifo` [G7 §2]. `glifo` is the candidate for this path, and pinning it at 0.4.0 keeps the Skrifa version consistent with Parley's.

**Why not SDF text by default:** the analysis already rejects it below display sizes (A§2.4, A§4.3). This document adds one reason: CJK atlas pressure. One SDF per glyph at a single resolution saves memory, but loses exactly the small-size hinting and contrast that the quality bar measures.

**Hinting and stem darkening are tuning parameters, not fixed truths.** The defaults above are starting points. The bar (§5) decides the final values, per platform if necessary. *Unverified in this pass:* how Chromium hints on each OS. The suite measures it rather than assuming it.

## 5. Quality bar

**Reference.** Chromium, at a pinned version, rendering the same string with **the same font file**, at the same CSS pixel size, on the same machine, at device scale 1.0 and 2.0. Using the same font file isolates rasterisation and layout from font choice.

**Test strings** (fixed corpus, versioned):
- **Latin:** UI labels, a pangram paragraph, digits and punctuation.
- **CJK:** Simplified Chinese and Japanese UI phrases, with kana and kanji mixed.
- **Arabic:** joined text, digits, mixed Arabic–Latin bidi.
- **Devanagari:** conjuncts and matras. *Added to the supervisor's proposed list* because Hindi is in G2's language arm and Indic shaping is a classic failure point (A§2.4).
- **Emoji:** single, ZWJ sequences, skin-tone modifiers, flags.

**Sizes:** 11, 13 and 16 px, plus 24 px as a sanity check.

**Measurements:**
1. **Raster fidelity: LDR-FLIP** [G7 S.6] mean error between PVC's render and Chromium's, per script, per size, per device scale.
2. **Layout equality:** line-break positions and glyph advances compared with Chromium for the same width. Line breaks must match **exactly** on the corpus. Glyph positions must match within ¼ px.
3. **Shaping equality:** shaped glyph-ID sequences compared with a HarfBuzz reference. They must match exactly, since HarfRust is a HarfBuzz port [§9.7].
4. **Cross-tier parity:** Tier 0 against Tier 2 FLIP, within the D4 tolerance (G4 §14).
5. **Human legibility check:** at 11 and 13 px, a blind side-by-side preference test with at least 20 people per script, including native readers for CJK, Arabic and Devanagari.

**Threshold (relative, decision):**
- For each script, size and scale: **PVC's mean FLIP against Chromium must not exceed the mean FLIP of Firefox (pinned) against Chromium** on the same strings and machine. In words: PVC may differ from Chromium no more than another production browser does.
- In the human check, PVC is **chosen at least 40% of the time** against Chromium. That allows "not noticeably worse", not "must be better".
- Layout and shaping equality (items 2 and 3) are **pass/fail**.

**Why relative.** No sourced absolute FLIP threshold for text legibility exists in this program's findings, and inventing one would break the "no invented statistics" rule. Anchoring to browser-to-browser difference measures exactly what users experience as "normal". **Its weakness:** if Firefox and Chromium share rasterisation code on a platform (*unverified*), their difference understates normal variance and the bar becomes too strict. If so, the human check decides and the deviation is published.

**Gate rule (rescoped 2026-10-09):** text is "done" only when the bar passes on **Tier 0 (c)** in CI, with results published (A§6 item 4). Results from physical devices are welcome as community contributions; they are not a gate, because target devices are the developer's choice (A§4.4).

## 6. Editing, IME, selection, clipboard

**Editing model (v0).** Plain text only: single-line and multi-line, password (masked), numeric, email and search variants (G4 §4 `text-input`). It is built on Parley's selection and editing utilities. Caret and selection movement follow grapheme, word and line boundaries from ICU4X segmentation. In bidi text, keys move the caret in **logical** order and the caret is drawn at the **visual** position. Undo and redo apply per field.

**IME (composition input, needed for CJK and many other scripts):**

| Host | Path | Status in this pass |
|---|---|---|
| Windows / macOS / Linux | Windowing-layer IME events (winit 0.30, pinned in G7) feeding composition into the field, with the candidate window positioned at the caret | *Unverified:* completeness of IME support per OS in the pinned winit. A prototype check (§9) |
| Android | The thin host implements the platform's text-input connection for the soft keyboard and IME | *General knowledge, unverified in this pass* |
| iOS | The thin host implements the platform's text-input protocol | *General knowledge, unverified in this pass* |
| Web | **Hidden native `<input>`/`<textarea>` placed pixel-aligned over the canvas field.** The browser handles IME, autofill and spellcheck; the canvas draws the text | Prior art: Flutter web [G7 S.3] |

**Consequence for the analysis:** the mobile hosts must carry text-input and accessibility bridging. A§1.7's "the host shell is a few hundred lines" is therefore optimistic. *Estimate:* the iOS and Android hosts are each a small but real native component, and their text-input part is the riskiest piece. The integration report lists this as an inconsistency.

**Selection UI:** mouse drag, double-click to select a word, triple-click to select a line. Touch handles on mobile. Keyboard: Shift with arrows, word modifiers, Home and End.

**Clipboard:** copy uses `clipboard.write` (implicit, shown to the user). Paste is a user gesture that grants `clipboard.read` for that gesture only (G4 §10). Plain text only in v0.

**Accessibility of text:** AccessKit adapters support single- and multi-line text input but not rich text [G7 S.1]. v0's plain-text-only editing matches what can be exposed accessibly today. On the web, the native overlay also *is* the accessible text field, and the semantics mirror (G4 §8.3) points at it.

## 7. Performance notes (estimates, to be measured)

- Shaping runs on the CPU when text changes, not every frame. Shaped runs are cached per (string, style, width).
- *Estimate:* typing in a single field on Tier 0 reference (a), the Raspberry Pi 4, should fit within the 33 ms interaction budget for Latin. CJK with IME is unknown. This is an explicit prototype measurement (§9).
- Atlas thrash risk: a CJK-heavy screen at several sizes can exceed the 16 MB Tier 0/1 cap. The fallback is re-rasterisation, which costs frame time, not correctness.

## 8. Out of v0 (explicit)

Vertical text (CJK vertical writing, Mongolian) — G2 X42 expects a diagnostic. Ruby/furigana. Rich-text editing (styled spans inside inputs). LCD subpixel anti-aliasing. Hyphenation. Arabic kashida justification. Text on a path. Author-controlled OpenType features beyond tabular numerals (*decision:* tabular numerals are in, for data grids). Variable-font animation. Native spellcheck on desktop and mobile hosts (the web overlay gets the browser's). Emoji picker UI. Find-in-page.

Each is a known gap, not an oversight. Each returns as an RFC with its own quality bar.

## 9. Residual risks and the measurements that would close them

| # | Risk | Closed only by |
|---|---|---|
| 1 | Grayscale anti-aliasing may look blurrier than native text on 1× Windows displays, where users expect ClearType-style rendering (*unverified*) | The §5 human check on a 1× Windows Tier 1 laptop |
| 2 | Relative FLIP bar too strict or too loose (§5 weakness) | First measurement of Firefox against Chromium |
| 3 | IME completeness in pinned winit per OS; iOS/Android host text-input effort | A Tier 1 prototype typing CJK through the system IME on each host |
| 4 | Emoji colour-font format support in the pinned stack | Rendering the emoji corpus on Tier 0 |
| 5 | Web overlay drift: the native input's caret and the canvas text misalign at fractional scales | Web prototype at device scales 1.0, 1.25, 1.5, 2.0 |
| 6 | CJK font download size makes CJK-input apps heavy on budget devices | Measuring publish size and memory for a CJK-input specimen from G2 |
| 7 | Atlas memory on Tier 0/1 under CJK load | Measuring atlas pressure on the Raspberry Pi 4 and the budget Android reference |

**Bottom line:** the *algorithms* are a solved dependency. The *integration* (IME on six hosts, the web overlay, fonts within budget) and the *perceptual bar* are not, and no amount of paper closes them. This is one of the reasons the program's confidence stops below 95% without a prototype (A§0).
