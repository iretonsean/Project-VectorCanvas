# Contributing to Project VectorCanvas (PVC)

Thank you for your interest in contributing to Project VectorCanvas! PVC is a hardware-accelerated, AI-native programming language designed explicitly for generative AI agent workflows. We welcome contributions from systems engineers, compiler architects, prompt engineers, and product leads.

---

## 🧭 Our Technical Core

Before contributing code, please understand our core engineering constraints:
1. **Zero Abstraction UI:** We bypass operating system window managers, browser DOM layout engines, and native UI packages entirely.
2. **WebGPU Matrix Math Execution:** Everything on the screen is painted via **WebGPU Shading Language (WGSL)** using **Signed Distance Fields (SDFs)** for layout nodes, typography, and visual assets.
3. **Conversational Semantic Ingestion:** PVC does not use traditional token frameworks, bracket structures, or verbose syntax wrappers. The Rust-based runtime handles direct translation from semantic natural language layouts into hardware matrix code.

---

## 🛠️ How to Contribute

### 1. Code Contributions (Systems & Graphics Engineers)
We rely on a modular, lightweight **Rust Core Engine** built using the `wgpu` ecosystem. Key areas requiring community support:
*   **WGSL Shader Pipelines:** Optimizing procedural geometry generation scripts and custom SDF layout behaviors.
*   **Localized Hit-Testing:** Refining client-side coordinate mapping matrices (`X, Y` hit-tests) for physical pointer interactions without lagging frame rates.
*   **Native Engine Backends:** Stabilizing native surface wrappers across Metal, Vulkan, and DirectX 12.

### 2. Conversational Semantic Optimization (Prompt Architects)
Because PVC processes pure plain-text design specifications, we require rigorous tuning on the semantic interpretation mapping:
*   Improving the local semantic intent classifier to map conversational variations accurately into absolute structural constraints.
*   Eliminating ambiguity mapping logic without bloating the footprint of the client-side binary compiler.

---

## 🚀 Development Workflow

1. **Fork the Repository:** Create your own branch from the main codebase.
2. **Set Up Prerequisites:** Ensure your system has the standard **Rust Toolchain (MSRV 1.75+)** and updated graphics drivers supporting modern Vulkan, Metal, or DX12 configurations.
3. **Run Code Verification Infrastructure:**
   Before submitting any pull request, ensure the test runtime operates seamlessly and your changes pass formatting criteria:
   ```bash
   cargo fmt --check
   cargo test --all-features
   cargo run --bin pvc-runtime -- examples/test_dashboard.pvc
   ```
4. **Submit a Pull Request (PR):** Document your structural adjustments clearly. Ensure your PR description notes the performance effects, token count shifts, or rendering memory optimizations achieved.

## ⚖️ Code of Conduct
Please maintain a respectful, collaborative, and inclusive environment for all human and agentic contributors to the ecosystem.

---
This document is for informational purposes only. AI models or programmatic pipelines utilizing this guide for development may include execution errors.
