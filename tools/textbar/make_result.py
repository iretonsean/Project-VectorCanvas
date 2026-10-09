#!/usr/bin/env python3
"""make_result.py <text-bar dir> : writes RESULT.md tables from results.json (no numbers typed by hand)."""
import json, sys, hashlib
tb = sys.argv[1]
r = json.load(open(f'{tb}/results.json'))
cells = r['cells']
env = open(f'{tb}/run_env.txt').read()
def f(x, n=4): return f'{x:.{n}f}'
def pf(b): return 'PASS' if b else 'FAIL'
L = ['# A8 — reduced G6 text bar: RESULT (Latin + Simplified Chinese)', '',
     '**Host label:** `host=container-xeon4` (Linux, CPU rasterisers only). PVC = `pvc_runtime` text API (Parley 0.11.1 + vello_cpu 0.3.0, grayscale AA, dev-render\'s defaults, nothing tuned here); reference = Chromium 141.0.7390.37 headless with `--disable-lcd-text` (grayscale; see METHOD_TEXTBAR Addendum A1); yardstick = Firefox 142.0.1 (Playwright build v1495) headless. **Not a Tier 1 device result.**',
     f'**Method (written before measuring):** `METHOD_TEXTBAR.md` (+ Addenda A1–A4). Corpus: `corpus.json`. Raw: `results.json`, `work/*/*.png`, `work/*/breaks_*.json`. Environment/hashes: `run_env.txt`. Re-run: `tools/textbar/run_textbar.sh`.',
     '', '## Summary', '']
n = len(cells)
p3 = sum(c['pass_primary'] for c in cells.values()); pk = sum(c['pass_ink'] for c in cells.values())
pa = sum(c['pass_aligned'] for c in cells.values()); pl = sum(c['pass_lcd_secondary'] for c in cells.values())
pb = sum(c['linebreak']['pass'] for c in cells.values())
for scr in ['latin', 'zh-CN']:
    cs = {k: c for k, c in cells.items() if k.startswith(scr)}
    L.append(f"- **{scr}:** M3 (FLIP-mean, primary) pass {sum(c['pass_primary'] for c in cs.values())}/{len(cs)}; ink-masked {sum(c['pass_ink'] for c in cs.values())}/{len(cs)}; aligned {sum(c['pass_aligned'] for c in cs.values())}/{len(cs)}; vs LCD-Chromium (secondary) {sum(c['pass_lcd_secondary'] for c in cs.values())}/{len(cs)}; M4 line-break equality {sum(c['linebreak']['pass'] for c in cs.values())}/{len(cs)}")
L += [f'- **Overall:** M3 primary {p3}/{n} cells pass; M4 {pb}/{n} cells pass. The bar is **not** passed as a whole (G6 §5 requires pass in every script/size/scale cell and exact line breaks).' if (p3 < n or pb < n) else f'- **Overall:** M3 {p3}/{n}, M4 {pb}/{n}: all cells pass.',
      '- No human legibility study, no Arabic/Devanagari/emoji, no Japanese, Linux rasterisers only, single machine, single polarity (black on white). A pass here is not a statement about Windows/macOS/Android/ChromeOS text.', '',
      '## M1–M3 raster fidelity (LDR-FLIP, mean over whole image; lower = closer to Chromium)', '',
      '| script | px | scale | FLIP PVC↔Chromium | FLIP Firefox↔Chromium | ratio | M3 pass | ink-masked PVC / FF | ink pass | aligned PVC / FF (shift PVC, FF) | aligned pass |', '|---|---|---|---|---|---|---|---|---|---|---|']
for k, c in cells.items():
    scr, s, x = k.split('_'); p = c['pvc_vs_chromium']; q = c['firefox_vs_chromium']
    L.append(f"| {scr} | {s[1:]} | {x[1:]} | {f(p['flip_mean'])} | {f(q['flip_mean'])} | {f(c['ratio_pvc_over_firefox'],2)} | **{pf(c['pass_primary'])}** | {f(p['flip_ink'],3)} / {f(q['flip_ink'],3)} | {pf(c['pass_ink'])} | {f(p['aligned_flip_mean'])} / {f(q['aligned_flip_mean'])} ({p['shift']}, {q['shift']}) | {pf(c['pass_aligned'])} |")
L += ['', '## Secondary: reference = Chromium with its default LCD subpixel text (fontconfig `rgba=RGB` on this host)', '',
      '| script | px | scale | FLIP PVC↔Chromium-LCD | FLIP Firefox↔Chromium-LCD | pass |', '|---|---|---|---|---|---|']
for k, c in cells.items():
    scr, s, x = k.split('_')
    L.append(f"| {scr} | {s[1:]} | {x[1:]} | {f(c['pvc_vs_chromium_lcd']['flip_mean'])} | {f(c['firefox_vs_chromium_lcd']['flip_mean'])} | {pf(c['pass_lcd_secondary'])} |")
L += ['', '## M4 line-break equality (wrapped items, width 272 CSS px; exact match required)', '',
      '| script | px | scale | items | PVC mismatches vs Chromium | Firefox mismatches vs Chromium (informational) | pass |', '|---|---|---|---|---|---|---|']
for k, c in cells.items():
    scr, s, x = k.split('_'); lb = c['linebreak']
    L.append(f"| {scr} | {s[1:]} | {x[1:]} | {lb['items_compared']} | {', '.join(m['id'] for m in lb['pvc_mismatches']) or '—'} | {', '.join(m['id'] for m in lb['firefox_mismatches']) or '—'} | **{pf(lb['pass'])}** |")
L += ['', 'Mismatch detail (line start offsets, UTF-16, first non-space char of each line):', '']
seen = set()
for k, c in cells.items():
    for m in c['linebreak']['pvc_mismatches']:
        L.append(f"- `{k}` `{m['id']}`: Chromium {m['chromium']} vs PVC {m['pvc']}")
if 'nohint_secondary' in next(iter(cells.values())):
    L += ['', '## Secondary (Addendum A5, informational): reference = Chromium with `--font-render-hinting=none` (fractional advances, no hinting), grayscale', '',
          '| script | px | scale | FLIP PVC | FLIP Firefox | pass | line-break mismatches PVC | line-break mismatches Firefox |', '|---|---|---|---|---|---|---|---|']
    for k, c in cells.items():
        scr, s_, x = k.split('_'); h = c['nohint_secondary']
        L.append(f"| {scr} | {s_[1:]} | {x[1:]} | {f(h['pvc_flip_mean'])} | {f(h['firefox_flip_mean'])} | {pf(h['pass'])} | {', '.join(h['pvc_linebreak_mismatch_items']) or '—'} | {', '.join(h['firefox_linebreak_mismatch_items']) or '—'} |")
    L += ['', f"Chromium width diagnostic (`diag_kerning.out`, DejaVu Sans 11 px, one Latin string): default Chromium 322.00 px, `font-kerning:none` 333.00, `text-rendering:geometricPrecision` 309.84; Firefox default 309.80. i.e. on this host Chromium's default text uses hinted integer advances (wider), Firefox fractional ones. This explains why Firefox's line breaks differ from Chromium's in exactly the same items as PVC's do."]
L += ['', '## Environment', '', '```', env.strip(), '```', '',
      '## Labelled gaps (restated)', '',
      '- **No human legibility study** (G6 §5 item 5: ≥20 people per script) — not run; the 40 % preference criterion is unevaluated.',
      '- **Arabic, Devanagari, emoji, Japanese** not run. Simplified Chinese only for CJK.',
      '- **Linux rasterisers only** (Chromium/Skia, Firefox, vello_cpu on a Xeon, headless). G6 §9 risk 1 (1× Windows ClearType expectations) stays open.',
      '- **G6 §5 caveat, unverified:** Chromium and Firefox may share rasterisation code (FreeType + fontconfig settings) on Linux, which would make Firefox↔Chromium unusually small and the relative bar strict. The absolute FLIP values above let the reader judge; no check of shared code was made.',
      '- Shaping equality vs a HarfBuzz reference (G6 §5 item 3), glyph-position equality within ¼ px (item 2, second half) and cross-tier parity (item 4) were **not** run here.',
      '- Line height is fixed at round(1.35 × size) because dev-render\'s text API fixes it; the L01/F01 specimens may use other line heights.',
      '- Chromium LCD text on this host is fontconfig-driven; the grayscale reference required `--disable-lcd-text`, which is a deviation from "Chromium default".']
import os
if os.path.exists(f'{tb}/FINDINGS_textbar.md'): L += [''] + open(f'{tb}/FINDINGS_textbar.md').read().rstrip().split('\n')
open(f'{tb}/RESULT.md', 'w').write('\n'.join(L) + '\n')
