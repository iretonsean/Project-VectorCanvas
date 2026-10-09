# TOLERANCE.md — golden-image parity bar (A7)

Written **before any golden comparison was computed**. Author: dev-render. Date: 2026-10-09. The numbers below are a-priori engineering decisions; none is derived from a measurement of these renderers, and none is taken from a sourced legibility study (G6 §5 notes that no sourced absolute FLIP threshold exists in this program). They are not edited after results exist.

## 1. What is compared
Reference (golden) = **Tier 0 (c): `vello_cpu` 0.3.0** RGBA8 render of the runtime's display list (`render::render_cpu_png`), host `container-xeon4`.
Tests, each against that reference:

| path id | what renders | host label (COMMON §5) |
|---|---|---|
| `gl-llvmpipe` | same display list replayed on `vello_gpu` 0.3.0 → wgpu 30.0.1, GL backend only, `Limits::downlevel_webgl2_defaults()`, offscreen texture + readback | `host=container-xeon4 / gl-llvmpipe` |
| `web-webgl2` | same runtime compiled to wasm32, `vello_gpu` `webgl` feature, WebGL2 canvas in Chromium 141.0.7390.37 headless with `navigator.gpu` hidden; pixels read back from the canvas (readPixels / toDataURL of the WebGL canvas, **not** a page screenshot) | `host=container-xeon4 / chromium141-webgl2-swiftshader` |

Both use the same scene: the display list is built once by the runtime from the same signed IR, same fonts, same layout (D3). Display-list identity is checked (hash of the textual op dump) and reported next to each state; a mismatch is a test-setup failure, not a rendering result.

## 2. States compared (all at 1366×768, device scale 1.0, en unless stated)
1. **L01 idle**: signed `L01.pvca`, first frame, no focus.
2. **F01 empty**: signed `F01.pvca`, first frame, no focus.
3. **F01 with an error shown**: F01 `en`; click the email field, `text("not-an-email")`. Error text visible, field focused (focus ring and caret drawn).
4. **F01 zh-CN with IME preedit**: F01 `zh-CN`; click name, `ime_commit("张伟")`; click email, `ime_preedit("ni hao", Some((6,6)))`. Committed CJK text, Latin pinyin preedit with underline, caret, focus ring. The caret is drawn without blink (a fixed state).

## 3. Metric
**LDR-FLIP** (NVIDIA FLIP, `flip-evaluator` 1.7, pip, in `/workspace/dev_work/dev-render/.venv`), `dynamic_range="LDR"`, inputs sRGB, **library default 67 pixels-per-degree** (not tuned). Images are fully opaque (opaque page background), so RGB is compared directly.
Per state and per path we report, from the FLIP error map (float, no colour map):
- `mean` = mean FLIP error,
- `p99.9` = 99.9th percentile of the per-pixel error,
- `max` = maximum per-pixel error,
- plus informational: number of pixels not bit-identical, and max per-channel absolute difference (0–255).

## 4. Bound (pass requires all three, for every state, for each path)
| statistic | bound |
|---|---|
| mean FLIP | **≤ 0.005** |
| p99.9 FLIP | **≤ 0.05** |
| max FLIP | **≤ 0.25** |

The same bound applies to both paths.

## 5. Rationale
- Both renderers are the same Vello family: same `vello_common` front end, same glyph-outline preparation (`glifo`, outlines drawn as paths, glyph atlas cache off), same display list. What can differ is the last-bit arithmetic of rasterisation/compositing (CPU SIMD vs GL fragment precision; ANGLE/SwiftShader on the web path; G4 §6.4 states the guarantee is *parity within tolerance*, not bit-identity). That should produce sparse anti-aliasing differences at edges, so the **mean** is tiny and the **tail** is small.
- A mean ≤ 0.005 over an image that is largely flat background says "no systematic difference". The p99.9 bound (0.05) allows rare edge pixels with a coverage difference; the max bound (0.25) catches local defects that a mean hides: a missing or shifted glyph, a wrong clip or blend, a missing focus ring. All three must hold, because each guards against a different failure.
- The bound is deliberately **tight and uniform**. It is not widened for the web path: if SwiftShader-via-ANGLE does not meet it, that is a finding to report, not a reason to relax it.
- **What this does not show:** FLIP is a rendered-image difference metric, not a legibility measure (G7 S.6, G6 §5). Passing says the paths agree with Tier 0 on these four states on this host; it says nothing about how text looks to a person, nothing about real GPUs, and nothing about other states.
- Known limits: llvmpipe and SwiftShader are CPU rasterisers on a Xeon, not a Tier 1 GPU. A real Intel/Mali/Adreno GPU may differ more. Not run here.

## 6. On failure
1. The state/path is recorded as **FAIL** in the results table with all four statistics, the diff image (FLIP magma map, plus an amplified absolute-difference image) and both renders.
2. This file is **not edited**. If a different bound is argued for, it goes in a separate, dated "post-hoc" note in the REPORT, labelled as post-hoc, and the original verdict stays.
3. If the display-list identity check fails, the comparison is voided and redone after the cause is fixed; the void run is kept.

## 7. Procedure fixed in advance
- Script: `/workspace/dev_work/dev-render/tools/golden_compare.py` (created before the run, exercised only on synthetic images until the comparison is released).
- Outputs: `/workspace/dev_work/dev-render/measurements/golden/` — `results.md`, `results.json`, `<state>.<path>.png`, `<state>.<path>.flip.png`, `<state>.<path>.absdiff_x8.png`.
- Environment recorded with the results: `cat /proc/loadavg`, commit/hash of the runtime sources, tool versions.
