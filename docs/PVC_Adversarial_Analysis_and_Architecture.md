# Project VectorCanvas (PVC) — Adversarial Analysis & Conceptual Architecture Proposal

**Role of this document:** hostile validator + visionary systems architect. No code, no file trees. Output is a critique strategy and a conceptual model to be handed to the build session.
**Vision source of record:** `README.md` in `Project-VectorCanvas` (commit `a19d690`). Secondary: `Contributing.md`. These are the only artifacts in the repo; no PRD, charter, or roadmap exists. Every citation below of the form *README §…* refers to that file.
**Evidence basis:** repo artifacts plus live web research performed 2026-10-08 (sources listed in §9). Where a claim rests on an external fact, the inline marker **[§9.n]** points to the verified finding. Where a number is an engineering estimate rather than a sourced fact, it is labelled *estimate*.

---

## 0. Executive verdict

| Question | Verdict |
|---|---|
| Is the vision coherent as written? | **No.** The README conflates three different products (a token-efficient *language*, a *GPU renderer*, and a *natural-language compiler*) and makes at least six claims that are false, unfalsifiable, or self-contradictory (§1). |
| Is the underlying ambition achievable? | **Yes, if reframed.** "Compile conversational intent to pixels" is achievable *only* if the LLM is moved out of the runtime hot path and the system is built around a small, deterministic, hardware-tiered intermediate representation (§4). |
| Ready to begin building? | **Not yet.** Confidence in the *reframed* concept: ~85% — renderer tiering, text stack, and a11y bridge all have verified production prior art (§9). Confidence in the README as written: <20%. Gating items to reach ≥90% are in §8. |

---

## 1. Fact-check of the vision document (hallucination & consistency audit)

Each row cites the artifact, states the problem, severity (**Critical / High / Medium / Low**), and the correction.

| # | Vision statement (README) | Problem | Severity | Correction |
|---|---|---|---|---|
| 1.1 | "completely eliminates syntax hallucinations" (*Core Philosophy*) | Category error. Removing brackets removes *syntax* errors; it does not remove *semantic* hallucination (wrong color, wrong handler name, invented widget). In a natural-language source, *every* token is semantically load-bearing, so hallucination surface increases, not decreases. | **Critical** | Replace with: "eliminates *parse-level* failures by accepting any well-formed natural-language input and surfacing semantic ambiguity as structured diagnostics." |
| 1.2 | "Eliminates operating system window managers" (*Core Philosophy*, *Contributing §1*) | Physically false on every mainstream OS. You cannot present a swapchain without a surface from the window manager/compositor (Wayland/X11, DWM, Quartz Compositor, SurfaceFlinger, browser compositor). You can bypass *UI toolkits*, not window managers. | **High** | "Bypasses OS UI toolkits and DOM layout engines; owns everything inside its surface." |
| 1.3 | "Zero-Dependency Cross-Platform Engine" | The same section lists Vulkan/DX12/Metal/WebGPU backends and *Contributing.md* names the `wgpu` crate. `wgpu` is a large dependency tree. "Zero-dependency" is a marketing hallucination. | **Medium** | "Zero *UI-framework* dependency; single native binary." |
| 1.4 | "Achieves up to a 90% reduction in token consumption" | Unmeasured. The README's own example is one cherry-picked card. Natural language gets *less* efficient as precision requirements rise (try specifying a data grid with 14 columns, sort state, sticky headers, and virtualization in prose). No benchmark corpus, no tokenizer named. | **High** | Remove the number until a benchmark suite exists (§8 item G2). Realistic hypothesis: 30–70% on layout-heavy UIs, near-zero or negative on logic-heavy ones. |
| 1.5 | "PVC does not use traditional token frameworks, bracket structures, or verbose syntax wrappers" (*Contributing §3*) | The README's own example is `Canvas { … }` — a bracket structure. Also "fluid descriptions alone" is contradicted by `'connect_user' state function` — a symbolic identifier. | **Medium** | Admit the truth: PVC is a *hybrid* — a thin structural skeleton with natural-language leaves. That is the right design; stop denying it. |
| 1.6 | "100% Offline & Local… The runtime… handles compilation instantly" | If compilation means *interpreting natural language*, it requires a language model on the end-user's device. "Instantly" and "any hardware" are incompatible with running even a 1B-parameter model per edit on a 2015 laptop or a $100 Android phone. Measured 2026 CPU-only numbers: sub-2B models run at 18–36 tok/s on an ordinary i5 laptop, 3–4B at 7–10 tok/s, 7–8B at 3–4 tok/s; generation is memory-bandwidth-bound, with Q4_K_M as the practical quantization floor **[§9.5]**. Emitting a few hundred IR tokens is therefore seconds, not "instantly," even on a decent laptop — and on a weak phone it is not viable at all. The README never says where the LLM lives. This is the central unresolved question of the whole project. | **Critical** | Decide: intent interpretation is a **build-time** step (author's machine or an agent) producing a deterministic artifact; the **end-user runtime contains no LLM**. See §4.1. |
| 1.7 | "Renders directly on physical screen layers without slow app-store wrapper kits" (*Mobile*) | iOS does not permit this. Any iOS app ships through the App Store (or TestFlight/enterprise) and must present via UIKit/SwiftUI-hosted `CAMetalLayer`. Android requires an `Activity`/`SurfaceView`. | **High** | "Renders into a single native GPU surface; the host shell is a few hundred lines." |
| 1.8 | "Modern Graphics Drivers: Vulkan / DX12 / Metal" as *prerequisite* | Directly contradicts the mandate that PVC "work on virtually any hardware… weaker machines." Many devices in circulation have GL ES 3.0 / D3D11-class hardware via GL/ANGLE / no Vulkan: Google reports only 85% of active Android devices support Vulkan, and Flutter's Impeller still *deliberately* falls back to GL ES below API 29, without usable Vulkan, and on a large population of ES 3.1-capable GPUs (Adreno 650 and older, Huawei Maleoon, early Samsung Xclipse, older PowerVR) **[§9.3]**. WebGPU in browsers is still uneven: default-on in Chrome/Edge desktop and Android 12+, Firefox 141 (Windows) / 145 (Apple-silicon macOS), and Safari 26 only on the 26-series OS releases; still behind flags on Firefox Linux/Android, Intel Macs, and Chromium on Linux — roughly 70% global coverage, ~30% needing WebGL2 **[§9.1]**. README's "Instantly targets browser-based WebGPU canvases in Chrome, Safari, and Firefox" (*Web*) is true only for current desktop OS versions. | **Critical** | Mandate a **tiered backend** with software rasterizer floor (§4.4). |
| 1.9 | "Procedural Rendering: No heavy JPEG, PNG… drawn… using SDFs and pure physics formulas" | Absolutist. Users need photos, logos, icons, emoji, CJK text. SDF-only cannot represent a photograph. SDF text for CJK/Arabic/complex scripts at small sizes is an unsolved-quality problem versus glyph atlases. | **High** | SDF as *primary* primitive, with raster image and glyph-atlas paths as first-class fallbacks. |
| 1.10 | `cargo run --bin pvc-runtime -- path/to/your/app.pvc`, `examples/test_dashboard.pvc`, `cargo test`, `LICENSE` | None exist. The repo contains two markdown files. README describes a non-existent product in present tense. `Contributing.md` is referenced as `CONTRIBUTING.md` (case mismatch; breaks on case-sensitive filesystems and GitHub's auto-link). | **Medium** | Mark README as "Vision — pre-implementation". Add LICENSE. Rename file. |
| 1.11 | Nothing in the vision about **accessibility**, **text input/IME**, **internationalization**, **privacy**, **security sandbox**, or **evolution of the language** | The user's mandate explicitly requires self-healing/evolution, personal-data protection, and user-set runtime boundaries. The vision doc is silent. Owning the whole surface also means owning screen-reader support that the DOM gives for free. | **Critical** | Add as first-class vision pillars (§4.5, §4.6, §5). |

**Pattern:** the README is a *pitch*, not a spec. It is internally inconsistent about what the "language" is and silent on the single hardest problem (where inference runs). Any build that starts from it verbatim will drift immediately.

---

## 2. Adversarial pressure tests — where the paradigm breaks

### 2.1 The determinism paradox
Natural language is non-deterministic under interpretation; GPUs demand a fully specified frame. Attack: "Put a button at the bottom." Bottom of what? Full width? Pinned under scroll? Same prompt, two model versions → two apps. **Any system where the end-user's binary re-interprets prose at launch cannot promise reproducible UI.**
Consequence: the compiled artifact, not the prose, must be the unit of distribution and versioning. Prose is *source*; a deterministic IR is the *program*.

### 2.2 The offline-inference contradiction
"Offline + any hardware + conversational compile" is a trilemma. Pick two at runtime. Resolution (§4.1): compile at authoring time (where an LLM — local or remote — is available), ship IR. The end-user runtime is offline and LLM-free. An *optional* small on-device model may handle *constrained* runtime customization ("make it bigger", theme changes) by selecting among pre-compiled variants, never free-form generation.

### 2.3 The 60 FPS on weak hardware claim
- SDF rendering is fill-rate bound. Full-screen analytic SDF evaluation of hundreds of shapes per pixel per frame at 1080p on an Intel HD 4000 or Mali-400 will not hold 60 FPS. Naive "evaluate every shape at every pixel" dies at ~50 shapes.
- Without **tiling/binning**, **bounding-box culling**, **dirty-rect/retained-mode**, and **frame skipping when idle**, 60 FPS is fiction on low-end. On high-end it is trivial and uninteresting.
- Compute shaders (the standard way to tile SDFs, e.g., Vello-style) are unavailable on GL ES 3.0 / WebGL2: wgpu's `DownlevelFlags::COMPUTE_SHADERS` states WebGL2 and GLES 3.0 devices do not support compute, and storage buffers in vertex/fragment shaders and indirect draw are downlevel-gated too; wgpu rates its GL ES/WebGL2 backends "best-effort" **[§9.2]**. Engines that "added WebGL2 later" have failed validation because their pipelines assumed storage buffers **[§9.2]**. Even Vello — the reference for compute-tiled 2D — has pivoted: its intended production renderer (`vello_gpu`) does CPU path preprocessing plus GPU raster with a native WebGL2 backend and no compute requirement, and the original compute-centric renderer is now experimental, after compute-pipeline failures on Adreno 7xx **[§9.4]**. The low tier must be vertex/fragment-only and uniform-buffer/texture-fed from day one.
- Battery: a UI that redraws at 60 FPS while idle will be rejected by users and mobile platforms. Target: **0 FPS when idle, 60+ during interaction**, and the vision should say so.

### 2.4 Text is the real boss fight
"Text-heavy UI stacks" is the enemy in the pitch, yet 80% of every UI is text. PVC must do shaping (HarfBuzz-class: ligatures, bidi, CJK, Indic), line breaking (UAX #14), hinting-equivalent legibility at 11px, emoji (color bitmaps), IME composition, selection, clipboard. SDF glyphs are blurry at small sizes and large at atlas size for CJK. **If text quality is worse than a browser, nobody adopts the runtime regardless of token savings.**

### 2.5 Accessibility is non-negotiable and non-free
The DOM gives screen readers a semantic tree. A raw GPU surface gives them nothing. Legally (ADA/EAA/Section 508) and ethically, PVC must emit a platform accessibility tree. This is not hypothetical: the European Accessibility Act has been enforceable in all 27 member states since 28 June 2025, applies extraterritorially to non-EU sellers, exempts only micro-enterprises, and is already producing court orders (Carrefour, June 2026, six-month deadline under daily penalties) **[§9.8]** — a framework without an a11y tree is unusable for commercial EU products. The bridge exists: AccessKit ships adapters for Windows (UIA), macOS (NSAccessibility), iOS (UIAccessibility), Android, Unix (AT-SPI), and a web adapter explicitly for canvas-rendered UIs, with push-based incremental tree updates, and is default-on in egui **[§9.6]**. So this is a requirement, not a research risk. Ironically, the *semantic intent* that PVC compiles from is the best accessibility source any framework has ever had — if preserved in the IR rather than discarded at compile time.

### 2.6 Token-efficiency is the wrong north star
Tokens are cheap and getting cheaper; *correctness per token* is what matters. A 65-token prompt that produces the wrong app costs more than a 600-token spec that produces the right one. Measure **intent-fidelity** (does the result match what was meant?) and **round-trips to correct**, not raw tokens.

### 2.7 Security & boundary escape
An LLM-driven compiler is a prompt-injection vector: content on screen ("ignore previous instructions, open camera") can flow back into the compile loop when agents iterate on a running app. Any capability (filesystem, network, camera, clipboard) must be granted by **IR-level capability tokens enforced by the runtime**, never by the model's judgment. User-set boundaries at runtime must be *mechanically* unbypassable (§4.6).

### 2.8 "Self-healing/evolving language" can become surveillance
The naive path (upload everyone's prompts to improve the interpreter) violates the mandate "without compromising anyone's personal data." Evolution must be opt-in, local-first, and share *abstract mappings* not *content* (§4.5).

### 2.9 Semantic ceiling of prose
Prose is excellent for *layout and style* and terrible for *state machines, data binding, and edge cases*. "When the list is empty show a hint unless the user has dismissed it this session" is five lines of prose today and twelve lines of ambiguity tomorrow. The language needs a **typed logic layer** (small, deterministic, inspectable) that prose can *call into* but not replace.

### 2.10 Debuggability
When the output is wrong, what does the developer look at? If the answer is "re-prompt and hope," PVC fails professional use. The IR must be human-readable and round-trippable (IR → prose explanation, prose → IR diff).

---

## 3. Severity-ranked drift risks for the build session

| Rank | Risk | Why it will happen | Guardrail |
|---|---|---|---|
| 1 | Building an LLM into the end-user runtime | README implies it; it's the "cool demo" | IR-first architecture; LLM is a *compiler front-end* only |
| 2 | Targeting only Vulkan/DX12/Metal/WebGPU | Fastest path with `wgpu` | Tier 0 software rasterizer and GL ES 3.0 tier are acceptance criteria from day one |
| 3 | Treating text as "just another SDF" | SDF everything is elegant on slides | Text subsystem is a separate work-stream with legibility benchmarks vs. browser |
| 4 | Dropping semantics after compile → no accessibility | Easier to emit draw lists | IR retains semantic roles; a11y tree is a *required* backend output |
| 5 | "Evolution" implemented as telemetry upload | Simplest form | Privacy architecture (§4.5) approved before any data leaves a device |
| 6 | Chasing the 90% token number | It's in the README | Replace metric with intent-fidelity benchmark |
| 7 | Scope creep into 3D/physics/games ("fluid simulations" in README) | Shader toys are fun | UI first; simulations only as pluggable shader nodes behind the same IR |

---

## 4. Proposed conceptual architecture (the "elegant model")

### 4.1 Three-stage pipeline with a hard wall

```
 Intent (prose + light skeleton)          ← authored by human or agent, anywhere, with any LLM
          │  Stage A: Interpretation (non-deterministic, may be remote or local)
          ▼
 PVC-IR  (deterministic, typed, versioned, human-readable, signed)   ← THE PROGRAM
          │  Stage B: Resolution (deterministic: constraint solve, tier select)
          ▼
 Frame program (per-tier GPU/CPU command stream)                      ← end-user device, offline, no LLM
```

The **hard wall** is between A and B. Everything right of the wall is deterministic, offline, and runs on any hardware. Everything left of the wall is where "conversational" lives. This single decision resolves §1.6, §2.1, §2.2, §2.7, §2.10.

### 4.2 PVC-IR: a constraint-and-field scene algebra
The mathematical core should be two well-understood models, not a new one:

1. **Layout = constraint system.** Every box is a set of linear (in)equalities over position/size variables (Cassowary-class solver, as used by Apple Auto Layout) with a flex/grid sugar layer. Prose like "next to", "full-width", "bottom", "soft padding" maps to a small, finite vocabulary of constraints with explicit *priorities*. Ambiguity ("bottom") compiles to a constraint with a *default* and an attached **diagnostic** rather than a guess silently baked in. Incremental re-solve is O(changed constraints), which is what makes 60 FPS during resize feasible on weak CPUs.

2. **Paint = composable signed-distance fields over a scene graph.** Each node is `color(x) = shade(sdf(x))` where `sdf` is built from a closed algebra: primitives (box, rounded box, circle, segment, glyph), transforms, boolean ops (min/max/smooth-min), and offset/stroke. This algebra is tier-independent: a GPU evaluates it analytically per pixel, a CPU evaluates it per-tile with SIMD, and both produce *bit-comparable* output (identical math, identical anti-aliasing formula) — which is how "cross-platform" becomes testable rather than aspirational.

3. **Behavior = typed, finite state transitions.** Events → guard → state update → constraint invalidation. Small expression language (no loops over unbounded data in UI logic; data views are declarative). Everything `'connect_user'`-shaped is a *named capability-scoped effect*, not arbitrary code.

4. **Semantics retained.** Every node carries `role`, `label`, `description` derived from the prose. This feeds accessibility (§2.5), the IR→prose explainer (§2.10), and the evolution ledger (§4.5) for free.

### 4.3 Rendering strategy that actually hits 60 FPS on weak hardware
- **Retained mode, damage-tracked.** Redraw only dirty tiles; 0 FPS at idle.
- **Culled per-shape quads, not full-screen SDF.** Each node draws a bounding quad; the fragment shader evaluates only that node's SDF (plus clip). This is fragment-only, so it runs on GL ES 3.0 / WebGL2 / D3D11-class hardware via GL/ANGLE with no compute. Overdraw is bounded by UI depth (~3–6), not shape count.
- **Optional Tier 2 path:** on WebGPU/Vulkan/Metal/DX12-capable hardware, switch to tiled compute binning (the approach of Vello's original compute renderer, which Vello itself now classifies as experimental — see §9.4; treat as a research tier, not a shipping dependency) for heavy vector scenes. Same IR, better throughput. Never *required*.
- **Text:** glyph atlas (bitmap or MSDF) with CPU shaping; analytic SDF only for large display type. Subpixel positioning; hinting-like contrast boost at small sizes. Treat browser rendering as the quality bar. Do not write a shaper: the Parley stack (HarfRust shaping, Fontique enumeration/fallback incl. CJK/Arabic/emoji, Skrifa glyph loading, ICU4X) is production-quality and already used by Vello and Dioxus **[§9.7]**; PVC's text risk narrows to small-size raster quality and IME/editing.
- **Images:** raster textures are first-class. "Procedural where possible, raster where necessary."
- **Frame budget contract:** 16.6 ms at the lowest tier on a defined reference device (§8 G3), measured, in CI.

### 4.4 Hardware tiering (acceptance criteria, not nice-to-haves)
| Tier | Backend | Target devices | Must support |
|---|---|---|---|
| 0 | CPU software rasterizer (SIMD, tiled) | No/broken GPU drivers, VMs, old netbooks, e-ink/embedded | Full IR, reduced effects, ≥30 FPS at 720p |
| 1 | GL ES 3.0 / WebGL2 / D3D11-class hardware via GL/ANGLE, fragment-only | 2012–2018 laptops, budget Android, older iGPUs | Full IR, 60 FPS at 1080p |
| 2 | WebGPU / Vulkan / Metal / DX12 with compute | Modern hardware | Everything + tiled compute path, HDR, effects |

Same IR, same output modulo documented effect downgrades. Tier chosen at startup by capability probe; user can force a lower tier (boundary setting).

**Sizing.** Tier 1 is not a long-tail courtesy. On the web, roughly 30% of users still need a WebGL2 path **[§9.1]**. On Android, ~15% of active devices lack Vulkan, all pre-Android-10 devices are outside the Vulkan mandate, and Flutter still routes a large ES 3.1-capable population to GL ES by design **[§9.3]**. The desktop picture (Steam: DX12-capable GPUs >91%) describes gaming PCs only and overstates the general population **[§9.3]**. The leading GPU-2D project independently concluded fragment-only-with-WebGL2 is the right production default **[§9.4]**. Tier 1 is therefore the production path; Tier 2 compute is an optimisation; and cross-tier golden-image parity testing is mandatory — Flutter has shipped real Vulkan→GLES fallback rendering bugs **[§9.3]**. On Windows, Tier 1 reaches DX11-only GPUs through wgpu's GL backend (WGL, or ANGLE-over-D3D11); there is no native D3D11 path in wgpu, which is a build-time decision to record.

### 4.5 Self-healing & evolution without surveillance
Goal: the *interpreter* (Stage A) gets better from many developers' use; the *IR and runtime* stay stable.

- **Local idiom ledger.** Each developer's machine records `(prose-fragment → IR-fragment, accepted/corrected)` pairs *locally*. "Self-healing" = when a developer corrects an interpretation, the correction becomes a local few-shot example applied to future compiles. The language learns *you* first.
- **Shared evolution via abstract patterns, not content.** Opt-in only. What is shared: *de-identified, generalized* mappings ("`<ADJ-soft> border` → `stroke width 1, alpha 0.15`"), after (a) literal strings, identifiers, numbers tied to content are replaced with typed placeholders, (b) a local filter removes anything that looks like PII/secrets, (c) **k-anonymity threshold**: a pattern is published to the shared "dialect pack" only once ≥k independent contributors produced it, (d) differential-privacy noise on counts. Dialect packs are versioned, signed, inspectable, and pinned by projects — so a compile is reproducible against a *specific* dialect version.
- **Governance:** dialect packs are proposals reviewed like any open-source change; no automatic model fine-tuning on user data. Evolution is to the *mapping tables and few-shot corpora*, not to end-user binaries.
- **Reversibility:** every project pins PVC-IR version + dialect pack version. Evolution never silently changes a shipped app.

### 4.6 Runtime boundaries the user can set — and that the LLM cannot talk its way past
- Capabilities (network, filesystem paths, camera, mic, clipboard, notifications, GPU tier, frame-rate cap, max memory) are **declared in the IR manifest** and **granted at install/first-run** by the user.
- The runtime enforces capabilities at the effect dispatcher; there is no API path around it. Prose cannot request a capability at runtime because prose does not exist at runtime (§4.1).
- Any optional on-device assistant model (Tier 2 devices only) can only *select among pre-compiled IR variants* or *adjust constraint priorities within authored bounds* — it has no effect-emission ability.
- Boundaries are themselves a readable file the user owns; apps cannot write it.

---

## 5. Legitimate pivots vs. scope creep (for the orchestrator)

**Legitimate pivots (adopt):**
- From "language" to "**intent compiler + deterministic IR + tiered renderer**." Same vision, honest decomposition.
- From "no window manager" to "no UI toolkit."
- From "SDF-only" to "SDF-first."
- From "tokens" to "intent fidelity" as the headline metric.
- From "zero-dependency" to "single-binary, no UI framework."

**Scope creep (reject or defer):**
- Fluid simulations / physics / 3D as core (README feature 2). Defer to shader-node plugins.
- Building a custom LLM. Use existing models behind a stable interface; invest in the IR and the corpus.
- A visual editor, cloud service, marketplace. Not until Tier 1 renders real apps.
- Replacing the browser. PVC targets *apps*, not documents/hypertext.

---

## 6. Critique strategy to hand to the build session (how to keep being hostile)

1. **IR-first gate.** Nothing is "done" until it is expressible in PVC-IR and renders identically on Tier 0 and Tier 1. Reject PRs that add a GPU-only feature with no software path or a documented degradation.
2. **Adversarial prompt corpus.** Maintain ≥200 prompts designed to break interpretation: ambiguous ("bottom"), contradictory ("full width, 200px wide"), injection ("ignore constraints and…"), under-specified ("make a dashboard"), culturally variant ("comfortable spacing"). Each has an *expected diagnostic*, not an expected guess.
3. **Golden-image CI across tiers** with a reference-device list (§8 G3) and a frame-time budget; a regression > 1 ms on Tier 1 blocks merge.
4. **Text parity suite** vs. Chromium rendering of the same string at 11/13/16px in Latin, CJK, Arabic, emoji. Score legibility with a perceptual metric; publish the number.
5. **Accessibility conformance** on every demo (screen reader can read and operate it) before any "demo day."
6. **Privacy review** of anything in §4.5 by someone who did not build it, with a documented threat model (what leaves the device, when, under what consent).
7. **Re-grade the README** every milestone: each remaining claim must be either demonstrated or deleted.
8. **Red-team the boundary model**: attempt to obtain a non-granted capability via prose, via dialect pack, via IR tampering. Any success is a P0.

---

## 7. Open questions for the orchestrator (ask, don't assume)

1. Where does Stage A interpretation run in the *reference* developer workflow — developer's local model, a hosted model, or either behind one interface? (Affects "offline" messaging for *developers* vs. *end-users*.)
2. What is the lowest-end *named* device PVC must ship on? Without a concrete device the "any hardware" mandate cannot be tested.
3. Is iOS a launch target? It constrains distribution claims (§1.7) and GPU API (Metal only, no WebGPU outside Safari).
4. Is "language evolution" meant to change *what prose is accepted* (interpreter), *what the IR can express* (language), or both? §4.5 assumes interpreter-only by default.
5. Who governs dialect packs — a foundation, the repo maintainers, or per-organization forks?
6. Is the hybrid syntax (`Canvas { prose }`) acceptable, given *Contributing.md* denies bracket structures? Recommend yes; needs an explicit decision.

---

## 8. Readiness gates to reach ≥90% confidence

| ID | Gate | Why it's blocking |
|---|---|---|
| G1 | README rewritten as a *vision + non-goals* document consistent with §1 corrections; LICENSE added; `CONTRIBUTING.md` renamed | Build session will otherwise inherit contradictions |
| G2 | Token/intent-fidelity benchmark corpus defined (≥50 UIs across layout-heavy and logic-heavy) with a named tokenizer | Replaces the unmeasured 90% claim |
| G3 | Reference-device list for Tiers 0/1/2 with frame budgets | Makes "any hardware" testable |
| G4 | PVC-IR v0 spec on paper: constraint vocabulary, SDF algebra, behavior model, semantic roles, capability manifest | The hard wall (§4.1) must be concrete before code |
| G5 | Privacy & evolution threat model (§4.5) reviewed | Mandated by the user; irreversible if wrong |
| G6 | Text strategy decision with a quality bar | Highest-risk subsystem (§2.4) |
| G7 | Re-check §9 findings at build start and pin exact crate versions [including the EU AI Act timeline in §9.8, which may shift] | Key dependencies are 0.x and moving (e.g. `vello_hybrid` → `vello_gpu` rename, Aug 2026) **[§9.4]** |

When G1–G7 are closed, the concept is build-ready. Until then, starting code will produce a GPU demo, not a runtime.

---

## 9. Verified findings (web research, 2026-10-08) — what changed in the analysis

Each finding: **fact → sources → consequence**. Body markers **[§9.n]** point here. Versions and percentages are as of the research date; G7 requires re-checking them at build start.

### 9.1 WebGPU browser status
- **Fact:** Chrome/Edge default-on since v113 (2023); Android 12+ since Chrome 121 on Qualcomm/ARM GPUs. Firefox default-on in 141 (Windows) and 145 (Apple-silicon macOS). Safari 26 on macOS Tahoe 26 / iOS 26 / iPadOS 26 / visionOS 26. Firefox on Linux/Android and on Intel Macs still behind flags (Mozilla targets 2026); Chromium on Linux still behind `enable-unsafe-webgpu`. Industry estimates put global coverage at ~70%, leaving ~30% needing a WebGL2 fallback.
- **Sources:** gpuweb Implementation-Status wiki (rev. 2 Oct 2026); web.dev, "WebGPU is now supported in major browsers"; webgpu.com news; byteiota 2026 survey.
- **Consequence:** README's "Instantly targets browser-based WebGPU canvases in Chrome, Safari, and Firefox" is true only for current desktop OS versions. Tier 1 (WebGL2) is mandatory (§1.8, §4.4).

### 9.2 wgpu downlevel limits
- **Fact:** `DownlevelFlags::COMPUTE_SHADERS` — "WebGL2, and GLES3.0 devices do not support compute." Storage buffers in fragment/vertex shaders, indirect draw, and some texture usages are also downlevel-gated. GL ES 3.0+/WebGL2 are "best-effort" in wgpu's support matrix; GL on macOS/iOS requires ANGLE. Real-world: flui #1048 and effectcraft #69 show engines claiming WebGL2 fallback failing validation because their pipelines assumed storage buffers.
- **Sources:** docs.rs, wgpu `DownlevelFlags`; gfx-rs/wgpu README; the issues above.
- **Consequence:** Tier 1 must be uniform-buffer/texture-fed and fragment-only from day one. "Add WebGL later" fails (§2.3).

### 9.3 Hardware share & platform policy
- **Fact:** Steam (Sept 2026): Windows 11 ≈75%, DX12-capable GPUs >91% — gaming-PC population only. Android: Vulkan 1.1 required for 64-bit devices launching with Android 10+; 1.3 for Android 13+ launches; 1.4 for Android 16+ launches. Google: 85% of active Android devices support Vulkan; GL ES is moving onto ANGLE-over-Vulkan (optional on Android 15+, manifest preference on 17+). Flutter Impeller still ships automatic GL ES fallback below API 29 or without usable Vulkan; Flutter's 2026 compute plan (flutter #188474) says Android deliberately falls back to GLES on a large population of ES 3.1-capable devices (Adreno 650 and older, Huawei Maleoon, early Samsung Xclipse, older PowerVR). Fallback bugs are real: flutter #179268 (gradients wrong after Vulkan→GLES fallback on PowerVR, Android 10).
- **Sources:** store.steampowered.com/hwsurvey; source.android.com Vulkan docs; developer.android.com Vulkan overview; docs.flutter.dev/perf/impeller; flutter issues above.
- **Consequence:** README's hard Vulkan/DX12/Metal prerequisite excludes ~15% of active Android plus all pre-Android-10 devices, contradicting the "weaker machines" mandate. Cross-tier golden-image parity testing is necessary — Flutter needed it (§1.8, §4.4, §6 item 3).

### 9.4 2D GPU renderer state of the art (Vello)
- **Fact:** Vello ships three renderers: `vello_cpu` (SIMD + multithreaded, "most mature", for software/no-GPU); `vello_gpu` (CPU path preprocessing + GPU raster; native WebGL2 backend and a wgpu backend without requiring compute shaders; the intended primary production renderer); and the original compute-centric `vello`, now under `research/` as experimental. `vello_hybrid` was renamed `vello_gpu` at 0.1.0 on 2026-08-26; APIs are 0.x. Compute-path failures on Adreno 7xx (8 pipelines fail to build) motivated the shift. MSRV 1.89.
- **Sources:** github.com/linebender/vello README; cuelight #293.
- **Consequence:** the leading GPU-2D team concluded compute-first is the wrong default. For PVC: fragment-only Tier 1 is the production path; compute Tier 2 is an optimisation. Build-vs-buy: PVC's SDF algebra can sit on a Vello-class raster backend (§2.3, §4.3, §4.4, G7).

### 9.5 On-device CPU-only inference (2026)
- **Fact:** sub-2B models run at 18–36 tok/s on an ordinary i5 laptop; 3–4B at 7–10 tok/s; 7–8B at 3–4 tok/s. A CEUR paper measures Llama-3.2-1B at ≈120 tok/s on a Sapphire Rapids Xeon and ≈25 tok/s on a 2013 Ivy Bridge Xeon E5-2695 v2. Generation is memory-bandwidth-bound; Q4_K_M is the practical quantization floor; BitNet ternary models run at roughly the same speed at ¼ the memory for <1 GB deployments.
- **Sources:** localaimaster.com (via itsfoss, May 2026); insiderllm.com; ceur-ws.org/Vol-4164/paper11.pdf; promptquorum.com.
- **Consequence:** a developer edit loop with a local 1–4B model is viable on 2018+ laptops (1–few-second compiles, *estimate*). A runtime LLM for end users on weak phones is not viable. The hard wall (§4.1) stands; any optional Tier 2 runtime assistant should be ≤2B / BitNet-class and selection-only (§1.6, §4.6).

### 9.6 Accessibility infrastructure (AccessKit)
- **Fact:** Chromium-derived schema; adapters for Windows (UIA), macOS (NSAccessibility), iOS (UIAccessibility), Android, Unix (AT-SPI via zbus), and a web adapter for applications that render their own UI to a canvas; winit adapter; C and Python bindings; default-on in egui/eframe. Push-based incremental tree updates; suits retained or immediate mode given stable node IDs. Actively developed (July 2026); lead developer presented at RustWeek 2026.
- **Sources:** github.com/AccessKit/accesskit; accesskit.dev; lib.rs/crates/accesskit; 2026.rustweek.org/talks/matt.
- **Consequence:** the a11y gap is closable without research; PVC-IR semantic roles (§4.2 item 4) map ~1:1 onto AccessKit roles. A requirement, not a risk (§2.5).

### 9.7 Text stack (Parley)
- **Fact:** Parley (rich-text layout) depends on HarfRust (production shaping, a HarfBuzz port), Fontique (font enumeration and fallback incl. CJK/Arabic/emoji), Skrifa (glyph loading), and ICU4X. NLnet-funded; MSRV 1.88; used by Vello's `glifo` and by Dioxus.
- **Sources:** github.com/linebender/parley; nlnet.nl/project/Parley; crates.io/parley.
- **Consequence:** shaping, fallback, and bidi are a solved dependency; PVC's text risk narrows to small-size raster quality and IME/editing (§2.4, §4.3, G6).

### 9.8 Regulation (EAA)
- **Fact:** Directive (EU) 2019/882 enforceable in all 27 member states since 28 June 2025; extraterritorial to non-EU sellers; EN 301 549 v4.1.1 (WCAG 2.2) published Sept 2025; first lawsuits in France Nov 2025; Carrefour ordered in June 2026 to make its site and app accessible within six months under daily penalties; Sweden, the Netherlands, and Germany enforcing; exemption only for micro-enterprises (<10 staff and <€2M). EU AI Act obligations are phasing in (general-purpose-model duties from Aug 2025; high-risk system duties scheduled from Aug 2026, with a Commission proposal to delay parts of that timeline under discussion) — relevance to PVC's shared-learning interpreter is indirect; §4.5's privacy posture should simply be documented against it.
- **Sources:** levelaccess.com EAA overview; dwt.com; raftlabs.com; freemius.com.
- **Consequence:** any PVC-built app sold in the EU inherits a legal accessibility obligation; a framework without an a11y tree is unusable for commercial EU products (§2.5, §6 item 5).

### 9.9 Prior art confirmed
- **Fact:** Flutter/Impeller (owns-the-surface, precompiled shaders, Vulkan→GL fallback), Vello (three-tier renderer), AccessKit, Parley/HarfRust, Cassowary (Apple Auto Layout).
- **Sources:** as cited in §9.3, §9.4, §9.6, §9.7.
- **Consequence:** each is precedent for one layer of §4; none compiles natural-language intent — that is PVC's novel contribution.
- **Not checked in this research pass, listed for the build session to evaluate:** Slint, Makepad, egui, Iced, Dear ImGui, Figma's rendering stack, and any 2025–26 intent-to-UI systems. Cassowary/Auto Layout is cited from general knowledge, not from a source fetched here.

---

## 10. One-paragraph summary for the build session

PVC's real idea is sound: *the screen is a mathematical surface and intent should compile to it directly*. Its README is not: it promises an LLM-free, offline, any-hardware runtime while implying an LLM in the runtime; it denies the syntax it uses; it claims to bypass things that cannot be bypassed; and it is silent on text, accessibility, privacy, and evolution — the four things that will actually decide adoption. The fix is architectural, not cosmetic: put a hard wall between non-deterministic interpretation (build-time) and a deterministic, semantics-preserving IR (runtime); render that IR through a constraint solver plus an SDF algebra across three hardware tiers with a software floor; evolve the *interpreter* through opt-in, k-anonymous, versioned dialect packs rather than data collection; and make user-set capabilities a property of the IR manifest that prose cannot reach. Close gates G1–G7, then build.
