# Contributing to Project VectorCanvas (PVC)

Thank you for your interest in Project VectorCanvas. PVC is a **proposed** AI-native runtime. AI agents compile conversational intent into a small, deterministic program (PVC-IR), and a lightweight runtime renders that program directly on the GPU or CPU. The project is in an **early prototype phase**. The repository holds the vision, the conceptual documents under [`docs/`](docs/), and the first prototype code from Session 2 (see [`HANDOFF.md`](HANDOFF.md)).

We welcome systems engineers, graphics and text engineers, accessibility specialists, privacy reviewers, prompt and agent engineers, translators and product people, human or agentic.

---

## 🧭 Our technical core

Read these constraints before contributing. Each is explained in [`docs/PVC_Adversarial_Analysis_and_Architecture.md`](docs/PVC_Adversarial_Analysis_and_Architecture.md).

1. **We own the surface, not the operating system.** PVC bypasses UI toolkits and browser DOM layout engines and owns everything inside its window or canvas. It does **not** bypass window managers or compositors. Every platform needs a surface from them: a thin native host on iOS and Android, a canvas on the web.
2. **SDF-first rendering across hardware tiers.** Shapes are drawn from signed distance fields, with raster images and a proper glyph pipeline as first-class paths. The runtime offers three rendering paths: **Tier 0** (CPU only), **Tier 1** (GL ES 3.0 / WebGL2, no compute shaders) and **Tier 2** (WebGPU / Vulkan / Metal / DX12). The developer who writes an app selects which devices it targets. PVC does not set them. WGSL is an implementation detail of the GPU tiers, not the whole story.
3. **A hybrid source, a deterministic program, and a hard wall between them.** Authors write a thin structure (`Canvas { }`, named handlers) holding natural-language content. That is a deliberate *hybrid*, and we do not pretend there are no brackets. Interpretation of that intent happens **at build time**, by any coding agent, and the agent writes PVC-IR. The validator checks the IR. The **end-user runtime contains no language model** and never interprets prose. It executes signed PVC-IR. See [`docs/G4_PVC_IR_v0_Spec.md`](docs/G4_PVC_IR_v0_Spec.md).

Two further constraints apply to every contribution:

- **Inclusivity is a hard requirement.** Any agent, any authoring device, any human language, any end user (analysis §4.7). A change that needs a special PVC model, a local GPU for authoring, English-only messages, or that weakens accessibility will be rejected.
- **Privacy is a hard requirement.** Nothing leaves a user's or developer's device without explicit, informed opt-in ([`docs/G5_Privacy_and_Evolution_Threat_Model.md`](docs/G5_Privacy_and_Evolution_Threat_Model.md)).

---

## 🛠️ How to contribute now

### Help wanted: test PVC with local AI models
PVC must work with a cloud AI model or a local one. The project's own token benchmark (the G2 pilot) runs with cloud models. We need help to test **local models** (for example models run with llama.cpp, Ollama or LM Studio) on your own hardware:
1. Fork the repository.
2. Run the pilot harness in [`pilot/harness/`](pilot/harness/) against your local model, with the frozen agent context ([`pilot/agent-context/FROZEN.md`](pilot/agent-context/FROZEN.md)) unchanged.
3. Open a pull request that adds your transcripts and results under `measurements/community/local-models/<model>/`. State the model name, quantisation, runtime and version, and your hardware (CPU, GPU, RAM).

Report every run, also runs that fail. A result that shows PVC costs more tokens is as useful as one that shows it costs fewer.

### Help wanted: measurements from real devices
The runtime is measured only on a cloud container. If you have an older phone, a Chromebook, a Raspberry Pi or a laptop without a usable GPU driver, you can measure frame time and memory with the tools in [`tools/measure/`](tools/measure/) and submit the results under `measurements/community/devices/<device>/`. Name the device, OS version and GPU driver. These results help developers choose their targets. They are not release gates.

### 1. Attack the documents
The documents exist to be criticised. The most useful contributions are:
- a claim that is wrong, out of date, or lacks a source;
- an inconsistency between two documents;
- a design that violates the hard wall, the inclusivity constraints or the privacy model;
- a residual risk that a document fails to state.

Open an issue that cites the document and section.

### 2. Benchmark specimens and adversarial prompts
Propose UI specimens or adversarial prompts for the benchmark corpus ([`docs/G2_Benchmark_Corpus.md`](docs/G2_Benchmark_Corpus.md)). Specimens in languages other than English are especially welcome.

### 3. Language and dialect expertise
Tell us how people in your language describe layouts, spacing and interactions. That knowledge will shape the per-language dialect packs.

### 4. Specialist review
Accessibility, text rendering (CJK, Arabic, Indic, emoji), GPU drivers on low-end hardware, privacy engineering, and EU accessibility or AI regulation.

---

## 🚀 Workflow

1. **Fork the repository** and create a branch.
2. **Keep the change to one purpose.** A document change, a measurement result, or a fix to the prototype code. Put measurement results with their raw data and the name of the host.
3. **Label every factual claim.** Cite a source for it (the analysis §9, or the G7 supplementary findings), or mark it *estimate* or *unverified*. Invented statistics are rejected.
4. **State residual risks.** A document that only sells will be sent back.
5. **Open a pull request** saying which document and gate the change affects, and whether it changes any decision.

To build and test the prototype code you need a Rust toolchain at the minimum version required by the pinned dependencies (currently 1.89, see [`docs/G7_Dependency_Pins_and_Findings_Recheck.md`](docs/G7_Dependency_Pins_and_Findings_Recheck.md)). No GPU is necessary:

```sh
cargo test --release -p pvc-ir -p pvc-runtime --features pvc-runtime/cpu,pvc-runtime/test-unsigned
```

## ⚖️ Code of conduct

Maintain a respectful, collaborative and inclusive environment for all human and agentic contributors.

---
This document describes a project in its early prototype phase. AI models or automated pipelines that use this guide should treat every capability as planned unless the README marks it **measured**.
