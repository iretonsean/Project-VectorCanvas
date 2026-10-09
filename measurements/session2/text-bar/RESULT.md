# A8 — reduced G6 text bar: RESULT (Latin + Simplified Chinese)

**Host label:** `host=container-xeon4` (Linux, CPU rasterisers only). PVC = `pvc_runtime` text API (Parley 0.11.1 + vello_cpu 0.3.0, grayscale AA, dev-render's defaults, nothing tuned here); reference = Chromium 141.0.7390.37 headless with `--disable-lcd-text` (grayscale; see METHOD_TEXTBAR Addendum A1); yardstick = Firefox 142.0.1 (Playwright build v1495) headless. **Not a Tier 1 device result.**
**Method (written before measuring):** `METHOD_TEXTBAR.md` (+ Addenda A1–A4). Corpus: `corpus.json`. Raw: `results.json`, `work/*/*.png`, `work/*/breaks_*.json`. Environment/hashes: `run_env.txt`. Re-run: `tools/textbar/run_textbar.sh`.

## Summary

- **latin:** M3 (FLIP-mean, primary) pass 8/8; ink-masked 8/8; aligned 5/8; vs LCD-Chromium (secondary) 8/8; M4 line-break equality 4/8
- **zh-CN:** M3 (FLIP-mean, primary) pass 2/8; ink-masked 1/8; aligned 1/8; vs LCD-Chromium (secondary) 2/8; M4 line-break equality 4/8
- **Overall:** M3 primary 10/16 cells pass; M4 8/16 cells pass. The bar is **not** passed as a whole (G6 §5 requires pass in every script/size/scale cell and exact line breaks).
- No human legibility study, no Arabic/Devanagari/emoji, no Japanese, Linux rasterisers only, single machine, single polarity (black on white). A pass here is not a statement about Windows/macOS/Android/ChromeOS text.

## M1–M3 raster fidelity (LDR-FLIP, mean over whole image; lower = closer to Chromium)

| script | px | scale | FLIP PVC↔Chromium | FLIP Firefox↔Chromium | ratio | M3 pass | ink-masked PVC / FF | ink pass | aligned PVC / FF (shift PVC, FF) | aligned pass |
|---|---|---|---|---|---|---|---|---|---|---|
| latin | 11 | 1 | 0.0206 | 0.0272 | 0.76 | **PASS** | 0.340 / 0.397 | PASS | 0.0201 / 0.0196 ([1, 0], [1, -1]) | FAIL |
| latin | 11 | 2 | 0.0261 | 0.0278 | 0.94 | **PASS** | 0.548 / 0.580 | PASS | 0.0259 / 0.0251 ([1, 0], [1, -1]) | FAIL |
| latin | 13 | 1 | 0.0176 | 0.0266 | 0.66 | **PASS** | 0.284 / 0.364 | PASS | 0.0176 / 0.0178 ([0, 0], [0, -1]) | PASS |
| latin | 13 | 2 | 0.0256 | 0.0269 | 0.95 | **PASS** | 0.524 / 0.544 | PASS | 0.0256 / 0.0269 ([0, 0], [0, 0]) | PASS |
| latin | 16 | 1 | 0.0195 | 0.0304 | 0.64 | **PASS** | 0.291 / 0.383 | PASS | 0.0195 / 0.0213 ([0, 0], [0, -1]) | PASS |
| latin | 16 | 2 | 0.0307 | 0.0347 | 0.88 | **PASS** | 0.557 / 0.607 | PASS | 0.0307 / 0.0295 ([0, 0], [0, -1]) | FAIL |
| latin | 24 | 1 | 0.0336 | 0.0360 | 0.93 | **PASS** | 0.370 / 0.387 | PASS | 0.0336 / 0.0360 ([0, 0], [0, 0]) | PASS |
| latin | 24 | 2 | 0.0383 | 0.0440 | 0.87 | **PASS** | 0.546 / 0.605 | PASS | 0.0383 / 0.0385 ([0, 0], [1, -1]) | PASS |
| zh-CN | 11 | 1 | 0.0166 | 0.0166 | 1.00 | **PASS** | 0.292 / 0.292 | FAIL | 0.0080 / 0.0078 ([0, -1], [0, -1]) | FAIL |
| zh-CN | 11 | 2 | 0.0187 | 0.0156 | 1.20 | **FAIL** | 0.465 / 0.397 | FAIL | 0.0123 / 0.0092 ([0, -2], [0, -1]) | FAIL |
| zh-CN | 13 | 1 | 0.0197 | 0.0198 | 1.00 | **PASS** | 0.315 / 0.315 | PASS | 0.0116 / 0.0114 ([0, -1], [0, -1]) | FAIL |
| zh-CN | 13 | 2 | 0.0247 | 0.0194 | 1.27 | **FAIL** | 0.541 / 0.448 | FAIL | 0.0154 / 0.0134 ([0, -2], [0, -1]) | FAIL |
| zh-CN | 16 | 1 | 0.0306 | 0.0225 | 1.36 | **FAIL** | 0.405 / 0.316 | FAIL | 0.0127 / 0.0127 ([0, -2], [0, -1]) | PASS |
| zh-CN | 16 | 2 | 0.0361 | 0.0234 | 1.54 | **FAIL** | 0.636 / 0.474 | FAIL | 0.0173 / 0.0142 ([0, -4], [0, -1]) | FAIL |
| zh-CN | 24 | 1 | 0.0424 | 0.0293 | 1.45 | **FAIL** | 0.457 / 0.326 | FAIL | 0.0175 / 0.0106 ([0, -2], [0, -1]) | FAIL |
| zh-CN | 24 | 2 | 0.0518 | 0.0295 | 1.75 | **FAIL** | 0.695 / 0.471 | FAIL | 0.0178 / 0.0114 ([0, -4], [0, -1]) | FAIL |

## Secondary: reference = Chromium with its default LCD subpixel text (fontconfig `rgba=RGB` on this host)

| script | px | scale | FLIP PVC↔Chromium-LCD | FLIP Firefox↔Chromium-LCD | pass |
|---|---|---|---|---|---|
| latin | 11 | 1 | 0.0245 | 0.0317 | PASS |
| latin | 11 | 2 | 0.0292 | 0.0310 | PASS |
| latin | 13 | 1 | 0.0216 | 0.0308 | PASS |
| latin | 13 | 2 | 0.0289 | 0.0301 | PASS |
| latin | 16 | 1 | 0.0247 | 0.0349 | PASS |
| latin | 16 | 2 | 0.0351 | 0.0391 | PASS |
| latin | 24 | 1 | 0.0410 | 0.0434 | PASS |
| latin | 24 | 2 | 0.0449 | 0.0504 | PASS |
| zh-CN | 11 | 1 | 0.0198 | 0.0199 | PASS |
| zh-CN | 11 | 2 | 0.0211 | 0.0181 | FAIL |
| zh-CN | 13 | 1 | 0.0233 | 0.0234 | PASS |
| zh-CN | 13 | 2 | 0.0275 | 0.0223 | FAIL |
| zh-CN | 16 | 1 | 0.0345 | 0.0265 | FAIL |
| zh-CN | 16 | 2 | 0.0393 | 0.0268 | FAIL |
| zh-CN | 24 | 1 | 0.0486 | 0.0360 | FAIL |
| zh-CN | 24 | 2 | 0.0570 | 0.0353 | FAIL |

## M4 line-break equality (wrapped items, width 272 CSS px; exact match required)

| script | px | scale | items | PVC mismatches vs Chromium | Firefox mismatches vs Chromium (informational) | pass |
|---|---|---|---|---|---|---|
| latin | 11 | 1 | 15 | lat-l01-bio, lat-f01-error, lat-mixed-case | lat-l01-bio, lat-f01-error, lat-mixed-case | **FAIL** |
| latin | 11 | 2 | 15 | lat-l01-bio, lat-f01-error, lat-mixed-case | lat-l01-bio, lat-f01-error, lat-mixed-case | **FAIL** |
| latin | 13 | 1 | 15 | lat-f01-terms, lat-digits | lat-f01-terms, lat-digits | **FAIL** |
| latin | 13 | 2 | 15 | lat-f01-terms, lat-digits | lat-f01-terms, lat-digits | **FAIL** |
| latin | 16 | 1 | 15 | — | — | **PASS** |
| latin | 16 | 2 | 15 | — | — | **PASS** |
| latin | 24 | 1 | 15 | — | — | **PASS** |
| latin | 24 | 2 | 15 | — | — | **PASS** |
| zh-CN | 11 | 1 | 14 | — | — | **PASS** |
| zh-CN | 11 | 2 | 14 | — | — | **PASS** |
| zh-CN | 13 | 1 | 14 | zh-ui-para | zh-ui-para | **FAIL** |
| zh-CN | 13 | 2 | 14 | zh-ui-para | zh-ui-para | **FAIL** |
| zh-CN | 16 | 1 | 14 | zh-ui-para | zh-ui-para | **FAIL** |
| zh-CN | 16 | 2 | 14 | zh-ui-para | zh-ui-para | **FAIL** |
| zh-CN | 24 | 1 | 14 | — | — | **PASS** |
| zh-CN | 24 | 2 | 14 | — | — | **PASS** |

Mismatch detail (line start offsets, UTF-16, first non-space char of each line):

- `latin_s11_x1` `lat-l01-bio`: Chromium [0, 45] vs PVC [0, 49]
- `latin_s11_x1` `lat-f01-error`: Chromium [0, 40] vs PVC [0, 48]
- `latin_s11_x1` `lat-mixed-case`: Chromium [0, 45] vs PVC [0, 52]
- `latin_s11_x2` `lat-l01-bio`: Chromium [0, 45] vs PVC [0, 49]
- `latin_s11_x2` `lat-f01-error`: Chromium [0, 40] vs PVC [0, 48]
- `latin_s11_x2` `lat-mixed-case`: Chromium [0, 45] vs PVC [0, 52]
- `latin_s13_x1` `lat-f01-terms`: Chromium [0, 42] vs PVC [0, 38]
- `latin_s13_x1` `lat-digits`: Chromium [0, 38, 76] vs PVC [0, 35, 74]
- `latin_s13_x2` `lat-f01-terms`: Chromium [0, 42] vs PVC [0, 38]
- `latin_s13_x2` `lat-digits`: Chromium [0, 38, 76] vs PVC [0, 35, 74]
- `zh-CN_s13_x1` `zh-ui-para`: Chromium [0, 20, 41] vs PVC [0, 19, 40]
- `zh-CN_s13_x2` `zh-ui-para`: Chromium [0, 20, 41] vs PVC [0, 19, 40]
- `zh-CN_s16_x1` `zh-ui-para`: Chromium [0, 17, 35, 52] vs PVC [0, 17, 34, 51]
- `zh-CN_s16_x2` `zh-ui-para`: Chromium [0, 17, 35, 52] vs PVC [0, 17, 34, 51]

## Secondary (Addendum A5, informational): reference = Chromium with `--font-render-hinting=none` (fractional advances, no hinting), grayscale

| script | px | scale | FLIP PVC | FLIP Firefox | pass | line-break mismatches PVC | line-break mismatches Firefox |
|---|---|---|---|---|---|---|---|
| latin | 11 | 1 | 0.0067 | 0.0187 | PASS | — | — |
| latin | 11 | 2 | 0.0033 | 0.0132 | PASS | — | — |
| latin | 13 | 1 | 0.0089 | 0.0210 | PASS | — | — |
| latin | 13 | 2 | 0.0036 | 0.0108 | PASS | — | — |
| latin | 16 | 1 | 0.0090 | 0.0218 | PASS | — | — |
| latin | 16 | 2 | 0.0044 | 0.0172 | PASS | — | — |
| latin | 24 | 1 | 0.0116 | 0.0215 | PASS | — | — |
| latin | 24 | 2 | 0.0098 | 0.0265 | PASS | — | — |
| zh-CN | 11 | 1 | 0.0138 | 0.0138 | PASS | — | — |
| zh-CN | 11 | 2 | 0.0183 | 0.0150 | FAIL | — | — |
| zh-CN | 13 | 1 | 0.0231 | 0.0231 | FAIL | zh-ui-para | zh-ui-para |
| zh-CN | 13 | 2 | 0.0233 | 0.0170 | FAIL | zh-ui-para | zh-ui-para |
| zh-CN | 16 | 1 | 0.0300 | 0.0213 | FAIL | — | — |
| zh-CN | 16 | 2 | 0.0342 | 0.0191 | FAIL | — | — |
| zh-CN | 24 | 1 | 0.0424 | 0.0290 | FAIL | — | — |
| zh-CN | 24 | 2 | 0.0512 | 0.0290 | FAIL | — | — |

Chromium width diagnostic (`diag_kerning.out`, DejaVu Sans 11 px, one Latin string): default Chromium 322.00 px, `font-kerning:none` 333.00, `text-rendering:geometricPrecision` 309.84; Firefox default 309.80. i.e. on this host Chromium's default text uses hinted integer advances (wider), Firefox fractional ones. This explains why Firefox's line breaks differ from Chromium's in exactly the same items as PVC's do.

## Environment

```
date_utc=2026-10-09T00:32:38Z
loadavg=8.47 2.60 1.66 18/301 11575
fc-match:  antialias: True(w); hintstyle: 1(i)(w); rgba: 1(i)(w);
4437e33de6a494c0a5a9aa2ab52442fe18f0659511f40e40996a53fd6b2abd25  /workspace/dev_work/dev-measure/measurements/session2/text-bar/corpus.json
ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280  /workspace/dev_work/dev-render/pvc-runtime/fonts/DejaVuSans.ttf
5c1247acef7f2b8522a31742c76d6adcb5569bacc0be7ceaa4dc39dd252ce895  /workspace/dev_work/dev-render/pvc-runtime/fonts/DejaVuSans-Bold.ttf
6ead94f36c0f9423a88356f5a58b2b1b8180a3ff553ecab2f7a816990b306c60  /workspace/dev_work/dev-render/pvc-runtime/fonts/NotoSansCJKsc-Regular.otf
dev-render src tree sha256 (cat of src/*.rs):
dd5ff8cf2bd5fe41d685262d4c6badb7eb314f8981838f32f038409cf836fb00  -
```

## Labelled gaps (restated)

- **No human legibility study** (G6 §5 item 5: ≥20 people per script) — not run; the 40 % preference criterion is unevaluated.
- **Arabic, Devanagari, emoji, Japanese** not run. Simplified Chinese only for CJK.
- **Linux rasterisers only** (Chromium/Skia, Firefox, vello_cpu on a Xeon, headless). G6 §9 risk 1 (1× Windows ClearType expectations) stays open.
- **G6 §5 caveat, unverified:** Chromium and Firefox may share rasterisation code (FreeType + fontconfig settings) on Linux, which would make Firefox↔Chromium unusually small and the relative bar strict. The absolute FLIP values above let the reader judge; no check of shared code was made.
- Shaping equality vs a HarfBuzz reference (G6 §5 item 3), glyph-position equality within ¼ px (item 2, second half) and cross-tier parity (item 4) were **not** run here.
- Line height is fixed at round(1.35 × size) because dev-render's text API fixes it; the L01/F01 specimens may use other line heights.
- Chromium LCD text on this host is fontconfig-driven; the grayscale reference required `--disable-lcd-text`, which is a deviation from "Chromium default".

## Findings and what they do / do not show (hand-written; every number is in the tables above or in `results.json`)

**Checked (computed by `tools/textbar/run_textbar.sh`, files above):**
1. Primary bar (grayscale Chromium reference, no tuning): **Latin passes M3 in all 8 cells** (PVC closer to Chromium than Firefox is); **zh-CN passes M3 in 2 of 8 cells** (11 px ×1 as a tie, 13 px ×1 as a near-tie 0.0197 vs 0.0198) and fails 11 px ×2, 13 px ×2 and all 16/24 px cells; the PVC-to-Firefox FLIP ratio grows with size (about 1.4–1.8 at 16–24 px). The bar is therefore **not passed** for Simplified Chinese on this host.
2. Latin, after the post-hoc A5 reference (Chromium with unhinted fractional advances): PVC↔Chromium FLIP-mean 0.003–0.012, Firefox↔Chromium 0.011–0.027, and all Latin line breaks agree. PVC's Latin layout and rasterisation are close to *unhinted* Chromium on this host.
3. M4 failures of PVC (Latin 11/13 px; zh-CN 13/16 px) occur on exactly the same items where Firefox fails against default Chromium. A width diagnostic (`diag_kerning.out`) shows Chromium's default text on this host uses **hinted integer advances** (322.00 px) while Firefox (309.80 px) and `geometricPrecision` Chromium (309.84 px) use fractional advances. So the Latin M4 failures are a Chromium-on-this-host property that Firefox shares with PVC, not a PVC-only fault; with unhinted Chromium all Latin items match.
4. zh-CN 13 px has one line-break mismatch (`zh-ui-para`, PVC breaks one character earlier, before 在, where Chromium breaks before the opening quote “) that persists against unhinted Chromium. Enabling Parley's `CHROMIUM_LINE_BREAK_OVERRIDE` does **not** change it (`diag_linebreak_override.out`: 7 mismatches vs default Chromium with and without the override; 1 vs unhinted Chromium with and without).
5. zh-CN vertical placement: the best integer shift that aligns PVC to Chromium is 1–2 CSS px (dy −2 at ×1, −4 at ×2 for 16–24 px) while Firefox needs only about 1 px (`aligned` column). After alignment PVC's FLIP equals Firefox's at 16 px ×1 (0.0127 vs 0.0127) and is still worse at 24 px (0.0175 vs 0.0106). Latin shows no such offset.

**Inferred (not checked):**
- The zh-CN gap at 16–24 px looks dominated by a baseline offset for the CJK face (PVC draws the Noto CJK line lower than Chromium by about 1–2 CSS px). *Inferred cause:* the ascent/descent used for half-leading and baseline differs between Parley and Chromium for the CJK face. I did not instrument Parley to confirm it, and I did not change PVC. It is a lead for dev-render, not a result.
- The line-break difference in finding 4 is probably a quotation-mark class difference (UAX #14 treats U+201C as a quote with no break either side, Chromium evidently breaks before it). *Inferred*; only the outcome (break offsets) was checked.
- Firefox↔Chromium may be unusually small if they share rasterisation code (G6 §5 caveat, *unverified*); the absolute FLIP values (0.011–0.04 Latin, 0.006–0.03 zh-CN) are given so the reader can judge.

**Labelled gaps:** no human legibility study; Arabic/Devanagari/emoji/Japanese not run; Linux rasterisers only (this container's fontconfig makes default Chromium use LCD subpixel text with slight hinting; the grayscale reference needs `--disable-lcd-text`); black-on-white only; line height fixed at 1.35× by dev-render's text API; glyph-position (¼ px) and HarfBuzz shaping equality not run; the F01 zh-CN corpus strings are dev-measure's drafts because dev-ir's strings table did not exist when the corpus was frozen.
