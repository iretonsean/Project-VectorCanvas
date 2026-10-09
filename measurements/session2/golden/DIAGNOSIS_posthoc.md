# POST-HOC diagnosis of the gl-llvmpipe vs Tier 0 result (written after results.md; the verdicts in results.md are NOT changed)

Verdict under the pre-registered bound (TOLERANCE.md sha256 0e6f1c8a…f6f7, committed before this run): **L01 idle PASS; F01 empty / error / zh-preedit FAIL on one statistic, p99.9 FLIP** (0.0529, 0.0533, 0.0526 vs bound 0.05). Mean (≤0.0005) and max (≤0.056) are far inside their bounds.

What the pixels show (checked, script in this directory's history: numpy over `F01_empty.*.png`):
- Every differing pixel differs by exactly **1/255** in at least one channel (max per-channel difference is 1 for the three F01 states, 2 for L01). 7129 of 1,049,088 pixels (0.68 %) differ in F01_empty.
- 2955 of those pixels have Tier 0 colour (159,201,197): the **disabled "Sign up" button** (accent fill drawn in a 0.4-opacity layer). Another ~1000 are 1-px field/checkbox border strokes (colours (225,228,233), (165,175,188), (192,198,207)). So the differences are flat-area/edge **8-bit rounding differences in layer compositing and stroke blending** between the CPU u8 pipeline and the GL fragment path, not geometry, glyph or clip errors. (Inferred cause; the pixel counts are checked.)
- Tier 0 and gl-llvmpipe renders are each bit-reproducible: re-running the whole generator gave byte-identical PNGs for all 8 files (`determinism_rerun.txt`).

Reading: the bound was set tight on purpose; a 1-LSB difference over a flat light-coloured region is weighted by FLIP at ≈0.05, just over the p99.9 line. This is a bound miss reported as a FAIL. If a different p99.9 bound were argued for, it is a post-hoc proposal (e.g. 0.06) and does not change this verdict.

## Web path (added after the web run; same post-hoc status)
The web-webgl2 results show the same pattern: the same 2955 disabled-button pixels (159,201,197) and the same border-stroke colours differ by 1/255 from Tier 0 (F01_empty: 7044 differing pixels); mean ≤ 0.0004, max ≤ 0.0555, p99.9 0.0527–0.0533 (FAIL against the 0.05 bound), L01 PASS. gl-llvmpipe and web-webgl2 differ from each other in only 168 pixels (max 1/255) on F01_empty: both GPU-pipeline paths (Mesa llvmpipe via wgpu-GL, ANGLE/SwiftShader via WebGL2) agree with each other and differ from the CPU u8 pipeline in the same places. This points at the CPU/GPU compositing-rounding difference, not at a path-specific bug (inferred).
Display-list identity check (TOLERANCE §1): FNV-1a fingerprints of the display list are identical on native and in wasm32 for all four states (`inputs/fingerprints.native.txt` vs `inputs_web/fingerprints.web.json`), so the web run replayed the same scene.
