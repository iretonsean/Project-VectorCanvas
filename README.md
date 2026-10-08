# Project VectorCanvas (PVC)

**A hardware-accelerated, AI-native programming language designed for generative AI workflows. Built for designers, Project VectorCanvas bypasses traditional DOM/UI frameworks by compiling conversational design intent directly into WebGPU/WGSL matrix math. High-performance, cross-platform layouts at 60+ FPS.**

PVC redefines how applications are built by shifting the execution layer away from legacy, text-heavy code frameworks (HTML, CSS, JavaScript, React) and mapping plain natural language design descriptions directly onto modern graphics pipelines. The entire language is built from the ground up for LLM coding agents, maximizing token efficiency and eliminating syntax hallucinations.

---

## ⚡ The Core Philosophy

Traditional programming languages are built for humans to read and write sequentially, making them incredibly token-inefficient for AI agents and heavy for hardware to process. PVC changes the paradigm:

*   **Optimized for LLM Agents:** Spits out simple conversational design summaries instead of verbose boilerplate. This reduces token consumption by up to 90%, slashes API transaction costs, and prevents code generation errors.
*   **For Designers:** Express software through structured layouts, mathematical constraints, aesthetic boundaries, and behavioral logic using natural, conversational language—without fighting rigid syntax or dependency hell.
*   **For Hardware:** Bypasses operating system window managers and browser layout engines to treat every screen on Earth as a raw mathematical canvas.

---

## 🛠 Features & Architecture

### 1. Zero-Dependency Cross-Platform Engine
PVC targets low-level graphics web and hardware standards (**WebGPU/WGSL**). It runs natively across the entire digital ecosystem by bypassing operating-system-specific UI constraints and utilizing native backend wrappers:
*   **Desktop:** Native windowing via Vulkan (Linux), DirectX 12 (Windows), and Metal (macOS).
*   **Web:** Instantly targets browser-based WebGPU canvases in Chrome, Safari, and Firefox.
*   **Mobile & Embedded:** Renders directly on physical screen layers without slow app-store wrapper kits.

### 2. 100% Offline & Local Execution
Applications built with PVC require **zero internet access** for end-users. 
*   **Local Compiler:** The runtime is a highly optimized, self-contained binary written in Rust/C that handles compilation instantly on the client machine using a local semantic intent classifier.
*   **Procedural Rendering:** No heavy JPEG, PNG, or 3D asset downloads. Complex user interfaces, fluid simulations, and visual effects are drawn on the fly by the local GPU using **Signed Distance Fields (SDFs)** and pure physics formulas.

---

## 📐 Input Syntax at a Glance

### The Legacy Way (HTML + Tailwind CSS ~600 tokens)
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

### The PVC Paradigm (~65 tokens)
```text
Canvas {
  Create a dark slate profile card, 320px wide with a soft border. 
  Inside, align a blue circular status avatar containing the initials "JD" next to the name "Jane Doe" (bold, 14px) and title "Designer" (muted, 12px).
  Add a text block with light gray body text explaining our spatial computing goals.
  Put a full-width teal button at the bottom labeled "Connect" that triggers the 'connect_user' state function on tap.
}
```

---

## 🚀 Getting Started

### 1. Installation (Prerequisites)
To run or develop with the PVC runtime, ensure you have the following installed locally on your machine:
* **Rust Toolchain:** (MSRV 1.75+) to compile the native client wrapper.
* **Modern Graphics Drivers:** A system supporting Vulkan (Linux), DirectX 12 (Windows), or Metal (macOS).

### 2. Running a Local Sandbox
Clone the repository and run the engine locally using an offline conversational input file:
```bash
git clone https://github.com/yourusername/project-vectorcanvas.git
cd project-vectorcanvas
cargo run --bin pvc-runtime -- path/to/your/app.pvc
```

### 3. Contributing
We welcome contributions from both designers and AI system engineers! 
* **Designers:** Help us expand conversational input parsing logic and build robust procedural layout modules.
* **Engineers:** Optimize the WGSL shader pipelines, improve client-side hit-testing matrix performance, or expand native window event integrations for AI execution.

## 📄 License
This project is open-source and available under the [MIT License](LICENSE).
