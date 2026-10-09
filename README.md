# Project VectorCanvas (PVC)

> **Status: vision, pre-implementation.** No PVC code existed before Session 2. This branch adds Session 2's incomplete prototype code (see the Session 2 status note below). This repository holds the project's vision and the conceptual documents that must hold up under criticism before any building starts. Everything below is **planned** unless it is explicitly marked **measured**.

> **Session 2 status (checkpoint, incomplete).** Session 2 was the first session allowed to write code. It stopped early. The first prototype code is on the `session2-evidence` branch (an IR crate, a runtime, native and web hosts), and [`HANDOFF.md`](HANDOFF.md) lists exactly what is done and what is not. This note states only what has been **measured**. Every result comes from one container (4-vCPU Xeon, no GPU). **No physical Tier 1 device (phone or Chromebook) was used.**
> - **Golden-image parity vs the CPU reference** (LDR-FLIP, bound committed before comparing): L01 **passes** on both the GL (llvmpipe) and WebGL2 (SwiftShader, CPU-emulated) paths. The three F01 states **fail** on both paths, on the 99.9th-percentile statistic (0.0526–0.0533 against a 0.05 bound; mean ≤ 0.0005). [`measurements/session2/golden/results.md`](measurements/session2/golden/results.md)
> - **Text quality vs Chromium** (reduced G6 bar, Latin + Simplified Chinese): **not passed**. Latin raster cells 8/8, Chinese 2/8, exact line breaks 4/8 per script. [`measurements/session2/text-bar/RESULT.md`](measurements/session2/text-bar/RESULT.md)
> - **Frame time, Tier 0 software path only:** an *indicative spot-check*, headless, not the planned measurement matrix. Typing frames p50 ≈ 12–13 ms, p95 ≈ 15–17 ms, and no frame is drawn when idle. [`measurements/session2/spotcheck/SUPERVISOR_frame_bench_tier0c.md`](measurements/session2/spotcheck/SUPERVISOR_frame_bench_tier0c.md). Tier 1 frame budgets, memory and web accessibility-mirror cost were **not measured**.
> - **Token benchmark (G2 pilot): not run.** The protocol is pre-registered and the PVC agent context is frozen at 2,511 tokens of fixed overhead per turn ([`pilot/agent-context/FROZEN.md`](pilot/agent-context/FROZEN.md)), but no agent has been run. **Whether G2's falsification trigger fires is therefore undetermined.** Two corrections to the example below were measured: the HTML block is 184 tokens, not 185 ([`measurements/session2/tokens/readme_recount.txt`](measurements/session2/tokens/readme_recount.txt)). A *complete, valid* PVC-IR card (with its manifest) is 150 tokens, not the 118 of the illustrative snippet ([`docs/session2/dev/dev-ir/evidence/l01-token-count.txt`](docs/session2/dev/dev-ir/evidence/l01-token-count.txt)).
> - The confidence figures in the analysis (§0, §8.1) have **not** been revised. No measurement yet supports a change.
> - Scope note: beyond the two prototype specimens (L01, F01), the only code built is headless validator coverage for the 15 pilot specimens, with no rendering (Supervisor decision D-2). The matching headless runtime probe was not built.

**What PVC is meant to be:** a way for AI agents to build digital products with fewer tokens, which then run on virtually any machine. You describe what you want in conversation with any coding agent, local or cloud. The agent compiles that intent into a small, checkable program called **PVC-IR**. A lightweight runtime draws that program directly on the GPU, or on the CPU when there is no usable GPU, with no browser engine or UI toolkit in between.

The owner's vision, in plain language:

> *"AI agents build any digital product with fewer tokens on virtually any machine that can use local or cloud AI agents."*

The first release targets **products with a user interface**, because that is where today's stacks spend the most tokens and where drawing directly to the screen pays off. The design leaves room for other product classes later. *UI first, not UI only.*

---

## How it is meant to work

```
 Your intent (conversation, any language)   ← you + any coding agent, on any device
        │  Stage A — interpretation: the agent writes PVC-IR, the validator checks it
        ▼
 PVC-IR (deterministic, typed, versioned, signed)   ← this is the program
        │  Stage B — resolution: layout solve, hardware-tier selection
        ▼
 Pixels on the end user's device                     ← offline, no AI model involved
```

The key rule is a **hard wall** between Stage A and Stage B. AI interpretation happens only while the app is being *built*. The app your users run contains **no language model**. It renders the same way every time, works offline, and runs on weak hardware. Details: [`docs/PVC_Adversarial_Analysis_and_Architecture.md`](docs/PVC_Adversarial_Analysis_and_Architecture.md) §4.1.

## Core ideas (planned)

- **Built for AI agents, checked by a validator.** The agent writes a compact form of PVC-IR. The **validator** expands it, checks it and returns structured, machine-fixable diagnostics, so even a small model can repair its own mistakes. The validator, not any particular model, decides what is correct. PVC catches *parse-level* and *structural* errors outright. It **does not** eliminate semantic mistakes such as a wrong colour or a misread "bottom". It surfaces them as diagnostics instead of guessing silently. ([G4 spec](docs/G4_PVC_IR_v0_Spec.md) §9)
- **No UI toolkit, no DOM layout.** PVC owns everything inside its window or canvas. On the web it draws into a canvas and uses hidden page elements only to expose accessibility information and to receive text input, because browsers offer no other way to do either for canvas content. It still needs a window (or a browser canvas, or a thin native host on mobile) from the operating system. Nothing can present pixels without one.
- **A single binary with no UI-framework dependency, not "zero dependencies".** The runtime is planned to build on established open-source libraries for GPU access (wgpu), rasterisation (Vello), text (Parley/HarfRust) and accessibility (AccessKit). Current versions are pinned in [G7](docs/G7_Dependency_Pins_and_Findings_Recheck.md).
- **Fewer tokens is the goal, and it will be measured, not asserted.** The figure that counts is **total tokens across every turn until the result is correct**, including fixes and the context the agent needs to know PVC. The benchmark is defined in [G2](docs/G2_Benchmark_Corpus.md). Until it runs, PVC makes no reduction claim beyond the single measured example below.
- **Offline for end users, always. Offline for authors, optionally.** End-user apps never need the internet unless the app itself declares a network capability. Authors may use a cloud agent or a local model, whichever they have.
- **Shapes drawn from math first, images where they are needed.** Interfaces are drawn mainly from signed distance fields (SDFs). Photos, logos and emoji are ordinary raster images, and text uses a proper text engine ([G6](docs/G6_Text_Strategy.md)).
- **Inclusive by design.** These are four testable constraints, not slogans:
  1. **Any agent.** It works with any general-purpose coding agent. No special PVC model is required.
  2. **Any authoring device.** You can author from a phone, a Chromebook or a library PC through a browser. No local GPU, Rust toolchain or local model is needed.
  3. **Any language.** You can describe your app in any human language your agent understands. Diagnostics are localisable codes, not English-only messages.
  4. **Any end user.** Accessibility (screen readers, keyboard, text scaling) and a software-rendering fallback for machines without a usable GPU are release-blocking, not optional.
- **Permissions you set, that no AI can talk its way past.** An app must declare what it may touch (network hosts, files, camera, clipboard...). The user grants or narrows those permissions, and the runtime enforces them mechanically.

## Where it is meant to run (planned)

| Hardware tier | What it uses | Examples of reference devices |
|---|---|---|
| Tier 0 | CPU-only software rendering | Raspberry Pi 4; 2012–2014 laptops with no usable GPU driver |
| Tier 1 | GL ES 3.0 / WebGL2-class GPUs, no compute shaders | Budget Android phones from 2018–2020 (Android 9–11); older laptops; Chromebooks; browsers without WebGPU |
| Tier 2 | WebGPU / Vulkan / Metal / DirectX 12 | Current phones, laptops and desktops |

- **Web:** WebGPU where the browser has it, otherwise WebGL2. WebGPU is not yet available everywhere: by industry estimates about 30% of users still need the WebGL2 path (analysis §9.1).
- **iOS:** distributed through the App Store like any other app, using a minimal native host that presents PVC's GPU surface.
- **Android:** a thin native host. Vulkan is **not** assumed.
- **Desktop:** Windows, macOS and Linux, including machines without Vulkan or DirectX 12.

Full device list and frame budgets: analysis §4.4.1.

## Example: one profile card, three ways

This is an illustration, not a benchmark. Token counts are **measured** with OpenAI's `tiktoken`, `o200k_base` encoding ([G7 S.4](docs/G7_Dependency_Pins_and_Findings_Recheck.md)).

**What you might say to your agent (English, 110 tokens):**
```text
Canvas {
  Create a dark slate profile card, 320px wide with a soft border.
  Inside, align a blue circular avatar with the initials "JD" next to the name "Jane Doe" (bold, 14px) and the title "Product Designer" (muted, 12px).
  Add a light gray paragraph with the text "Building the future of spatial computing via GPU pipelines."
  Put a full-width teal button at the bottom labeled "Connect" that triggers the 'connect_user' state function on tap.
}
```

**The same request in Spanish (144 tokens):**
```text
Canvas {
  Crea una tarjeta de perfil oscura en tono pizarra, de 320px de ancho y con un borde suave.
  Dentro, coloca un avatar circular azul con las iniciales "JD" junto al nombre "Jane Doe" (negrita, 14px) y el cargo "Diseñadora de producto" (atenuado, 12px).
  Añade un párrafo en gris claro con el texto "Construyendo el futuro de la computación espacial con canalizaciones de GPU."
  Abajo, un botón turquesa a todo lo ancho con la etiqueta "Conectar" que, al tocarlo, active la función de estado 'connect_user'.
}
```
Both requests produce the **same PVC-IR structure**. Only the strings table differs: Spanish labels in one, English in the other. The Spanish request costs more tokens (144 against 110) because this tokenizer splits Spanish into more pieces than English. That is a property of the tokenizer, not of the request. PVC's benchmark therefore compares stacks *within* each language and never treats the difference as a user's fault.

The request format is a *hybrid*: a thin structure (`Canvas { }`, named handlers such as `'connect_user'`) holds natural-language content. This is intentional.

**What the agent writes: compact PVC-IR (illustrative syntax, 118 tokens):**
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

**What an agent would write today: HTML + Tailwind CSS (185 tokens):**
```html
<div class="max-w-sm rounded-xl bg-gray-900 border border-gray-800 p-6 flex flex-col gap-4">
  <div class="flex items-center gap-3">
    <div class="w-10 h-10 rounded-full bg-blue-500 flex items-center justify-center text-white font-bold">JD</div>
    <div>
      <h3 class="text-white font-semibold text-sm">Jane Doe</h3>
      <p class="text-gray-400 text-xs">Product Designer</p>
    </div>
  </div>
  <p class="text-gray-300 text-sm leading-relaxed">Building the future of spatial computing via GPU pipelines.</p>
  <button class="w-full py-2.5 bg-teal-500 hover:bg-teal-400 text-gray-900 rounded-lg text-sm">Connect</button>
</div>
```

**What this does and does not show.** Earlier versions of this README claimed "~600 tokens" for the HTML and "~65" for the PVC request, and "up to 90%" savings. Measured, the HTML is 185 tokens. The *original README's* request text was 107 tokens. That request described a slightly different card ("Designer" rather than "Product Designer", and no body text given), so it was replaced by the corrected request above, which is 110 tokens. The fair comparison is between what the *agent* writes in each case: 118 against 185, about 36% fewer, for one hand-picked card. That is a single data point, not a result. It leaves out the context an agent needs in order to know PVC, which counts *against* PVC. The real answer will come from the benchmark in [G2](docs/G2_Benchmark_Corpus.md).

## Non-goals

- **No AI model in the apps you ship.** Interpretation happens only at build time.
- **No replacing the browser.** PVC targets applications, not web documents or hypertext.
- **No custom language model.** PVC works with existing agents and invests in the IR, the validator and examples.
- **No author-written shaders in the first version.** They would break the cross-device guarantees and the permission sandbox.
- **No non-UI products (backend services, CLIs, pipelines) in release one**, and no 3D, physics or game features until the GL ES 3.0 / WebGL2 tier renders real apps. These are planned as extensions that sit behind the same validator.
- **No visual editor, cloud service or marketplace** until the core works.
- **No collecting your prompts.** Any learning from usage is opt-in and local-first, and shares only abstract, anonymised patterns ([G5](docs/G5_Privacy_and_Evolution_Threat_Model.md)).

## Project documents

| Document | What it settles |
|---|---|
| [Adversarial analysis & architecture](docs/PVC_Adversarial_Analysis_and_Architecture.md) | The critique of the original pitch, the architecture, and the readiness gates |
| [G2 — Benchmark corpus & token metric](docs/G2_Benchmark_Corpus.md) | How "fewer tokens" will be measured honestly |
| [G4 — PVC-IR v0 conceptual spec](docs/G4_PVC_IR_v0_Spec.md) | What the program format must express and guarantee |
| [G5 — Privacy & evolution threat model](docs/G5_Privacy_and_Evolution_Threat_Model.md) | How the language can improve without surveillance |
| [G6 — Text strategy](docs/G6_Text_Strategy.md) | How text is shaped, drawn and edited, and the quality bar it must meet |
| [G7 — Dependency pins](docs/G7_Dependency_Pins_and_Findings_Recheck.md) | Current versions of key libraries, and facts re-checked |

## Getting started

There is nothing to install or run yet. When there is, there will be two separate paths:

- **Author with PVC (planned).** For people building apps. You will need any coding agent, local or cloud, and a web browser. The validator and preview are planned to run in a browser tab or as a small standalone tool. No GPU, Rust toolchain or local model will be required.
- **Build PVC itself (planned).** For people developing the runtime. You will need a Rust toolchain at or above the minimum version set by PVC's pinned dependencies (currently 1.89, per [G7](docs/G7_Dependency_Pins_and_Findings_Recheck.md)). A modern GPU is **not** required, because the CPU-only tier is a first-class target.

## Contributing

The most valuable contribution right now is criticism. Read the documents above and attack them: find a claim that is wrong, unsourced or inconsistent. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

MIT. See [`LICENSE`](LICENSE).
