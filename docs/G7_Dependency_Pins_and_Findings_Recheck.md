# G7 — Dependency pins and re-check of §9 findings

**Gate:** G7 in §8 of `PVC_Adversarial_Analysis_and_Architecture.md` ("re-check §9 findings at build start and pin exact crate versions").
**Status:** Closed on paper, 2026-10-08. **Expires:** the day the build session starts. At that point it must be re-run, because every pinned crate is pre-1.0 or moves quickly.
**Reference convention:** `§n.n` with a decimal (e.g. §4.7, §9.6) refers to the main analysis document; "section n" refers to this document; `G4 §n` etc. to other gate documents.
**Method:** each version below was read from the public crates.io registry API on 2026-10-08, and each upstream claim from the project's own README fetched that day. Nothing here is from memory. Where a statement is a judgment rather than a lookup, it is labelled *decision* or *estimate*.
**Scope guard:** this document names versions. It does not set up a build, a manifest, or a dependency tree. The pins are a constraint that the build session inherits, not a build plan.

---

## 1. Why this gate exists

The architecture leans on five external projects: wgpu (GPU abstraction), Vello CPU/GPU (raster backends), Parley (text layout) with its stack, AccessKit (accessibility), and winit (windowing). All five are 0.x or release breaking majors several times a year. The gate is not just "write down numbers". It checks three things: (a) the facts §9 relied on still hold, (b) the stack can be pinned to one *mutually consistent* set, and (c) any finding that changed is fed back into the analysis.

All three produced findings. S.1 and S.2 correct the main analysis, and S.4 corrects the README (section 4 below).

## 2. Pin table (as of 2026-10-08)

"Stack-consistent" means the version the *other* pinned crates actually require. That is not always the newest one published. The newest is shown for comparison.

| Component | Pin (stack-consistent) | Newest published | Released | MSRV (registry) | Role in PVC | Note |
|---|---|---|---|---|---|---|
| `wgpu` | **30.0.1** | 30.0.1 | 2026-08-22 | 1.87 | Tier 1/2 GPU abstraction | `vello_gpu` 0.3.0 and `vello` 0.11.0 both require `^30.0.0`, so consistent. Majors ship about quarterly (29.x in March–July, 30.0.0 on 2026-07-01) |
| `vello_cpu` | **0.3.0** | 0.3.0 | 2026-10-02 | 1.89 | Tier 0 raster backend candidate | Upstream calls it the most mature Vello renderer |
| `vello_gpu` | **0.3.0** | 0.3.0 | 2026-10-02 | 1.89 | Tier 1 raster backend candidate (no compute; native WebGL2 backend) | Published as 0.1.0 on 2026-08-26, then 0.3.0. There was no 0.2.0, so the version jumped to match `vello_cpu` |
| `vello` (compute) | not pinned | 0.11.0 | 2026-10-02 | — | Tier 2 research path only | Upstream marks it experimental; §4.3 already keeps it out of the shipping dependency set |
| `glifo` (Vello text/glyph runs) | **0.4.0** | 0.4.0 | 2026-10-02 | — | Glyph-run support shared by Vello CPU/GPU | Requires `skrifa ^0.44.0` |
| `parley` | **0.11.1** | 0.11.1 | 2026-08-16 | 1.88 | Text layout, selection/editing utilities | Requires `fontique ^0.11.1`, `harfrust ^0.12.0`, `skrifa ^0.44.0`, `icu_segmenter`/`icu_properties ^2.1.2` |
| `fontique` | **0.11.1** | 0.11.1 | 2026-08-16 | 1.88 | Font enumeration and fallback | |
| `harfrust` | **0.12.0** | 0.14.0 | 2026-07-03 | 1.85 | Shaping | **Do not take the newest.** For a 0.x crate, Parley's `^0.12.0` means 0.12.x only |
| `skrifa` | **0.44.0** | 0.48.0 | 2026-07-04 | — | Font reading, hinted outlines | Parley, `glifo`, and `vello` all require `^0.44.0`. Taking 0.48 would put two skrifa versions in one graph |
| `icu_segmenter` | **2.3.0** (within `^2.1.2`) | 2.3.0 | 2026-08-13 | — | Line/word/grapheme segmentation | ICU4X is ≥1.0, so a normal semver range applies |
| `accesskit` | **0.25.1** | 0.25.1 | 2026-09-25 | 1.87 | Accessibility schema | See the skew note below |
| `accesskit_winit` | **0.34.1** | 0.34.1 | 2026-09-25 | 1.87 | Desktop a11y adapter via winit | Requires `accesskit ^0.25.1`, `winit ^0.30.5` |
| `winit` | **0.30.13** | 0.31.0-beta.3 (pre-release) | 2026-03-02 | 1.70 | Desktop/Android windowing | Do not take 0.31 beta. AccessKit's winit adapter targets 0.30 |

**Toolchain floor (decision):** **Rust 1.89**, the maximum MSRV across the pinned set (Vello CPU/GPU). The README's "MSRV 1.75+" is out of date by fourteen minor versions and has been corrected in G1. The wgpu repository's own tests and examples need a newer toolchain (1.95 per its README) than the `wgpu` crate does (1.87). That matters only to contributors who run wgpu's tests, not to PVC.

**Version-skew finding (decision required at build start):** Parley's *optional* AccessKit integration targets `accesskit ^0.24.0`. The current `accesskit_winit` requires `^0.25.1`. Turning on Parley's AccessKit feature would therefore put two incompatible AccessKit versions in the graph. *Decision:* PVC does not use Parley's AccessKit feature. G4 maps IR semantic roles to AccessKit directly (G4 §8), so the integration is not needed. Text-field accessibility is wired in PVC's own semantics bridge. Re-check at build start in case Parley has moved to 0.25.

**Pinning policy (decision, conceptual only):**
1. Pins are exact for every 0.x crate, and the pin set is recorded with its date.
2. The pin set moves only as a unit, at a milestone, with the cross-tier golden-image suite (§4.4.1 rule 1) re-run. No drive-by upgrades.
3. A pin upgrade that changes rasterisation output (Vello, Skrifa hinting, HarfRust shaping) counts as a *determinism event*. It needs a runtime minor-version bump under the G4 §11 versioning rules, because golden images will change.
4. The build session re-runs this gate on day one. If anything has moved, this table is re-issued, not patched.

## 3. Re-check of §9 findings

| § | Finding | Re-check result 2026-10-08 | Action |
|---|---|---|---|
| 9.1 | WebGPU browser status | Not re-fetched in this pass; the source revision is 2 Oct 2026, six days old | Re-check at build start |
| 9.2 | wgpu downlevel limits, GL "best effort" | **Confirmed** from the wgpu README: OpenGL support is GL 3.3+ (desktop), GL ES 3.0+ and WebGL2, all marked "Downlevel/Best Effort". ANGLE is needed on macOS/iOS. On Windows the GL backend uses WGL by default, with ANGLE behind a build configuration | Strengthens §4.4's Windows note; no change |
| 9.3 | Hardware share and platform policy | Not re-fetched | Re-check at build start |
| 9.4 | Vello's three renderers | **Confirmed** from the Vello README: `vello_cpu` is "the most mature choice" for software rendering; `vello_gpu` "supports a native WebGL2 backend and a wgpu backend without requiring compute shaders" and is intended to become the primary production GPU renderer; the compute `vello` crate "remains an experimental implementation". MSRV 1.89 confirmed. **Refinement:** `vello_hybrid` had already published 0.1.0 (2026-07-29) and 0.2.0 (2026-08-07) before the rename. `vello_gpu` 0.1.0 (2026-08-26) is the renamed line, now at 0.3.0 | §9.4 wording is accurate. Add the version detail to avoid confusion when pinning |
| 9.5 | CPU-only inference speeds | Not re-fetched | Re-check at build start |
| 9.6 | AccessKit adapters | **Partly wrong.** The AccessKit README lists *released* adapters for Android, iOS, macOS, Unix (AT-SPI) and Windows (UIA), at "rough feature parity". The **web adapter** ("for applications that render their own UI elements to a canvas") is listed under **"Planned adapters"**, not shipped. The README also says adapters "don't yet support rich text or hypertext". Single- and multi-line text input *are* supported | **Correction to §2.5, §9.6 and the supervisor findings file**. See S.1 |
| 9.7 | Parley stack | **Confirmed** from the Parley README: Fontique, HarfRust, Skrifa, ICU4X, MSRV 1.88. **Addition:** Parley "includes utilities for text selection and editing", and Parley can be run against the Web Platform Tests text-layout suite via Blitz's runner | Feeds G6 |
| 9.8 | EAA, EU AI Act | EAA not re-fetched. **AI Act timeline has changed** and is no longer "under discussion". See S.2 | **Correction to §9.8** |
| 9.9 | Prior art | Flutter-web parallel-DOM semantics confirmed as prior art for the S.1 gap. See S.3 | Add to §9.9 |

## 4. Supplementary verified findings (S.n)

These are new facts established by this program. The other gate documents cite them as **[G7 S.n]**. Local copies of the fetched upstream READMEs and docs pages are kept in the PM scratch directory `/workspace/orchestrator_work/pm/`, outside the repo, as evidence for the red-team. Each follows the §9 format: fact → source → consequence.

### S.1 AccessKit has no shipped web adapter
- **Fact:** the AccessKit README lists the web adapter "for applications that render their own UI elements to a canvas" under "Planned adapters". Released adapters are Android, iOS, macOS, Unix and Windows. Adapters "don't yet support rich text or hypertext". Single- and multi-line text input controls are supported. The registry shows `accesskit_android` 0.9.0, `accesskit_ios` 0.2.1, `accesskit_macos` 0.27.1, `accesskit_unix` 0.24.0 and `accesskit_windows` 0.35.1, all published 2026-09-25, and no web adapter crate.
- **Source:** github.com/AccessKit/accesskit README (fetched 2026-10-08); crates.io registry search for "accesskit" (2026-10-08).
- **Consequence:** the web target (Tier 1 WebGL2 and Tier 2 WebGPU in a browser) has **no off-the-shelf accessibility bridge**. The analysis said this bridge existed, which overstated it. PVC must build a web semantics mirror: a hidden, positioned DOM/ARIA tree generated from the same IR semantics. Prior art exists (S.3), so this is *engineering with precedent*, not research. It is still unbudgeted work on a release-blocking path (§4.7 item 4). The iOS adapter is at 0.2.x, which is young for a launch target (Q3). *Estimate:* expect gaps there too.

### S.2 EU AI Act timeline is now law, not a proposal
- **Fact:** the Digital Omnibus on AI, Regulation (EU) 2026/1744, was published in the Official Journal on 24 July 2026 and entered into force on 27 July 2026. It defers stand-alone (Annex III) high-risk obligations to 2 December 2027 and product-embedded (Annex I) ones to 2 August 2028. Article 50 transparency duties (telling people when they interact with an AI system; labelling AI-generated content) stayed on 2 August 2026. Systems already on the market get until 2 December 2026 for the Article 50(2) machine-readable marking. GPAI provider duties have applied since 2 August 2025.
- **Source:** Council of the EU press release, 29 June 2026; Cloud Security Alliance research note on Regulation (EU) 2026/1744; Cooley, "Digital AI Omnibus Delays Key Deadlines" (all retrieved 2026-10-08).
- **Consequence:** §9.8 must stop calling the delay "under discussion". The supervisor findings file's line "EU AI Act high-risk obligations began Aug 2025" is wrong: that date applies to GPAI provider duties. For PVC, the relevant AI Act surface is **Article 50 transparency**, not high-risk duties, because PVC's interpreter is a developer tool and its runtime contains no AI (hard wall). G5 §10 documents this. *Not legal advice.*

### S.3 Prior art for accessible canvas-rendered web UIs
- **Fact:** Flutter's web engine renders to a canvas and exposes accessibility by translating its semantics tree into an accessible HTML DOM structure. Semantics nodes are absolutely positioned DOM elements carrying ARIA roles and labels, laid over the canvas. Flutter also places a real `<input>`/`<textarea>` pixel-aligned over canvas text fields, which keeps browser conveniences such as autofill working. Flutter web accessibility is off by default for performance and is enabled through a hidden "Enable accessibility" button. Repositioning on scroll is described as potentially expensive.
- **Source:** docs.flutter.dev/ui/accessibility/web-accessibility (updated 2026-09-29); Flutter blog, "Accessibility in Flutter on the Web".
- **Consequence:** the S.1 gap has a proven design pattern. PVC adopts the pattern but **rejects the off-by-default choice**: §4.7 item 4 makes accessibility release-blocking, so the web mirror is on by default and its cost is budgeted (G4 §8.3). Flutter's hidden text-input overlay is also the precedent for web IME (G6 §6).

### S.4 The README's token figures are wrong by about 3×
- **Fact:** measured with OpenAI's `tiktoken` 0.14.0 on the exact text of the two README code blocks (2026-10-08). The "Legacy Way" HTML block, labelled "~600 tokens", is **185 tokens** under both `o200k_base` and `cl100k_base`. It is 638 characters long, which suggests characters were mistaken for tokens. The "PVC Conversational Paradigm" block, labelled "~65 tokens", is **107 tokens** under both encodings. The measured reduction is **42%**, not the implied ~89% or the headline "up to 90%". The two blocks also do not describe the same UI: the PVC prose says "Designer" where the HTML says "Product Designer", and it replaces the body sentence with an instruction to write one ("explaining our spatial computing goals"). The PVC version is under-specified relative to the baseline it is compared with.
- **Source:** direct measurement; method and text recorded in G2 §3.4.
- **Consequence:** the 42% figure is a single cherry-picked specimen and proves nothing either way. It sits inside §1.4's 30–70% hypothesis band. The README now cites the measurement instead of the claim (G1), and G2's fidelity checklist exists because of the "Designer" problem.

### S.5 Tokenizer facts
- **Fact:** `tiktoken` (OpenAI's BPE tokenizer library, current release 0.14.0 on PyPI) provides `o200k_base`, and its README shows `encoding_for_model("gpt-4o")` resolving to it. `cl100k_base` is also provided.
- **Source:** github.com/openai/tiktoken README; pypi.org/project/tiktoken (2026-10-08).
- **Consequence:** feeds G2 §3.2. *Unverified, not looked up in this pass:* which tokenizers non-OpenAI vendors use, and whether those are public. G2 treats them as unknown and measures billed usage instead.

### S.6 FLIP image-difference metric
- **Fact:** NVIDIA's FLIP (LDR-FLIP and HDR-FLIP) is an image error metric with a reference tool at v1.7. It is BSD-3-Clause licensed and pip-installable as `flip-evaluator` (1.7 on PyPI).
- **Source:** github.com/NVlabs/flip README; pypi.org/project/flip-evaluator (2026-10-08).
- **Consequence:** G6 uses LDR-FLIP as the named perceptual metric for the text-parity suite (§6 item 4), with the caveats G6 §5 lists. FLIP was designed for rendered-image comparison, not for text legibility as such.

### S.7 AccessKit 0.25.1 role and action vocabulary
- **Fact:** the `accesskit` 0.25.1 `Role` enum has 182 variants. They include `Button`, `CheckBox`, `Switch`, `RadioButton`, `RadioGroup`, `Slider`, `SpinButton`, `TextInput`, `MultilineTextInput`, `PasswordInput`, `SearchInput`, `EmailInput`, `NumberInput`, `ComboBox`, `ListBox`, `ListBoxOption`, `List`, `ListItem`, `Grid`, `GridCell`, `Row`, `ColumnHeader`, `Table`, `Cell`, `Tab`, `TabList`, `TabPanel`, `Dialog`, `AlertDialog`, `Alert`, `Status`, `Heading`, `Image`, `Link`, `Label`, `Paragraph`, `Navigation`, `Main`, `Region`, `Group`, `GenericContainer`, `ProgressIndicator`, `Meter`, `Menu`, `MenuItem`, `Toolbar`, `Tooltip`, `ScrollView`, `Tree`, `TreeItem`, `Form`, `Canvas`, `Video`, `Audio`. The `Action` enum includes `Click`, `Focus`, `Blur`, `Expand`, `Collapse`, `Increment`, `Decrement`, `SetValue`, `ReplaceSelectedText`, `SetTextSelection`, `ScrollIntoView`, `ScrollUp`/`Down`/`Left`/`Right`, `SetScrollOffset`, `ShowContextMenu`, `ShowTooltip`/`HideTooltip`, `CustomAction`.
- **Source:** docs.rs, `accesskit` 0.25.1, `enum.Role` and `enum.Action` pages (fetched 2026-10-08).
- **Consequence:** the G4 §8.1 role-mapping table uses these exact names, so every mapping can be checked.

## 5. Residual risks of this gate

- **Shelf life.** Every pin will be stale within weeks. The pin table is a snapshot that proves the stack *can* be made consistent today. It does not promise it will be next quarter.
- **Not re-fetched:** §9.1, 9.3, 9.5 and the EAA part of 9.8. They are six days old or less and are re-check items for build start.
- **Transitive graph not resolved.** Only direct requirements between the pinned crates were checked. Full resolution was not attempted, because it would mean writing a build manifest, which is out of scope. *Estimate:* other duplicate-version conflicts may surface when the build session resolves the graph.
- **Upstream claims taken at face value.** "Most mature", "intended to become primary" and "rough feature parity" are each project's self-description, not independent evaluations.
