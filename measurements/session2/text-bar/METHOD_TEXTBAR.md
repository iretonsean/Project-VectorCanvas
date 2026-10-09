# METHOD_TEXTBAR.md — reduced G6 text bar (A8), method written BEFORE any text-bar measurement

**Author:** dev-measure. Corpus: `corpus.json` (same directory, frozen before any render; sha256 recorded in RESULT.md).
**Host label (all numbers):** `host=container-xeon4`, Linux rasterisers only (Chromium 141.0.7390.37 headless, Firefox 142.0.1 Playwright build v1495 headless, PVC = Parley 0.11.1 + vello_cpu 0.3.0 on CPU). Not a Tier 1 device result.

## 1. What is compared
1. **PVC** text rendering: Parley 0.11.1 shaping/layout (HarfRust 0.12) → `vello_cpu` 0.3.0 glyph runs into an RGBA8 pixmap, grayscale AA, no LCD AA (G6 §4). Entry point: *see §6 on which text API*.
2. **Chromium 141** (reference, G6 §5) DOM text.
3. **Firefox 142** DOM text (the G6 §5 relative-threshold yardstick).

**Same font files for all three** (G6 §5): the bundled faces dev-render ships, `/workspace/dev_work/dev-render/pvc-runtime/fonts/{DejaVuSans.ttf, DejaVuSans-Bold.ttf, NotoSansCJKsc-Regular.otf}` (current dev-render bundle; see Addendum A4: this replaced Liberation Sans) (sha256 in RESULT.md). Browsers get them through `@font-face` from a local `file:`/loopback page with `font-family: PVCLatin, PVCCJK` (and nothing else; the harness asserts via `document.fonts` and a glyph-coverage check that no system fallback font is used). PVC uses the same two faces with the same per-character fallback order (Latin face first, CJK face for characters the Latin face lacks).

## 2. Conditions
- Strings: `corpus.json` (29 items: Latin 15 = L01 strings, F01 strings, G6 pangram/digits/punctuation; zh-CN 14 = L01/F01 zh-CN strings + UI phrases). Latin and Simplified Chinese only.
- CSS font sizes **11, 13, 16, 24 px**; device scale **1.0 and 2.0** (browser `deviceScaleFactor`; PVC renders at scale 2.0 with font size ×2 and unhinted per G6 §4 "hinting ≤16 px when scale < 1.5, otherwise unhinted").
- One image per (script, size, scale), containing every item of that script stacked in fixed-height slots (slot height = `slot_lines × line_height + 8` CSS px; the slot height is fixed in advance so a differing line count in one engine cannot move later items). Canvas 800 CSS px wide, height = Σ slots. **Background white (#fff), text black (#000)**, left margin 8 CSS px. Line height = `round(1.35 × size)` CSS px in all three engines (see Addendum A2). Wrapped items ("paragraph") use box width 272 CSS px (L01 card inner width class), `white-space: normal`/Parley wrap width, left-aligned, `lang` = `en` / `zh-CN`. Labels are single-line.
- Single polarity (black on white). Light-on-dark is **not** measured (gap; stem-darkening behaves differently with polarity).
- Browsers: grayscale AA is verified by checking that the browser screenshots contain no coloured (R≠G≠B beyond tolerance 2) pixels in text areas; if LCD AA appears the cell is flagged and an attempt is made to disable it (flag) and reported.
- Screenshots at the same pixel size as PVC's image: `800×DSF` wide.

## 3. Measures (per script × size × scale)
**M1 raster fidelity, PVC vs Chromium.** `flip-evaluator` 1.7, LDR-FLIP, default viewing conditions (`pixels_per_degree` default of the library, ≈67; stated, not tuned). Inputs: sRGB images as float [0,1]. Two numbers per pair:
- **FLIP-mean (primary):** mean of the FLIP error map over the **whole image** (G6 §5 "mean error"). The blank area dilutes the mean equally for all pairs, so the *relative* comparison is not biased by it.
- **FLIP-ink (secondary):** mean over pixels where either image differs from the background by > 0 (ink union mask).
**M2 raster fidelity, Firefox vs Chromium**, same two numbers.
**M3 pass (G6 §5 relative threshold):** PASS iff `FLIP-mean(PVC,Chromium) ≤ FLIP-mean(Firefox,Chromium)`. Also reported on FLIP-ink (secondary pass) and the ratio M1/M2.
**M4 line-break equality** at the fixed width (272 CSS px) for every `paragraph` item: the sorted list of line start offsets (UTF-16 code-unit offsets into the string) must be **identical** between PVC (Parley `Layout::lines()` text ranges) and Chromium (per-character `Range.getClientRects()` grouped by line `top`). Pass = exact match on all items for that (script, size, scale). Mismatching items are listed with both break lists. Firefox's breaks are also extracted the same way and reported (informational).
**M5 registration offset (informational, pre-declared):** because baseline placement/rounding can differ by a pixel between engines independent of rasterisation, the harness also computes FLIP-mean after the best integer-pixel translation of the PVC (or Firefox) image over `dx,dy ∈ [-2,2]·DSF` px relative to Chromium, and reports the offset found. This **aligned** figure is secondary; the **primary pass uses no alignment** (as in G6 §5). The same treatment is applied to the Firefox↔Chromium pair, so the aligned pass is also like for like.

Measures **not** run here (labelled gaps): G6 §5 items 3 (shaping equality vs a HarfBuzz reference; HarfRust is a HarfBuzz port but no reference comparison is made), item 4 (cross-tier parity; owned by dev-render's TOLERANCE), item 5 (human legibility, ≥20 people per script) and glyph-position equality within ¼ px (only line breaks are compared).

## 4. Pass/fail honesty rules (fixed now)
- A cell passes or fails exactly by M3 on FLIP-mean; failures are reported, not tuned away. No parameter of PVC's text path (gamma, stem darkening, hinting) is tuned **inside this measurement**. If PVC's path as delivered by dev-render has such knobs, the value used is the shipped default, recorded in RESULT.md.
- If a browser screenshot reveals a font-fallback or LCD-AA artefact, the affected cells are flagged `invalid-reference` and not counted as pass.
- 16 cells (4 sizes × 2 scales × 2 scripts) are reported; no cell is dropped. Overall verdict is "N of 16 cells pass M3; M of 16 pass M4".

## 5. Known weaknesses of the bar, stated in advance (G6 §5 and §9 risk 2)
- **Firefox and Chromium may share rasterisation code on Linux** (both can use FreeType and Skia-family backends and the same fontconfig settings). *Unverified* here. If their mutual FLIP is very small, the relative bar becomes strict. The RESULT reports the absolute FLIP values next to the verdict so the reader can judge.
- Single machine, single fontconfig, headless; no human judgement; no Arabic/Devanagari/emoji; no Windows/macOS/Android rasterisers (G6 §9 risk 1 stays open).
- Chromium/Firefox use font hinting from the host's fontconfig (recorded in RESULT.md as `fc-match` output); PVC uses its own configured hinting. Differences in hinting are one of the things measured, not removed.

## 6. PVC text path used
The brief says "through dev-render's text API". At the time this method was written `pvc_runtime` contained no text API (src/lib.rs was a 1-line placeholder). Rule: if dev-render's CHECKPOINT publishes a text-draw entry point before the measurement run, the harness uses it (`tools/textbar/pvc_text` with `--backend pvc_runtime`). Otherwise the harness uses its own glue (`--backend local-parley-vello_cpu`) that depends only on the pinned crates (parley 0.11.1, fontique 0.11.1, harfrust 0.12.0, skrifa 0.44.0, vello_cpu 0.3.0, glifo 0.4.0) and the same font files, and RESULT.md says so. The backend actually used is printed in every result file.

## 7. Re-run
`tools/textbar/run_textbar.sh` regenerates everything (renders, FLIP, line breaks, RESULT.md) with `flock -s` on the CPU lock (heavy job, not a timing measurement).

## 8. Addendum A1 (written after the browser renders existed, before ANY FLIP or PVC render was computed)
Observed: the container's fontconfig (`fc-match -v sans`: `rgba=1`, `hintstyle=1` slight, `antialias=True`) makes **Chromium 141 render LCD subpixel text** (17–107 k coloured pixels per image); **Firefox 142 renders grayscale**. PVC is grayscale only (G6 §4: LCD AA out of v0). Comparing grayscale PVC with LCD Chromium would charge PVC for a rendering mode the doc excludes. Per §2 ("if LCD AA appears … an attempt is made to disable it and reported"), the **primary reference is Chromium launched with `--disable-lcd-text`** (verified: 0 coloured pixels), which is the only Chromium change. The default LCD Chromium renders are kept (`work/chromium-lcd/`) and the Firefox↔Chromium(LCD) and PVC↔Chromium(LCD) FLIPs are reported as a **secondary** table. Hinting is left at the host's fontconfig default in both browsers (slight). Pass/fail rules of §3/§4 are unchanged and apply to the grayscale primary.

## 9. Addendum A2 (still before any FLIP or PVC render was computed)
dev-render's CHECKPOINT (2026-10-08) published the text entry point `pvc_runtime::render::text_display_list` + `render_cpu_rgba` and fixes line height at `round(size × 1.35)` logical px, layouts built at scale 1.0 and scaled by the renderer. Per §6, the harness therefore uses **that API as delivered** (not harness-local glue), and the browsers' line height is changed from `round(1.4 × size)` to `round(1.35 × size)` so all three engines use the same line box (this is a change to the *browsers' CSS*, made to match PVC's fixed property, not a tuning of any result). The browser renders produced before this addendum (line height 1.4) are discarded and regenerated. Because the API renders one string per call, the harness renders each corpus item on its own canvas (792 × (slot−4) CSS px, pad 0) and pastes it, at integer pixel offset (8·scale, (slot top+4)·scale), into the page image; this reproduces the browser page layout exactly. Line breaks are read from `TextSystem::layout` (the same layout the renderer draws). The Chromium line-break override that Parley offers (`CHROMIUM_LINE_BREAK_OVERRIDE`) is **not** enabled by the dev-render API; M4 is therefore measured with Parley's default breaker and the existence of the override is noted as a possible remedy if M4 fails (not applied).

## 10. Addendum A3 (before any FLIP computed)
M5 (aligned figure): the best integer shift is chosen by minimum sum of squared pixel difference over `dx,dy ∈ [-2·scale, +2·scale]` (a cheap proxy; FLIP is then computed once at that shift), not by minimising FLIP, to keep the run time bounded. The shift is reported.

## 11. Addendum A4 (font change by dev-render; written before the final run)
When this method was written and the first full run executed, dev-render's bundle used **Liberation Sans** for Latin. Minutes after that run dev-render replaced it with **DejaVu Sans / DejaVu Sans Bold** (their CHECKPOINT, "Fonts" table). The task is to test PVC's text path as shipped, with the same font file in all three engines, so the **final run uses DejaVu Sans (+ Noto Sans CJK SC Regular)** everywhere (browsers' `@font-face` and PVC's bundle). The first run (Liberation Sans) is kept, unedited, as `results_SUPERSEDED_liberation_fonts.json` for transparency and is **not** the result. Disclosure: I had seen the first-run numbers (Latin 11 px failed, 13–24 px passed, zh-CN mostly failed) before the final run was started; no parameter of the method, corpus or PVC path was changed in response to them (the only changes are the font files, forced by dev-render's change). The CJK face is unchanged (same sha256). A byproduct of the first run is a diagnostic on width differences (Chromium narrower than Firefox/PVC), investigated separately in `diag_kerning.out` (informational; not part of the bar).

## 12. Addendum A5 (written AFTER seeing the primary results of the final run; informational only)
The primary M4 results show PVC and Firefox mismatching Chromium on the *same* items, and a width diagnostic (`diag_kerning.out`) shows Chromium's default text on this host uses hinted integer advances (322.00 px vs Firefox 309.80 px for one 11 px string; `geometricPrecision` gives 309.84). To tell "PVC differs from Chromium" apart from "Chromium's Linux default differs from everyone", a **secondary** reference is added: Chromium launched with `--font-render-hinting=none --disable-lcd-text`. It does **not** replace the primary reference, and no pass/fail verdict of §3/§4 is changed by it. It was added after I had seen the primary numbers, so it is labelled post-hoc.
